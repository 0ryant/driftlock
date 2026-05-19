# Versioning

Driftlock follows [Semantic Versioning](https://semver.org/) for CLI and contract surfaces.

| Surface | Policy |
| --- | --- |
| `dev.driftlock.*.v1` events | Breaking changes require new type suffix (`v2`) |
| JSON Schemas under `contracts/` | Additive by default; breaking bumps need ADR |
| Rust public APIs in `driftlock-core` | Semver per crate |
| MCP tool names | Stable; new tools additive |

**Pre-1.0:** `0.1.0-rc.N` may include breaking changes with changelog notes.

**Promotion:** `rc` → `0.1.0` when Gates A–D in `docs/CODE_COMPLETE.md` are green and parity P01–P04 are done.
