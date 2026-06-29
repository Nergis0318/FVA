//! Call graph construction and traversal.

mod builder;

pub use builder::extract_edges;

use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};

use bincode::{deserialize, serialize};
use parking_lot::RwLock;
use petgraph::Direction;
use petgraph::graph::{DiGraph, NodeIndex};
use serde::{Deserialize, Serialize};

use crate::error::{FvaError, Result};
use crate::indexer::chunker::CodeChunk;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct SymbolId {
    pub name: String,
    pub file: String,
    pub line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub caller: SymbolId,
    pub callee: String,
    pub file: String,
    pub line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallGraphSnapshot {
    pub nodes: Vec<SymbolId>,
    pub edges: Vec<(usize, usize)>,
    pub callee_names: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GraphStats {
    pub nodes: usize,
    pub edges: usize,
}

/// Delta operation for incremental graph persistence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GraphDelta {
    /// Add a new symbol node to the graph
    AddNode(SymbolId),
    /// Remove a symbol node from the graph
    RemoveNode(SymbolId),
    /// Add a call edge between symbols
    AddEdge {
        caller: SymbolId,
        callee: String,
        line: usize,
    },
    /// Remove all nodes belonging to a file
    RemoveFile(String),
}

fn remove_node_inner(
    graph: &mut DiGraph<SymbolId, String>,
    node_index: &mut HashMap<SymbolId, NodeIndex>,
    callee_index: &mut HashMap<String, Vec<NodeIndex>>,
    symbol: &SymbolId,
) {
    if let Some(idx) = node_index.remove(symbol) {
        let _ = graph.remove_node(idx);
    }
    if let Some(indices) = callee_index.get_mut(&symbol.name.to_lowercase()) {
        indices.retain(|&i| node_index.values().any(|&v| v == i));
    }
}

/// Thread-safe call graph store.
pub struct CallGraphStore {
    path: PathBuf,
    delta_path: PathBuf,
    graph: RwLock<DiGraph<SymbolId, String>>,
    node_index: RwLock<HashMap<SymbolId, NodeIndex>>,
    callee_index: RwLock<HashMap<String, Vec<NodeIndex>>>,
}

impl CallGraphStore {
    pub fn open(data_dir: &Path) -> Result<Self> {
        let path = data_dir.join("call_graph.bin");
        let delta_path = data_dir.join("call_graph.delta.bin");
        let store = Self {
            path,
            delta_path,
            graph: RwLock::new(DiGraph::new()),
            node_index: RwLock::new(HashMap::new()),
            callee_index: RwLock::new(HashMap::new()),
        };

        // Load full snapshot if exists
        if store.path.exists()
            && let Ok(bytes) = std::fs::read(&store.path)
            && let Ok(snapshot) = deserialize::<CallGraphSnapshot>(&bytes)
        {
            store.load_snapshot(snapshot);
            tracing::info!(
                "loaded call graph snapshot: {} nodes, {} edges",
                store.graph.read().node_count(),
                store.graph.read().edge_count()
            );
        }

        // Replay delta log if exists
        if store.delta_path.exists() {
            store.replay_deltas()?;
            tracing::info!(
                "replayed deltas: {} nodes, {} edges",
                store.graph.read().node_count(),
                store.graph.read().edge_count()
            );
        }

        Ok(store)
    }

    fn load_snapshot(&self, snapshot: CallGraphSnapshot) {
        let mut graph = DiGraph::new();
        let mut node_index = HashMap::new();
        let mut callee_index: HashMap<String, Vec<NodeIndex>> = HashMap::new();

        for node in &snapshot.nodes {
            let idx = graph.add_node(node.clone());
            node_index.insert(node.clone(), idx);
            callee_index
                .entry(node.name.to_lowercase())
                .or_default()
                .push(idx);
        }

        for (from, to) in snapshot.edges {
            if from >= snapshot.nodes.len() || to >= snapshot.callee_names.len() {
                continue;
            }
            let label = snapshot.callee_names[to].clone();
            let from_idx = node_index[&snapshot.nodes[from]];

            let callee_symbol = SymbolId {
                name: label.clone(),
                file: String::new(),
                line: 0,
            };
            let to_idx = if let Some(&idx) = node_index.get(&callee_symbol) {
                idx
            } else {
                let idx = graph.add_node(callee_symbol.clone());
                callee_index
                    .entry(label.to_lowercase())
                    .or_default()
                    .push(idx);
                node_index.insert(callee_symbol, idx);
                idx
            };

            graph.add_edge(from_idx, to_idx, label);
        }

        *self.graph.write() = graph;
        *self.node_index.write() = node_index;
        *self.callee_index.write() = callee_index;
    }

    pub fn index_chunks(&self, chunks: &[CodeChunk]) -> Result<usize> {
        let edges = extract_edges(chunks);
        let mut added = 0usize;

        for edge in edges {
            self.add_edge(&edge.caller, &edge.callee, edge.line)?;
            // Record delta for incremental persistence
            self.record_delta(GraphDelta::AddEdge {
                caller: edge.caller.clone(),
                callee: edge.callee.clone(),
                line: edge.line,
            })?;
            added += 1;
        }

        Ok(added)
    }

    pub fn add_edge(&self, caller: &SymbolId, callee: &str, line: usize) -> Result<()> {
        let mut graph = self.graph.write();
        let mut node_index = self.node_index.write();
        let mut callee_index = self.callee_index.write();

        let caller_idx = *node_index.entry(caller.clone()).or_insert_with(|| {
            let idx = graph.add_node(caller.clone());
            callee_index
                .entry(caller.name.to_lowercase())
                .or_default()
                .push(idx);
            idx
        });

        let callee_lower = callee.to_lowercase();
        let callee_idx = if let Some(&idx) = callee_index.get(&callee_lower).and_then(|v| v.first())
        {
            idx
        } else {
            let symbol = SymbolId {
                name: callee.to_string(),
                file: String::new(),
                line: 0,
            };
            let idx = graph.add_node(symbol.clone());
            callee_index.entry(callee_lower).or_default().push(idx);
            node_index.insert(symbol, idx);
            idx
        };

        graph.add_edge(caller_idx, callee_idx, format!("{line}"));
        Ok(())
    }

    pub fn remove_file(&self, relative_path: &str) -> Result<()> {
        let mut graph = self.graph.write();
        let mut node_index = self.node_index.write();
        let mut callee_index = self.callee_index.write();

        let to_remove: Vec<SymbolId> = node_index
            .keys()
            .filter(|s| s.file == relative_path)
            .cloned()
            .collect();

        // Record delta before removing
        self.record_delta(GraphDelta::RemoveFile(relative_path.to_string()))?;

        for symbol in &to_remove {
            // Record individual node removals for replay accuracy
            self.record_delta(GraphDelta::RemoveNode(symbol.clone()))?;
        }

        for symbol in &to_remove {
            remove_node_inner(&mut graph, &mut node_index, &mut callee_index, symbol);
        }

        // Rebuild node index from remaining graph nodes
        node_index.clear();
        callee_index.clear();
        for idx in graph.node_indices() {
            if let Some(node) = graph.node_weight(idx).cloned() {
                node_index.insert(node.clone(), idx);
                callee_index
                    .entry(node.name.to_lowercase())
                    .or_default()
                    .push(idx);
            }
        }
        Ok(())
    }

    pub fn find_symbol_nodes(&self, name: &str) -> Vec<SymbolId> {
        let key = name.to_lowercase();
        let graph = self.graph.read();
        let callee_index = self.callee_index.read();

        callee_index
            .get(&key)
            .map(|indices| {
                indices
                    .iter()
                    .filter_map(|&idx| graph.node_weight(idx).cloned())
                    .filter(|s| !s.file.is_empty())
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn callers(&self, symbol_name: &str, depth: usize) -> Vec<SymbolId> {
        self.traverse(symbol_name, Direction::Incoming, depth)
    }

    pub fn callees(&self, symbol_name: &str, depth: usize) -> Vec<SymbolId> {
        self.traverse(symbol_name, Direction::Outgoing, depth)
    }

    fn traverse(&self, symbol_name: &str, direction: Direction, depth: usize) -> Vec<SymbolId> {
        let graph = self.graph.read();
        let callee_index = self.callee_index.read();
        let key = symbol_name.to_lowercase();

        let Some(start_indices) = callee_index.get(&key) else {
            return vec![];
        };

        let mut result = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut queue: Vec<(NodeIndex, usize)> = start_indices.iter().map(|&i| (i, 0)).collect();

        while let Some((node, d)) = queue.pop() {
            if d > depth || !visited.insert(node) {
                continue;
            }

            if let Some(weight) = graph.node_weight(node)
                && !weight.file.is_empty()
                && d > 0
            {
                result.push(weight.clone());
            }

            if d < depth {
                let neighbors = match direction {
                    Direction::Incoming => graph.neighbors_directed(node, Direction::Incoming),
                    Direction::Outgoing => graph.neighbors_directed(node, Direction::Outgoing),
                };
                for neighbor in neighbors {
                    queue.push((neighbor, d + 1));
                }
            }
        }

        result
    }

    pub fn stats(&self) -> GraphStats {
        let graph = self.graph.read();
        GraphStats {
            nodes: graph.node_count(),
            edges: graph.edge_count(),
        }
    }

    pub fn persist(&self) -> Result<()> {
        // Save full snapshot
        let graph = self.graph.read();
        let mut nodes = Vec::new();
        let mut node_map = HashMap::new();

        for idx in graph.node_indices() {
            if let Some(n) = graph.node_weight(idx) {
                let i = nodes.len();
                node_map.insert(idx, i);
                nodes.push(n.clone());
            }
        }

        let mut edges = Vec::new();
        let mut callee_names = Vec::new();
        let mut callee_map: HashMap<String, usize> = HashMap::new();

        for edge_idx in graph.edge_indices() {
            if let Some((from, to)) = graph.edge_endpoints(edge_idx)
                && let (Some(&fi), Some(label)) = (node_map.get(&from), graph.edge_weight(edge_idx))
            {
                let ci = *callee_map.entry(label.clone()).or_insert_with(|| {
                    let i = callee_names.len();
                    callee_names.push(label.clone());
                    i
                });
                let _ = graph.node_weight(to);
                edges.push((fi, ci));
            }
        }

        let snapshot = CallGraphSnapshot {
            nodes,
            edges,
            callee_names,
        };

        let bytes =
            serialize(&snapshot).map_err(|e| FvaError::Other(format!("graph serialize: {e}")))?;
        std::fs::write(&self.path, bytes)?;

        // Clear delta log after full snapshot
        if self.delta_path.exists() {
            std::fs::remove_file(&self.delta_path)?;
        }

        tracing::info!(
            "persisted call graph: {} nodes, {} edges",
            snapshot.nodes.len(),
            snapshot.edges.len()
        );
        Ok(())
    }

    /// Record a delta operation to the delta log
    fn record_delta(&self, delta: GraphDelta) -> Result<()> {
        let bytes =
            serialize(&delta).map_err(|e| FvaError::Other(format!("delta serialize: {e}")))?;

        // Create delta directory if needed
        if let Some(parent) = self.delta_path.parent()
            && !parent.exists()
        {
            std::fs::create_dir_all(parent)?;
        }

        // Append delta to log file
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.delta_path)?;

        // Write delta size (u32) followed by delta bytes
        let size = bytes.len() as u32;
        file.write_all(&size.to_le_bytes())?;
        file.write_all(&bytes)?;

        Ok(())
    }

    /// Replay all deltas from the delta log
    fn replay_deltas(&self) -> Result<()> {
        if !self.delta_path.exists() {
            return Ok(());
        }

        let bytes = std::fs::read(&self.delta_path)?;
        let mut offset = 0;

        while offset < bytes.len() {
            // Read size (u32)
            if offset + 4 > bytes.len() {
                break; // Incomplete size
            }
            let size = u32::from_le_bytes([
                bytes[offset],
                bytes[offset + 1],
                bytes[offset + 2],
                bytes[offset + 3],
            ]) as usize;
            offset += 4;

            // Read delta
            if offset + size > bytes.len() {
                break; // Incomplete delta
            }
            let delta_bytes = &bytes[offset..offset + size];

            if let Ok(delta) = deserialize::<GraphDelta>(delta_bytes) {
                self.apply_delta(delta)?;
            }

            offset += size;
        }

        Ok(())
    }

    /// Apply a single delta to the graph
    fn apply_delta(&self, delta: GraphDelta) -> Result<()> {
        match delta {
            GraphDelta::AddNode(symbol) => {
                let mut graph = self.graph.write();
                let mut node_index = self.node_index.write();
                let mut callee_index = self.callee_index.write();

                if node_index.contains_key(&symbol) {
                    return Ok(()); // Already exists
                }

                let idx = graph.add_node(symbol.clone());
                node_index.insert(symbol.clone(), idx);
                callee_index
                    .entry(symbol.name.to_lowercase())
                    .or_default()
                    .push(idx);
            }
            GraphDelta::RemoveNode(symbol) => {
                let mut graph = self.graph.write();
                let mut node_index = self.node_index.write();
                let mut callee_index = self.callee_index.write();

                remove_node_inner(&mut graph, &mut node_index, &mut callee_index, &symbol);
            }
            GraphDelta::AddEdge {
                caller,
                callee,
                line,
            } => {
                // Use add_edge to ensure proper indexing
                let _ = self.add_edge(&caller, &callee, line);
            }
            GraphDelta::RemoveFile(path) => {
                let mut graph = self.graph.write();
                let mut node_index = self.node_index.write();
                let mut callee_index = self.callee_index.write();

                let to_remove: Vec<SymbolId> = node_index
                    .keys()
                    .filter(|s| s.file == path)
                    .cloned()
                    .collect();

                for symbol in &to_remove {
                    remove_node_inner(&mut graph, &mut node_index, &mut callee_index, symbol);
                }
            }
        }
        Ok(())
    }
}
