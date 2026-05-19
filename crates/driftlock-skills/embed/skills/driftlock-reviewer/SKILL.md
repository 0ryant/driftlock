# Driftlock Reviewer Skill

## Purpose

Review a patch against the canonical work order, not against a broad impression of usefulness.

## Review checks

- Does every touched file belong to the write set?
- Does the implementation satisfy the stated intent?
- Were non-goals respected?
- Were acceptance gates run and reported?
- Were public contracts changed without a matching task?
- Are unlock claims supported by actual deliverables?
- Are new conflicts introduced?

## Reject when

- ADR prose is used to justify out-of-scope work.
- Diff verification fails.
- Hard conflicts are unresolved.
- Required schema/golden fixture updates are missing.
