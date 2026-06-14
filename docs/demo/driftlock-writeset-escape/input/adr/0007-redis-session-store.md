# ADR-0007: Redis-backed session store

- Status: Accepted
- Date: 2026-06-12
- Revision: scaffold-demo

## Context

The `acme-demo` service (synthetic) keeps user sessions in process memory, so
sessions are lost on restart and cannot be shared across replicas. We need a
durable, shared session store.

## Decision

Introduce a Redis-backed session store as a new crate, `session-store`.

The work order derived from this decision is **bounded to the session-store
crate and its tests**. It MUST NOT touch unrelated subsystems such as billing.

## Authorised write boundary (work order adr-0007:T01)

The single canonical work order for this ADR may write only:

- `crates/session-store/**` — the new crate
- `tests/session/**` — its integration tests

## Non-goals

- Changing the billing subsystem (`crates/billing/**`).
- Touching CI or release configuration.

## Acceptance

- Sessions survive a process restart.
- No write occurs outside the authorised write boundary above.
