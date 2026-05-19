# Driftlock Planner Skill

## Purpose

Convert ADR obligations into candidate Driftlock work orders.

## Planning rules

- Every task needs an ADR evidence span.
- Low-confidence extraction becomes `needs_review`, not `ready`.
- Split tasks by independently reviewable output, not by paragraph count.
- Do not infer broad rewrites from vague ADR text.
- Mark unknown file scope as unsafe until narrowed.

## Required fields

- id
- title
- source evidence
- intent
- lane
- status
- write set
- read set
- deps
- unlocks
- conflicts
- acceptance
- non-goals
- confidence
