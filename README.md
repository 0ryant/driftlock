# Driftlock

Driftlock turns Architecture Decision Records into evidence-backed work orders, then protects multi-agent delivery from scope drift.

The main artifact is not a todo list. The main artifact is a **TaskGraph**:

```text
ADR obligations
  -> canonical work orders
  -> repo/lane impact
  -> dependency and conflict graph
  -> agent-safe delivery briefs
  -> diff verification gates
```

## What ships in 0.1.0-rc

This repository is a production-ready workspace for the first release line:

- Rust workspace with six crates.
- Contract-first JSON Schemas under `contracts/schemas`.
- ADRs under `docs/adrs`.
- Completed task ledger under `tasks`.
- Root governance docs and crate-level metadata.
- CLI scaffold.
- MCP stdio server scaffold exposing tools, resources, and prompts.
- Skills pack for planner, worker, reviewer, maintainer, TDD, and MCP operation roles.
- TDD fixtures, golden outputs, and contract tests.

## Core invariant

> ADR prose explains intent. Driftlock work orders define delivery boundaries.

Agents must not implement directly from broad ADR prose. They must work from a ready, claimed work order with explicit evidence, write boundaries, acceptance gates, and non-goals.

## Quick start

```bash
./scripts/harden.sh
driftlock init
driftlock doctor --strict
driftlock build-graph --adr docs/adrs/0001-contracts-and-schema-first.md --repo .
driftlock ready --lane core --repo .
driftlock emit-host-config --repo .
```

See `docs/getting-started.md` and `docs/CODE_COMPLETE.md`.

## Repository layout

```text
contracts/              JSON Schemas and contract notes
crates/                 Rust workspace crates
docs/                   Architecture, operating docs, and ADRs
examples/               Fixture repo and sample MCP config
lanes/                  Lane manifests
metadata/               Root and crate metadata snapshots
prompts/                MCP prompt templates
skills/                 Blessed agent SKILL.md files
scripts/                Local verification scripts
tasks/                  Completed canonical task ledger and task graph
```

## Crates

| Crate | Purpose |
|---|---|
| `driftlock-core` | Domain model, ADR extraction, conflicts, readiness, briefs, diff verification. |
| `driftlock-git` | Repo indexing and Git diff file extraction. |
| `driftlock-contracts` | Schema export and contract fixtures. |
| `driftlock-skills` | Embedded skill and prompt resource catalog. |
| `driftlock-store` | `.driftlock/` graph, claims, and audit events. |
| `driftlock-cli` | Human and automation CLI. |
| `driftlock-mcp` | MCP stdio server for agent clients. |

## Safety defaults

Driftlock is intentionally conservative:

- Unknown safety is unsafe.
- Missing ADR evidence is non-canonical.
- Stale base refs invalidate readiness.
- Hard conflicts block delivery.
- Diff verification is required before task completion. Completing a task with
  no changed files fails closed — the MCP and CLI paths both fall back to
  `git diff` and an empty change set is treated as a verification failure.
- Scope expansion requires a new work order or maintainer override.

## Audit signing trust model

Audit-event signing keys are **not** self-trusting. `driftlock key generate`
writes the private key to `.driftlock/keys/active.ed25519` with owner-only
(0600) permissions on Unix but does **not** add it to the trust store. Trust is
an explicit operator action that pins the fingerprint out-of-band:

```bash
driftlock key generate            # prints the key_id fingerprint
driftlock key trust fp:<fingerprint>   # confirm the fingerprint, then trust
driftlock audit verify --signed   # require signatures in CI gates
```

The trust directory (`.driftlock/keys/trust/`) should be version-controlled and
reviewed; any key it contains can sign verifiable events. MCP tool paths reject
absolute paths and `..` traversal and contain all reads within the repository
root.

## Status

v0.1.0-rc.1 — wired end-to-end: `.driftlock/` state, CLI lifecycle, MCP tools (including claim/complete), contract validation, and sibling-parity CI (`quality.yml`, `governance.yml`). See `docs/PARITY_BACKLOG.md` for remaining v1.0 items.
