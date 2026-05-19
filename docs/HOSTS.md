# MCP host configuration

Driftlock exposes an MCP stdio server via `driftlock-mcp`.

## Generate configs

```bash
driftlock emit-host-config --repo . --out .driftlock/hosts
```

## Cursor

Use `.driftlock/hosts/cursor.json` or merge into Cursor MCP settings.

## Claude Code / Codex

Use `claude.json` / `codex.json` from the same directory.

## Manual

```json
{
  "mcpServers": {
    "driftlock": {
      "command": "cargo",
      "args": ["run", "-p", "driftlock-mcp", "--", "stdio", "--repo", "/absolute/path/to/repo"]
    }
  }
}
```

Stdout must remain protocol-clean (JSON-RPC only).
