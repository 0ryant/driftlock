# gaps

Source: `C:/Users/0ryant/prj/ecosystem-catalog/manager-reports/2026-05-21-ecosystem-synthesis-next-level.md`

## Goal

Prove that `driftlock` reduces implementation drift from approved ADR/spec
intent to delivered changes.

## Missing

- Standardised MCP/CLI conformance.
- Real multi-agent or multi-lane receipt showing drift reduction.
- Canonical audit chain wording aligned with actual implementation.
- Measurement of acceptance-gate pass/fail quality.

## Steps

1. Select one ADR-backed implementation task as a fixture.
2. Generate a Driftlock task graph and work orders from the approved intent.
3. Execute the task with at least two bounded lanes or simulated lanes.
4. Run acceptance gates and diff verification.
5. Compare delivered changes to the original evidence spans and write bounds.
6. Emit a receipt showing drift found, drift prevented, and accepted changes.
7. Update standardisation conformance docs with actual CLI/MCP shape.

## Acceptance evidence

- Task graph and work orders committed as fixtures.
- Diff verification report.
- Acceptance gate transcript.
- Drift metrics in the receipt.

## Stop conditions

- Do not claim hash-chain or audit-chain properties that are not implemented.
- Do not treat planning output alone as proof of drift reduction.
