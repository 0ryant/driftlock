# ADR-0005: Skill pack as runtime governance

## Status

Accepted

## Context

Tool descriptions are not enough to keep agents within canonical workflows.

## Decision

Skill pack as runtime governance. Driftlock must preserve this decision through schemas, tests, implementation, docs, MCP behavior, and skills.

## Obligations

- Ship role-specific SKILL.md files with the crate.
- Expose skill content as MCP resources.
- Expose blessed workflows as MCP prompts.

## Consequences

- Implementations must be traceable to this ADR.
- Task extraction must cite line ranges or sections from this ADR.
- Any future weakening requires a superseding ADR.

## Completed bootstrapping tasks

- `adr-0005:T01` — completed: Ship role-specific SKILL.md files with the crate.
- `adr-0005:T02` — completed: Expose skill content as MCP resources.
- `adr-0005:T03` — completed: Expose blessed workflows as MCP prompts.
