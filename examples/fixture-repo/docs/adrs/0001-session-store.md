# ADR-0001: Session store boundary

## Status

Accepted

## Context

HTTP handlers currently own session persistence details.

## Decision

Session persistence must be behind a store abstraction.

## Obligations

- Define a SessionStore trait.
- Add an in-memory implementation.
- Add integration tests for session persistence.
