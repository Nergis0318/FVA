---
name: fva-mcp-setup
description: >
  Configure FVA MCP server for AI coding agents (Cursor, Claude Code, VS Code,
  Windsurf, Zed, Continue, Gemini CLI, Cline). Trigger when setting up FVA MCP,
  editing mcp.json, connecting fva to an agent, or troubleshooting empty search
  results from FVA tools.
---

# FVA MCP Setup

Connect the `fva` binary to AI agent MCP clients via stdio transport.

## Prerequisites

1. Install `fva` — see README install scripts or `cargo install --path .`
2. Verify: `fva --version`
3. Index the project once: `fva index --path <project-root>`

## Generic Config

```json
{
  "mcpServers": {
    "fva": {
      "command": "fva",
      "args": ["--path", "/path/to/your/project"],
      "env": { "RUST_LOG": "info" }
    }
  }
}
```

**Windows** — use full path if not on PATH:

```json
{
  "mcpServers": {
    "fva": {
      "command": "C:\\Users\\You\\AppData\\Local\\Programs\\fva\\bin\\fva.exe",
      "args": ["--path", "D:\\Dev\\YourProject"],
      "env": { "RUST_LOG": "info" }
    }
  }
}
```

## Agent-Specific Install Paths

Ready-to-copy examples live in `examples/mcp-clients/`. See `manifest.json` for paths.

| Agent | Example file | Install location |
| --- | --- | --- |
| Cursor (project) | `cursor.project.mcp.json` | `<project>/.cursor/mcp.json` |
| Cursor (global) | `cursor.global.mcp.json` | `~/.cursor/mcp.json` |
| Claude Code | `claude-code.project.mcp.json` | `<project>/.mcp.json` |
| Claude Desktop | `claude-desktop.*.json` | OS-specific — see manifest |
| VS Code / Copilot | `vscode.workspace.mcp.json` | `<project>/.vscode/mcp.json` |
| Windsurf | `windsurf.mcp_config.json` | `~/.codeium/windsurf/mcp_config.json` |
| Zed | `zed.context_servers.json` | Merge into Zed `settings.json` |
| Continue | `continue.fva.yaml` | `<project>/.continue/mcpServers/` |
| Gemini CLI | `gemini-cli.settings.json` | `~/.gemini/settings.json` |
| Cline / Roo Code | `cline.mcp_settings.json` | Extension MCP settings |

### Claude Code CLI shortcut

```bash
claude mcp add --transport stdio fva -- fva --path .
```

### Cursor project template

Use `${workspaceFolder}` for the project path:

```json
{
  "mcpServers": {
    "fva": {
      "type": "stdio",
      "command": "fva",
      "args": ["--path", "${workspaceFolder}"],
      "env": { "RUST_LOG": "info" }
    }
  }
}
```

Copy from `examples/mcp-clients/cursor.project.mcp.json`.

## Post-Setup Checklist

1. Restart the MCP client (or reload MCP servers)
2. Confirm `index_status` returns non-zero `indexed_files`
3. Run a test `hybrid_search` query
4. Add the recommended agent prompt (see `fva` skill or README)

## Troubleshooting

| Symptom | Fix |
| --- | --- |
| Tool not found | Check `command` path; verify `fva --version` in same shell |
| Empty search results | Run `fva index --path .` then retry |
| Stale results | Re-index or enable `watch = true` in config |
| Voyage errors | Set `VOYAGE_API_KEY` or switch `provider = "local"` |
| Permission denied (Unix) | `chmod +x` on binary; ensure PATH includes install dir |

## Optional: Voyage Embeddings

```toml
# fva.toml or ~/.config/fva/config.toml
[embedding]
provider = "voyage"
```

```bash
export VOYAGE_API_KEY=your-key-here
```

## Further Reference

See [references/manifest-summary.md](references/manifest-summary.md) for full install path matrix.