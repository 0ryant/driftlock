# Code complete path

## Proof commands

```bash
just harden          # or ./scripts/harden.sh
just conformance
cargo run -p driftlock-cli -- doctor --strict
bash scripts/release_smoke.sh
```

## Gates

| Gate | Status | Proof |
|------|--------|-------|
| A Scaffold | ✅ | workspace |
| B MVP product | ✅ | `.driftlock/` lifecycle + CLI + MCP mutating tools |
| C Verification | ✅ | harden + validate_contracts + audit ops |
| B+ Host-ready | ✅ | HOSTS.md + emit-host-config |
| P Sibling parity | ✅ | PARITY_BACKLOG.md (P05 publish needs token) |
| E v1.0 ship | 🟡 | release attest + crates.io token |

See [BUILD_SPEC.md](BUILD_SPEC.md), [PARITY_BACKLOG.md](PARITY_BACKLOG.md).
