# FVA MCP Tools Reference

## hybrid_search

**Default tool.** Fuses FFF file search, vector semantic search, and call graph.

| Parameter    | Type   | Required | Notes                                          |
| ------------ | ------ | -------- | ---------------------------------------------- |
| `query`      | string | yes      | Natural language or identifier                 |
| `maxResults` | number | no       | Default 20                                     |
| `path`       | string | no       | Filter hits to paths containing this substring |

## semantic_search

Embedding-based concept search over AST chunks.

| Parameter    | Type   | Required |
| ------------ | ------ | -------- |
| `query`      | string | yes      |
| `maxResults` | number | no       |

## get_smart_context

Token-budget context builder combining hybrid search + call graph + file context.

| Parameter    | Type   | Required       |
| ------------ | ------ | -------------- |
| `query`      | string | yes            |
| `path`       | string | no — file hint |
| `maxResults` | number | no             |

## get_symbol_info

Look up symbol by name; returns full AST chunks with source.

| Parameter    | Type   | Required |
| ------------ | ------ | -------- |
| `symbol`     | string | yes      |
| `maxResults` | number | no       |

## get_chunks

AST chunks (functions, classes, methods). Provide `path` OR `query`.

| Parameter        | Type    | Required          |
| ---------------- | ------- | ----------------- |
| `path`           | string  | one of path/query |
| `query`          | string  | one of path/query |
| `maxResults`     | number  | no                |
| `offset`         | number  | no                |
| `includeContent` | boolean | no — default true |

## get_call_graph

Callers and callees of a function/symbol.

| Parameter  | Type   | Required       |
| ---------- | ------ | -------------- |
| `function` | string | yes            |
| `depth`    | number | no — default 1 |

## grep

Content search with definition expansion. **Bare identifiers only.**

| Parameter    | Type   | Required               |
| ------------ | ------ | ---------------------- |
| `query`      | string | yes (alias: `pattern`) |
| `maxResults` | number | no                     |
| `offset`     | number | no                     |

## find_files

Fuzzy path search, frecency-ranked.

| Parameter    | Type   | Required               |
| ------------ | ------ | ---------------------- |
| `query`      | string | yes (alias: `pattern`) |
| `maxResults` | number | no                     |
| `offset`     | number | no                     |

## index_status

No parameters. Returns JSON with FFF, AST, vector, and call graph stats.

## Recommended Agent Prompt

```
For codebase exploration, use FVA MCP tools:
- hybrid_search: default — combines file search + semantic + call graph
- semantic_search: natural language concept search
- get_smart_context: token-efficient context for a task
- get_symbol_info / get_chunks: full function/class bodies
- get_call_graph: callers and callees
Prefer hybrid_search over repeated grep+read cycles.
```
