# ADR-0008: Local state and audit ledger

## Status

Accepted

## Context

Agent coordination needs inspectable state before it needs a distributed backend.

## Decision

Local state and audit ledger. Driftlock must preserve this decision through schemas, tests, implementation, docs, MCP behavior, and skills.

## Obligations

- Store graph, claims, and events under `.driftlock` by default.
- Use event JSONL for auditable state transitions.
- Bind claims to task ID, actor, base ref, and write set.

## Consequences

- Implementations must be traceable to this ADR.
- Task extraction must cite line ranges or sections from this ADR.
- Any future weakening requires a superseding ADR.

## Completed bootstrapping tasks

- `adr-0008:T01` — completed: Store graph, claims, and events under `.driftlock` by default.
- `adr-0008:T02` — completed: Use event JSONL for auditable state transitions.
- `adr-0008:T03` — completed: Bind claims to task ID, actor, base ref, and write set.
