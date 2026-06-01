# MCP Surface

Driftlock ships an MCP stdio server (`driftlock-mcp`). It advertises
`serverInfo.name = "driftlock-mcp"`, `serverInfo.version` = the crate version,
and `protocolVersion = 2025-06-18` on both the default (`rmcp` SDK) and the
optional manual-stdio transports. The server uses stdio and must never print
non-MCP data to stdout.

## Tools

Planning and inspection:

- `index_repo` — conservative repository file inventory.
- `extract_tasks` — proposed work orders from an ADR file.
- `build_task_graph` — task graph from an ADR and lane manifest.
- `check_conflicts` — conflict report for a task graph.
- `ready_tasks` — tasks ready for a lane against the current base.
- `agent_brief` — bounded implementation brief for one work order.
- `verify_diff_against_task` — check changed files against a work order write set.
- `list_skills` — list embedded Driftlock skills.
- `get_skill` — return one embedded Driftlock skill.
- `export_schemas` — return the contract schema bundle.

Claim lifecycle (these write to `.driftlock/` in the repo):

- `claim_task` — claim a ready work order for an actor.
- `release_task` — release a task claim.
- `complete_task` — complete a claimed task after diff verification passes.

## Resources

Schemas:

- `driftlock://schemas/taskgraph`
- `driftlock://schemas/work-order`
- `driftlock://schemas/lane-manifest`

Skills:

- `driftlock://skills/driftlock`
- `driftlock://skills/planner`
- `driftlock://skills/worker`
- `driftlock://skills/reviewer`
- `driftlock://skills/maintainer`
- `driftlock://skills/tdd`
- `driftlock://skills/mcp-operator`

Prompts (also addressable as resources):

- `driftlock://prompts/worker-start`
- `driftlock://prompts/planner-extract-adr`
- `driftlock://prompts/reviewer-gate`
- `driftlock://prompts/conflict-review`
- `driftlock://prompts/maintainer-refresh`
- `driftlock://prompts/agent-brief-template`

## Prompts

- `driftlock.worker_start`
- `driftlock.planner_extract_adr`
- `driftlock.reviewer_gate`
- `driftlock.conflict_review`
- `driftlock.maintainer_refresh`
- `driftlock.agent_brief_template`

## Running the server

From a checkout (no install required):

```sh
cargo run -p driftlock-mcp -- stdio --repo .
```

Installed on the `PATH` (recommended for MCP clients). Once the crate is
published to crates.io (see `docs/PUBLISH.md`):

```sh
cargo install driftlock-mcp
driftlock-mcp stdio --repo .
```

Before a crates.io release, install the same binary directly from the
repository:

```sh
cargo install --git https://github.com/0ryant/driftlock driftlock-mcp
# or, from a local checkout:
cargo install --path crates/driftlock-mcp
```

See `examples/mcp-client-config.example.json` for both `cargo run` and
installed-binary client configurations.
