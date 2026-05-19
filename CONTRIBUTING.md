# Contributing

## Local checks

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p driftlock-contracts --bin export-schemas -- contracts/schemas
python3 scripts/verify_scaffold.py
```

## Change classes

| Change | Required evidence |
|---|---|
| Domain model change | ADR or task update plus schema test. |
| Conflict algorithm change | Fixture proving false-negative risk does not increase silently. |
| MCP tool change | Tool schema update, prompt/skill review, compatibility note. |
| Lane policy change | Governance note and example lane manifest update. |
| Skill change | Skill pack version bump. |

## Test discipline

This project is contract-first and TDD-oriented. Add or update a failing test/fixture before changing behavior.
