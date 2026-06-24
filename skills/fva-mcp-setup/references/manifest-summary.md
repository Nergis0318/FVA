# FVA MCP Client Install Paths

From `examples/mcp-clients/manifest.json`.

## cursor.project

- Unix: `~/.cursor/mcp.json` or `<project>/.cursor/mcp.json`
- Windows: `%USERPROFILE%\.cursor\mcp.json` or `<project>\.cursor\mcp.json`
- Example: `cursor.project.mcp.json`

## cursor.project.windows

- Windows: `<project>\.cursor\mcp.json`
- Example: `cursor.project.windows.mcp.json`

## cursor.global

- Unix: `~/.cursor/mcp.json`
- Windows: `%USERPROFILE%\.cursor\mcp.json`
- Example: `cursor.global.mcp.json`

## claude-desktop

- macOS: `~/Library/Application Support/Claude/claude_desktop_config.json`
- Linux: `~/.config/Claude/claude_desktop_config.json`
- Windows: `%APPDATA%\Claude\claude_desktop_config.json`

## claude-code

- Project: `<project>/.mcp.json`
- CLI: `claude mcp add --transport stdio fva -- fva --path .`

## vscode

- Workspace: `<project>/.vscode/mcp.json`
- User: MCP user configuration in profile folder

## windsurf

- Unix: `~/.codeium/windsurf/mcp_config.json`
- Windows: `%USERPROFILE%\.codeium\windsurf\mcp_config.json`

## zed

- Merge `context_servers` block into Zed `settings.json`

## continue

- `<project>/.continue/mcpServers/fva.yaml`

## gemini-cli

- Merge into `~/.gemini/settings.json`

## Notes

- Replace `/path/to/your/project` with actual project root
- On Windows, prefer `.windows.json` variants or full `fva.exe` path
- Run `fva index --path .` before heavy search workloads
- Set `VOYAGE_API_KEY` when using `provider = voyage`
