# ADR-0003: Lane manifest and conflict model

## Status

Accepted

## Context

Multi-agent delivery fails when disjointness is inferred from intent rather than enforced boundaries.

## Decision

Lane manifest and conflict model. Driftlock must preserve this decision through schemas, tests, implementation, docs, MCP behavior, and skills.

## Obligations

- Define lane allowlists, read lists, and exclusive resources.
- Detect same-file, exclusive-resource, and contract-level conflicts.
- Classify conflicts as hard, soft, or unknown.

## Consequences

- Implementations must be traceable to this ADR.
- Task extraction must cite line ranges or sections from this ADR.
- Any future weakening requires a superseding ADR.

## Completed bootstrapping tasks

- `adr-0003:T01` — completed: Define lane allowlists, read lists, and exclusive resources.
- `adr-0003:T02` — completed: Detect same-file, exclusive-resource, and contract-level conflicts.
- `adr-0003:T03` — completed: Classify conflicts as hard, soft, or unknown.
