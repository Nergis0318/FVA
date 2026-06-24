---
name: fva
description: >
  Use FVA (FFF · Vector · AST) for hybrid codebase intelligence via MCP or CLI.
  Trigger when exploring, searching, or understanding a codebase with FVA;
  when the user asks for hybrid_search, semantic search, call graphs, AST chunks,
  smart context, or wants to replace grep+read loops. Use when FVA MCP tools are
  available or when running `fva` CLI commands.
---

# FVA — Codebase Intelligence

FVA combines **FFF** (fuzzy file search + grep), **vector embeddings**, and **AST chunking** (Tree-sitter) with call graphs into one hybrid search engine for AI agents.

## Check Availability

```bash
fva --version
```

If missing, install from [GitHub Releases](https://github.com/Nergis0318/FVA/releases) or build from source:

```bash
cargo install --path . --force
```

Upgrade the binary (not the project index):

```bash
fva upgrade
```

## Index Before Heavy Search

From the target project root:

```bash
fva index --path .
fva status --path .
```

FVA stores indexes in `.fva/` (frecency, history, vectors, call graph). Run `index` once before heavy workloads; the MCP server watches files when `watch = true` in config.

## MCP Tool Workflow

**Default order** — prefer fused search over repeated grep + read:

1. `hybrid_search` — **best default** (FFF + vector + call graph)
2. `get_smart_context` — token-budget context before edits
3. `semantic_search` — conceptual queries ("auth middleware", "retry logic")
4. `get_symbol_info` / `get_chunks` — full function/class bodies (AST-aware)
5. `get_call_graph` — callers and callees
6. `grep` — bare identifiers only (`MyHandler`, not `fn MyHandler`)
7. `find_files` — fuzzy path discovery
8. `index_status` — check indexing progress

### Tool Selection Guide

| Task                         | Tool                     |
| ---------------------------- | ------------------------ |
| "Where is X handled?"        | `hybrid_search`          |
| "Understand before changing" | `get_smart_context`      |
| Concept / pattern search     | `semantic_search`        |
| Exact symbol body            | `get_symbol_info`        |
| File structure / chunks      | `get_chunks` with `path` |
| Who calls this function?     | `get_call_graph`         |
| Exact identifier in text     | `grep`                   |
| Find file by partial path    | `find_files`             |

### Pagination

Tools support `maxResults` and `offset`. When output includes `offset: N`, pass `offset: N` on the next call.

## CLI Fallback

When MCP is unavailable:

```bash
fva search "authentication handler" --path . --limit 10
fva status --path .
fva index --path .
```

## Rules

- Prefer `hybrid_search` or `get_smart_context` over grep → read → grep loops.
- Use AST chunks (`get_chunks`, `get_symbol_info`) instead of raw full-file reads when possible.
- Grep bare identifiers only — FFF expands definitions automatically.
- Scope searches with `path` on `hybrid_search` / `get_smart_context` when the target file is known.
- Check `index_status` if searches return empty or stale results.

## Configuration

Copy `config.example.toml` → `fva.toml` or `.fva.toml` (project root) and/or `~/.config/fva/config.toml` (global). Project overrides global.

Key settings:

```toml
[embedding]
provider = "local"    # or "voyage" with VOYAGE_API_KEY

[query]
fff_weight = 0.3
vector_weight = 0.5
graph_weight = 0.2
max_context_tokens = 8000
```

CLI flags override config: `--path`, `--config`, `RUST_LOG`.

## Further Reference

See [references/mcp-tools.md](references/mcp-tools.md) for parameter details and example prompts.
