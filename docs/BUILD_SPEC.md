# BUILD_SPEC — Driftlock v0.1

| Criterion | Proof | Status |
| --- | --- | --- |
| Workspace builds | `cargo build --workspace` | ✅ |
| Tests pass | `cargo test --workspace` | ✅ |
| Clippy clean | `cargo clippy -D warnings` | ✅ |
| Schemas export idempotent | `export-schemas` in harden | ✅ |
| Contract fixtures validate | `validate_contracts.py` | ✅ |
| Audit ops registry | `validate_audit_operations.py` | ✅ |
| `.driftlock` lifecycle | `init` → `claim` → `complete` | ✅ |
| Diff verification gate | `check-diff` / `complete` | ✅ |
| MCP stdio tools | `conformance.sh` | 🟡 |
| Multi-host configs | `emit-host-config` + HOSTS.md | ✅ |
| Doctor strict | `driftlock doctor --strict` | ✅ |

Subtask IDs: `DRIFT-001` … see `CODE_COMPLETE.md`.
