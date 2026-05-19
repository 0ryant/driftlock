# ADR-0006: TDD-first delivery

## Status

Accepted

## Context

Contract drift is easiest to detect before implementation habits settle.

## Decision

TDD-first delivery. Driftlock must preserve this decision through schemas, tests, implementation, docs, MCP behavior, and skills.

## Obligations

- Add tests and golden fixtures before production implementation.
- Track every bootstrapping task as complete in the ledger.
- Include CI workflow and local verification scripts.

## Consequences

- Implementations must be traceable to this ADR.
- Task extraction must cite line ranges or sections from this ADR.
- Any future weakening requires a superseding ADR.

## Completed bootstrapping tasks

- `adr-0006:T01` — completed: Add tests and golden fixtures before production implementation.
- `adr-0006:T02` — completed: Track every bootstrapping task as complete in the ledger.
- `adr-0006:T03` — completed: Include CI workflow and local verification scripts.
