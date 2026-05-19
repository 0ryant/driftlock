# ADR-0002: TaskGraph as primary artifact

## Status

Accepted

## Context

A todo list cannot encode dependency and safety relationships.

## Decision

TaskGraph as primary artifact. Driftlock must preserve this decision through schemas, tests, implementation, docs, MCP behavior, and skills.

## Obligations

- Represent ADR obligations as work orders in a graph.
- Attach ADR evidence spans to every work order.
- Expose readiness, dependency, conflict, and unlock queries.

## Consequences

- Implementations must be traceable to this ADR.
- Task extraction must cite line ranges or sections from this ADR.
- Any future weakening requires a superseding ADR.

## Completed bootstrapping tasks

- `adr-0002:T01` — completed: Represent ADR obligations as work orders in a graph.
- `adr-0002:T02` — completed: Attach ADR evidence spans to every work order.
- `adr-0002:T03` — completed: Expose readiness, dependency, conflict, and unlock queries.
