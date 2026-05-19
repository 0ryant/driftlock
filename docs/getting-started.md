# Getting Started

1. Write or select an ADR.
2. Define lanes in `lanes/default.toml`.
3. Extract work orders.
4. Review proposed tasks.
5. Query ready tasks.
6. Claim one task.
7. Implement narrowly.
8. Verify the diff.
9. Complete the task and unlock downstream work.

```bash
driftlock init
driftlock doctor --strict
driftlock extract --adr docs/adrs/0002-taskgraph-primary-artifact.md --lanes lanes/default.toml --lane core
driftlock build-graph --adr docs/adrs/0002-taskgraph-primary-artifact.md --repo .
driftlock ready --lane core --repo .
driftlock claim --graph .driftlock/graph.json --task adr-0002:T01 --actor you
driftlock brief --graph .driftlock/graph.json --task adr-0002:T01
driftlock check-diff --graph .driftlock/graph.json --task adr-0002:T01 --repo .
driftlock complete --graph .driftlock/graph.json --task adr-0002:T01 --actor you --repo .
driftlock emit-host-config --repo .
driftlock key generate --repo .
driftlock audit verify --repo . --signed
```

Signed events require `key generate` before claims append to `events.jsonl`. Use `audit verify` without `--signed` to accept legacy unsigned rows.

MCP: see [HOSTS.md](HOSTS.md). Harden gate: `./scripts/harden.sh`.
