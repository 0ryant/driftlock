# Governance

## Decision authority

All changes that alter task semantics, conflict semantics, lane policy, or MCP tool contracts require an ADR.

## Roles

| Role | Authority |
|---|---|
| Maintainer | May alter lane policy, conflict policy, schemas, and release gates. |
| Planner | May convert ADR obligations into proposed work orders. |
| Worker | May implement one ready, claimed work order. |
| Reviewer | May accept or reject a diff against the work order and contracts. |
| Release manager | May cut releases after CI, schema, and governance checks pass. |

## Overrides

Overrides are allowed only when they are explicit, auditable, and narrow.

An override must include:

- work order ID
- actor
- reason
- affected files/resources
- expiry or release boundary
- review requirement

## Invariants

1. No implementation from raw ADR prose.
2. No ready work order without evidence span.
3. No completion without diff verification.
4. No hard conflict auto-resolution.
5. No schema break without ADR and major contract version bump.
