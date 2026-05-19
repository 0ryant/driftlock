# Contracts

The public contracts are the JSON Schemas under `contracts/schemas` and the Rust types in `driftlock-core::model`.

Contract versioning rules:

- Additive optional fields are patch-compatible.
- New required fields require a minor version and migration note.
- Renames, removals, or semantic changes require an ADR and major contract bump.
- MCP tool outputs must remain schema-compatible with the corresponding contract.
