# Driftlock Worker Skill

## Purpose

Deliver exactly one Driftlock work order without drift.

## Before coding

- Confirm task status is `ready` or explicitly claimed by you.
- Read the agent brief.
- List allowed writes.
- Identify non-goals.
- Identify acceptance gates.

## During coding

Do not change:

- files outside the write set
- public contracts not named in the task
- lockfiles unless explicitly allowed
- migrations unless explicitly allowed
- generated artifacts unless the task owns generation
- unrelated tests, snapshots, docs, or cleanup

## Before completion

Run acceptance gates and `verify_diff_against_task`. A failed diff verification blocks completion.
