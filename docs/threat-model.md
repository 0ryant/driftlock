# Threat Model

## Assets

- canonical work order graph
- lane policy
- claim state
- ADR evidence
- MCP tool and prompt definitions
- repo working tree

## Adversarial or failure inputs

- malicious ADR text
- stale lane manifest
- injected prompt instructions in source files
- compromised MCP client
- buggy planner agent
- worker agent that broadens scope

## Mitigations

- Work from work orders, not ADR prose.
- Bind work orders to evidence spans and base refs.
- Treat unknown safety as unsafe.
- Reject write-set violations.
- Make overrides explicit and auditable.
- Keep MCP stdout protocol-clean.
