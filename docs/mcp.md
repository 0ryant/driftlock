# MCP Surface

The MCP server exposes:

## Tools

- `index_repo`
- `extract_tasks`
- `build_task_graph`
- `check_conflicts`
- `ready_tasks`
- `agent_brief`
- `verify_diff_against_task`
- `list_skills`
- `get_skill`
- `export_schemas`

## Resources

- `driftlock://schemas/taskgraph`
- `driftlock://schemas/work-order`
- `driftlock://schemas/lane-manifest`
- `driftlock://skills/driftlock`
- `driftlock://skills/planner`
- `driftlock://skills/worker`
- `driftlock://skills/reviewer`
- `driftlock://skills/maintainer`
- `driftlock://prompts/worker-start`
- `driftlock://prompts/conflict-review`

## Prompts

- `driftlock.worker_start`
- `driftlock.planner_extract_adr`
- `driftlock.reviewer_gate`
- `driftlock.conflict_review`
- `driftlock.maintainer_refresh`

The server uses stdio and must never print non-MCP data to stdout.
