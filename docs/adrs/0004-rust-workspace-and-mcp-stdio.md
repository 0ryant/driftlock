# ADR-0004: Rust workspace and MCP stdio

## Status

Accepted

## Context

Rust gives small binaries and strong types; stdio is the simplest local MCP transport.

## Decision

Rust workspace and MCP stdio. Driftlock must preserve this decision through schemas, tests, implementation, docs, MCP behavior, and skills.

## Obligations

- Implement core logic as a reusable Rust library.
- Provide a CLI for local workflows.
- Provide an MCP stdio server exposing tools, resources, and prompts.

## Consequences

- Implementations must be traceable to this ADR.
- Task extraction must cite line ranges or sections from this ADR.
- Any future weakening requires a superseding ADR.

## Completed bootstrapping tasks

- `adr-0004:T01` — completed: Implement core logic as a reusable Rust library.
- `adr-0004:T02` — completed: Provide a CLI for local workflows.
- `adr-0004:T03` — completed: Provide an MCP stdio server exposing tools, resources, and prompts.
