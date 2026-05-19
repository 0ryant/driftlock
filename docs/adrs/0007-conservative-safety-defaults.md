# ADR-0007: Conservative safety defaults

## Status

Accepted

## Context

False safety is more damaging than conservative blocking.

## Decision

Conservative safety defaults. Driftlock must preserve this decision through schemas, tests, implementation, docs, MCP behavior, and skills.

## Obligations

- Treat unknown file scope as unsafe.
- Treat missing ADR evidence as non-canonical.
- Block ready status when hard conflicts or unsatisfied dependencies exist.

## Consequences

- Implementations must be traceable to this ADR.
- Task extraction must cite line ranges or sections from this ADR.
- Any future weakening requires a superseding ADR.

## Completed bootstrapping tasks

- `adr-0007:T01` — completed: Treat unknown file scope as unsafe.
- `adr-0007:T02` — completed: Treat missing ADR evidence as non-canonical.
- `adr-0007:T03` — completed: Block ready status when hard conflicts or unsatisfied dependencies exist.
