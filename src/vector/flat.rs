//! Flat in-memory vector store with disk persistence.
//!
//! Uses brute-force cosine similarity — fast for <100k chunks,
//! zero external dependencies, LanceDB-compatible interface.

use std::collections::HashMap;
use std::path::PathBuf;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use super::VectorStore;
use crate::embedding::cosine_similarity;
use crate::error::{FvaError, Result};
use crate::indexer::chunker::CodeChunk;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredVector {
    chunk_id: String,
    relative_path: String,
    symbol_name: String,
    symbol_kind: String,
    language: String,
    start_line: usize,
    end_line: usize,
    content_preview: String,
    vector: Vec<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct VectorSnapshot {
    dimensions: usize,
    entries: Vec<StoredVector>,
}

/// A vector search hit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorHit {
    pub chunk_id: String,
    pub relative_path: String,
    pub symbol_name: String,
    pub symbol_kind: String,
    pub language: String,
    pub start_line: usize,
    pub end_line: usize,
    pub content_preview: String,
    pub score: f32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VectorStats {
    pub total_vectors: usize,
    pub dimensions: usize,
}

pub struct FlatVectorStore {
    path: PathBuf,
    dimensions: usize,
    entries: RwLock<Vec<StoredVector>>,
    by_file: RwLock<HashMap<String, Vec<usize>>>,
}

impl FlatVectorStore {
    pub fn open(path: PathBuf, dimensions: usize) -> Result<Self> {
        std::fs::create_dir_all(path.parent().unwrap_or(&path))?;

        let store = Self {
            path: path.clone(),
            dimensions,
            entries: RwLock::new(Vec::new()),
            by_file: RwLock::new(HashMap::new()),
        };

        let data_file = path.join("vectors.bin");
        if data_file.exists() {
            if let Ok(bytes) = std::fs::read(&data_file) {
                if let Ok(snapshot) = bincode::deserialize::<VectorSnapshot>(&bytes) {
                    if snapshot.dimensions == dimensions {
                        store.load_snapshot(snapshot);
                        tracing::info!(
                            "loaded {} vectors from {}",
                            store.entries.read().len(),
                            data_file.display()
                        );
                    } else {
                        tracing::warn!(
                            "vector dimensions changed ({} -> {}), re-indexing required",
                            snapshot.dimensions,
                            dimensions
                        );
                    }
                }
            }
        }

        Ok(store)
    }

    fn load_snapshot(&self, snapshot: VectorSnapshot) {
        let mut by_file: HashMap<String, Vec<usize>> = HashMap::new();
        for (idx, entry) in snapshot.entries.iter().enumerate() {
            by_file
                .entry(entry.relative_path.clone())
                .or_default()
                .push(idx);
        }
        *self.entries.write() = snapshot.entries;
        *self.by_file.write() = by_file;
    }

    fn preview(content: &str, max_len: usize) -> String {
        if content.len() <= max_len {
            content.to_string()
        } else {
            format!("{}...", &content[..max_len])
        }
    }
}

impl VectorStore for FlatVectorStore {
    fn upsert_chunks(&self, chunks: &[CodeChunk], vectors: &[Vec<f32>]) -> Result<()> {
        if chunks.len() != vectors.len() {
            return Err(FvaError::Other(format!(
                "chunk/vector count mismatch: {} vs {}",
                chunks.len(),
                vectors.len()
            )));
        }

        if let Some(path) = chunks.first().map(|c| c.relative_path.clone()) {
            self.remove_file(&path)?;
        }

        let mut entries = self.entries.write();
        let mut by_file = self.by_file.write();

        for (chunk, vector) in chunks.iter().zip(vectors.iter()) {
            if vector.len() != self.dimensions {
                return Err(FvaError::Other(format!(
                    "vector dimension mismatch: expected {}, got {}",
                    self.dimensions,
                    vector.len()
                )));
            }

            let idx = entries.len();
            entries.push(StoredVector {
                chunk_id: chunk.id.clone(),
                relative_path: chunk.relative_path.clone(),
                symbol_name: chunk.symbol_name.clone(),
                symbol_kind: chunk.symbol_kind.clone(),
                language: chunk.language.clone(),
                start_line: chunk.start_line,
                end_line: chunk.end_line,
                content_preview: Self::preview(&chunk.content, 200),
                vector: vector.clone(),
            });
            by_file
                .entry(chunk.relative_path.clone())
                .or_default()
                .push(idx);
        }

        Ok(())
    }

    fn remove_file(&self, relative_path: &str) -> Result<()> {
        let mut entries = self.entries.write();
        entries.retain(|e| e.relative_path != relative_path);

        let mut by_file = self.by_file.write();
        by_file.clear();
        for (idx, entry) in entries.iter().enumerate() {
            by_file
                .entry(entry.relative_path.clone())
                .or_default()
                .push(idx);
        }

        Ok(())
    }

    fn search(&self, query_vector: &[f32], limit: usize) -> Result<Vec<VectorHit>> {
        let entries = self.entries.read();
        let mut hits: Vec<VectorHit> = entries
            .iter()
            .map(|e| VectorHit {
                chunk_id: e.chunk_id.clone(),
                relative_path: e.relative_path.clone(),
                symbol_name: e.symbol_name.clone(),
                symbol_kind: e.symbol_kind.clone(),
                language: e.language.clone(),
                start_line: e.start_line,
                end_line: e.end_line,
                content_preview: e.content_preview.clone(),
                score: cosine_similarity(query_vector, &e.vector),
            })
            .collect();

        hits.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        hits.truncate(limit);
        Ok(hits)
    }

    fn stats(&self) -> VectorStats {
        VectorStats {
            total_vectors: self.entries.read().len(),
            dimensions: self.dimensions,
        }
    }

    fn persist(&self) -> Result<()> {
        let snapshot = VectorSnapshot {
            dimensions: self.dimensions,
            entries: self.entries.read().clone(),
        };
        let bytes = bincode::serialize(&snapshot)
            .map_err(|e| FvaError::Other(format!("vector serialize: {e}")))?;
        let data_file = self.path.join("vectors.bin");
        std::fs::write(&data_file, bytes)?;
        Ok(())
    }
}