# ADR-0001: Contracts and schema first

## Status

Accepted

## Context

Driftlock coordinates agents; ambiguous contracts create unsafe autonomy.

## Decision

Contracts and schema first. Driftlock must preserve this decision through schemas, tests, implementation, docs, MCP behavior, and skills.

## Obligations

- Define JSON Schemas before implementation behavior.
- Generate or mirror Rust types from the schema contract.
- Add contract tests for representative golden examples.

## Consequences

- Implementations must be traceable to this ADR.
- Task extraction must cite line ranges or sections from this ADR.
- Any future weakening requires a superseding ADR.

## Completed bootstrapping tasks

- `adr-0001:T01` — completed: Define JSON Schemas before implementation behavior.
- `adr-0001:T02` — completed: Generate or mirror Rust types from the schema contract.
- `adr-0001:T03` — completed: Add contract tests for representative golden examples.
