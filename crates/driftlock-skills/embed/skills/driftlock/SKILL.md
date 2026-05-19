# Driftlock Skill

## Purpose

Use Driftlock to convert ADR intent into canonical, bounded work orders and prevent delivery drift.

## Hard rules

1. Never implement from ADR prose alone.
2. Implement only from a ready, claimed work order.
3. Treat unknown safety as unsafe.
4. Treat missing ADR evidence as non-canonical.
5. Verify the diff before declaring completion.
6. Do not broaden scope because the ADR seems to imply adjacent cleanup.

## Standard sequence

```text
index repo
-> inspect task graph
-> find ready tasks
-> claim one task
-> read agent brief
-> implement within write set
-> run acceptance gates
-> verify diff against task
-> complete or release claim
```

## Output discipline

Always report:

- task ID
- lane
- allowed write set
- acceptance gates run
- diff verification result
- downstream unlocks, if any
