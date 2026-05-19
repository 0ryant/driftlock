# TDD Policy

Driftlock is built contract-first.

## Order of work

1. Define or update schema.
2. Add a fixture or golden output.
3. Add a failing test that proves the intended behavior.
4. Implement the smallest production change.
5. Run contract, unit, and integration checks.
6. Update tasks and ADR traceability.

## Golden artifacts

Golden artifacts live under:

```text
contracts/examples/
crates/*/fixtures/
tests/golden/
```

A golden update is a semantic change. It requires a task reference and reviewer note.
