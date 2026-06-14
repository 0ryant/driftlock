# Changelog

## Unreleased

- Audit log is now a genuine **hash chain**: each `events.jsonl` row carries a
  `prev_hash` linking it to the SHA-256 of the previous row (genesis = 64 hex
  zeros). `audit verify` fails closed on a broken chain, detecting row deletion,
  reordering, and in-place edits — even for unsigned rows. On signed rows the
  link is folded into the signing preimage. Closes the "hash-chained" wording
  gap (the claim is now real and tested).

## 0.1.0-rc.1 - 2026-05-19

- Wired `.driftlock/` persistence (`driftlock-store`), full CLI lifecycle, MCP mutating tools.
- Ed25519 signed event lines, `driftlock key generate`, `driftlock audit verify --signed`.
- MCP default transport: official `rmcp` SDK; shared `service.rs`; `manual-stdio` optional.
- CI: quality + governance + release + conformance-taudit; publish dry-run; SBOM + SLSA binary provenance on tag.
- Harden/conformance gates (15 checks).
- Docs: `CODE_COMPLETE.md`, `PARITY_BACKLOG.md`, `PUBLISH.md`, `VERSIONING.md`.

## 0.1.0-scaffold - 2026-05-19

- Added contract-first schemas.
- Added Rust workspace with core, git, contracts, skills, CLI, and MCP crates.
- Added MCP stdio server scaffold with tools/resources/prompts.
- Added completed bootstrapping ADR/task ledger.
- Added skills pack and prompt templates.
- Added governance and root docs.
