# Results — write-set escape detection

The work order `adr-0007:T01` (derived from synthetic ADR-0007) authorises writes
to exactly two globs: `crates/session-store/**` and `tests/session/**`.

Two candidate diffs were verified against that write set with
`driftlock check-diff`.

| Diff | Files touched | In-scope | Escapes | `allowed` | `complete` gate |
|---|---|---|---|---|---|
| `compliant.diff` | 3 | 3 | 0 | **true** | exit 0 (would pass) |
| `escaping.diff` | 4 | 2 | **2** | **false** | exit 1 (fail-closed) |

## What escaped (escaping.diff)

`driftlock` flagged exactly the two paths outside the ADR-authorised write set:

- `crates/billing/src/charge.rs` — billing subsystem, an explicit non-goal.
- `.github/workflows/release.yml` — CI/release config, an explicit non-goal.

It did **not** flag `crates/session-store/src/lib.rs` or
`tests/session/roundtrip.rs`, which are inside the write set.

## The gate

`check-diff` is a reporter (always exits 0; the verdict is the `allowed` field).
The enforcing gate is `driftlock complete`, which calls the same
`verify_changed_files` and **bails with exit 1** when `allowed` is false — a task
can never be marked complete on a diff that escaped its write set.
