# Driftlock MCP Operator Skill

## Purpose

Use Driftlock through the MCP stdio server safely.

## Rules

- Treat tools as read-only unless metadata says otherwise.
- Never pass secrets in tool arguments.
- Prefer resources for static skills/schemas and tools for computed graph answers.
- Use prompts for blessed workflows.
- Keep stdout reserved for MCP JSON-RPC messages when running the server.

## Recommended flow

```text
resources/list
resources/read driftlock://skills/worker
tools/call ready_tasks
tools/call agent_brief
tools/call verify_diff_against_task
```
