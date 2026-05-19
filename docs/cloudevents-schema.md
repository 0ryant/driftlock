# Driftlock event catalog

Events are stored as JSON lines in `.driftlock/events.jsonl` (see `contracts/schemas/event.schema.json`).

| Event type | When emitted |
| --- | --- |
| `dev.driftlock.graph.built.v1` | `build-graph`, `refresh` |
| `dev.driftlock.task.claimed.v1` | `claim` |
| `dev.driftlock.task.released.v1` | `release` |
| `dev.driftlock.task.completed.v1` | `complete` (after diff OK) |
| `dev.driftlock.conflict.detected.v1` | `refresh` / conflict recompute |

Extensions follow [seam-freeze-v1.md](seam-freeze-v1.md).
