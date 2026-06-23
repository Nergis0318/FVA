# FVA — Fast Vector Agent

High-speed hybrid codebase intelligence engine for AI coding agents. Combines **FFF** (fuzzy file search), **Tree-sitter** (AST chunking), **vector embeddings**, and **call graphs** into a single MCP server.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    MCP Server (stdio)                        │
│  hybrid_search │ semantic_search │ grep │ find_files │ ... │
└─────────────┬───────────────────────┬───────────────────────┘
              │                       │
     ┌────────▼────────┐     ┌────────▼────────┐
     │   FFF Engine    │     │  AST Indexer    │
     │ frecency+fuzzy  │     │  Tree-sitter    │
     │  git-aware grep │     │  chunk store    │
     └────────┬────────┘     └────────┬────────┘
              │                ┌───────▼────────┐
              │                │  Vector Store  │
              │                │  (flat/LanceDB)│
              │                └───────┬────────┘
              │                ┌───────▼────────┐
              │                │  Call Graph    │
              │                │  (petgraph)    │
              └────────┬───────┴────────┬───────┘
                       │                │
                ┌──────▼────────────────▼──────┐
                │     Hybrid Query Engine      │
                │  FFF → Vector → Graph fusion  │
                └──────────────────────────────┘
```

## Features (Phases 1–4)

| Phase | Feature | Status |
|-------|---------|--------|
| 1 | FFF MCP + Tree-sitter chunking | Done |
| 2 | Embedding pipeline + vector store | Done |
| 3 | Call graph + hybrid query engine | Done |
| 4 | Full MCP tool set + CLI | Done |
| 5 | Large-scale benchmarks + docs | Planned |

## Quick Start

```bash
cargo build --release

# MCP server (default)
fva --path /path/to/project

# Full index (AST + vectors + call graph)
fva index --path .

# Status
fva status --path .

# CLI hybrid search
fva search "authentication handler" --path . --limit 5
```

## MCP Client Setup

```json
{
  "mcpServers": {
    "fva": {
      "command": "D:\\Dev\\FVA\\target\\release\\fva.exe",
      "args": ["--path", "D:\\Dev\\YourProject"],
      "env": { "RUST_LOG": "info" }
    }
  }
}
```

### Recommended Agent Prompt

```
For codebase exploration, use FVA MCP tools:
- hybrid_search: default — combines file search + semantic + call graph
- semantic_search: natural language concept search
- get_smart_context: token-efficient context for a task
- get_symbol_info / get_chunks: full function/class bodies
- get_call_graph: callers and callees
Prefer hybrid_search over repeated grep+read cycles.
```

## MCP Tools

| Tool | Description |
|------|-------------|
| `hybrid_search` | **Default.** FFF + vector + graph fusion |
| `semantic_search` | Natural language embedding search |
| `find_files` | Fuzzy path search, frecency-ranked |
| `grep` | Content search with definition expansion |
| `get_chunks` | AST chunks by file or query |
| `get_symbol_info` | Symbol lookup with full source |
| `get_call_graph` | Callers/callees of a function |
| `get_smart_context` | Token-budget smart context builder |
| `index_status` | Full indexing statistics |

## Configuration

Copy `config.example.toml` to `config.toml`:

```toml
[embedding]
provider = "local"    # "local" (default) or "voyage"
model = "voyage-code-3"

[vector]
backend = "flat"        # file-backed cosine search
db_path = ".fva/vectors"

[query]
fff_weight = 0.3
vector_weight = 0.5
graph_weight = 0.2
max_context_tokens = 8000
```

### Voyage API (optional)

```toml
[embedding]
provider = "voyage"
```

Set `VOYAGE_API_KEY` environment variable or `voyage_api_key` in config.

## Module Structure

```
src/
├── main.rs            # CLI (serve, index, status, search)
├── engine.rs          # Central orchestrator
├── embedding/         # local-hash + voyage providers
├── vector/            # flat vector store (LanceDB optional)
├── graph/             # call graph (petgraph)
├── indexer/           # AST parsing + chunking + pipeline
├── query/             # hybrid search + smart context
├── fff/               # FFF integration
└── mcp/               # MCP tool handlers
```

## Performance Targets

| Operation | Target | Notes |
|-----------|--------|-------|
| File search (100k files) | < 50ms | FFF frecency + SIMD |
| Grep (warm index) | < 100ms | mmap + content index |
| AST chunk (single file) | < 5ms | Tree-sitter |
| Vector search (10k chunks) | < 50ms | flat brute-force |
| Hybrid search | < 200ms | 3-stage fusion |
| Full index (10k files) | < 30s | rayon parallel |

## Security

- Sandboxed indexing (project root only)
- No telemetry — all data stored locally in `.fva/`
- Embeddings: local by default, Voyage only when explicitly configured

## License

MIT