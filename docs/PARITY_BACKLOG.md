# Parity backlog — Driftlock vs siblings

**Legend:** ✅ done · 🟡 partial · ⬜ open

| Dimension | corcept | mcpact | taudit | Driftlock |
| --- | --- | --- | --- | --- |
| `.doctrine/` | ✅ | ✅ | — | ✅ |
| `deny.toml` | ✅ | ✅ | ✅ | ✅ |
| `quality.yml` matrix | ✅ | ✅ | ✅ | ✅ |
| `governance.yml` | ✅ | ✅ | ✅ | ✅ |
| `doctor --strict` | ✅ | ✅ | — | ✅ |
| `just harden` / `harden.sh` | ✅ | ✅ | ✅ | ✅ |
| Contract validate CI | ✅ | ✅ | ✅ | ✅ |
| Audit `operations.yaml` | ✅ | ✅ | cortex | ✅ |
| CloudEvents / seam-freeze | ✅ | ✅ | ✅ | ✅ |
| MCP HOSTS.md | plugin | ✅ | — | ✅ |
| Conformance stdio e2e | hooks | ✅ 17+ | — | ✅ 15 |
| Release workflow | ✅ | ✅ | ✅ | ✅ |
| crates.io publish | ✅ | ⬜ | ✅ | 🟡 |
| Official MCP SDK | — | ⬜ ADR-0021 | — | ✅ rmcp default |
| Ecosystem cross-fixture | CE | ✅ | scan | ✅ stub |

## Phase P — remaining

| ID | Task | Status |
| --- | --- | --- |
| DRIFT-P01 | `governance.yml` gitleaks + taudit scan (optional tools) | ✅ |
| DRIFT-P02 | Expand `conformance.sh` to 12+ assertions | ✅ |
| DRIFT-P03 | `conformance-taudit.yml` CI job | ✅ |
| DRIFT-P04 | Signed ledger rows + `audit verify --signed` | ✅ |
| DRIFT-P05 | crates.io publish + `PUBLISH.md` | 🟡 CI dry-run + release job |
| DRIFT-P06 | rmcp-sdk default | ✅ |
| DRIFT-P07 | Gate E SBOM + binary provenance on release | ✅ |
