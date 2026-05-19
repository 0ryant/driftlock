# Architecture

Driftlock separates **decision extraction**, **delivery planning**, and **agent execution**.

## Layers

```text
Contracts       JSON Schema, Rust domain types, golden fixtures
Core            ADR extraction, task graph, conflicts, readiness, briefs
Git             Repo index and changed-file detection
CLI             Local operator workflows
MCP             Agent-facing stdio tools/resources/prompts
Skills          Blessed role-specific operating procedures
Governance      ADRs, lane policy, task ledger, audit events
```

## Data flow

```text
ADR markdown
  -> adr parser
  -> obligation candidates
  -> work orders with evidence spans
  -> lane and repo checks
  -> conflict/dependency graph
  -> ready/blocked/unsafe classification
  -> agent brief
  -> implementation diff
  -> diff verification report
```

## Non-negotiable boundary

The MCP and CLI surfaces do not define canonicality by themselves. Canonicality comes from the contract model:

- Work order ID.
- ADR evidence span.
- Current base ref.
- Lane manifest.
- Write set.
- Acceptance gates.
- Non-goals.
- Verification report.

## Extension points

Future extensions should preserve the schemas and only add optional fields unless an ADR approves a breaking contract version.

Possible additions:

- tree-sitter symbol graph
- package manager lockfile analysis
- generated artifact detection
- pull request integration
- remote claim backend
- async MCP task support
