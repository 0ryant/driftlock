# Seam freeze v1 — Driftlock ecosystem fields

**Status:** frozen for v0.1.0-rc  
**Scope:** JSONL event metadata extensions shared with mcpact / taudit joins

## Frozen extensions (v1)

| Field | Type | Producer | Join key |
| --- | --- | --- | --- |
| `correlationid` | string | Driftlock CLI/MCP, CI | primary |
| `provenancerepo` | string | runtime | secondary |
| `provenanceproducer` | string | `driftlock` | secondary |
| `provenanceversion` | string | crate version | secondary |
| `provenancekind` | string | `execution`, `lifecycle` | filter |

## Environment injection

| Variable | Maps to |
| --- | --- |
| `DRIFTLOCK_CORRELATION_ID` | `correlationid` |
| `DRIFTLOCK_PROVENANCE_REPO` | `provenancerepo` |
| `DRIFTLOCK_PROVENANCE_PRODUCER` | `provenanceproducer` |
| `DRIFTLOCK_PROVENANCE_VERSION` | `provenanceversion` |
| `DRIFTLOCK_PROVENANCE_KIND` | `provenancekind` |

## Join set (v1)

- `dev.driftlock.task.claimed.v1`
- `dev.driftlock.task.completed.v1`
- `dev.mcpact.tool.executed.v1` (sibling)
- `dev.taudit.finding.v1` (sibling)

Change process: add `seam-freeze-v2.md` for breaking extension changes.
