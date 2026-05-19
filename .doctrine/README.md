# Driftlock — repo-local doctrine

Minimum viable doctrine for ADR-derived agent delivery boundaries.

## Principles

1. **Event contracts** — state transitions emit versioned audit events.
2. **State machines** — task lifecycle is explicit (`needs_review` → `ready` → `claimed` → `complete`).
3. **Audit logging** — `.driftlock/events.jsonl` is append-only authority.
4. **Testing strategy** — contract fixtures + workspace tests in CI.
5. **Errors fail closed** — unknown scope, missing evidence, stale base ref block readiness.
6. **Single source of truth** — work orders, not ADR prose, define write sets.
7. **Configuration** — lanes in `lanes/default.toml`; secrets never in events.

Canonical library: `~/prj/engineering-doctrine`
