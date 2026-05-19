# Glossary

**ADR**: Architecture Decision Record.

**Work order**: Canonical, bounded implementation unit derived from ADR evidence.

**TaskGraph**: Graph of work orders, dependencies, conflicts, lanes, and readiness state.

**Lane**: Allowlist/readlist/exclusive-resource policy for agent delivery.

**Hard conflict**: Conflict that blocks concurrent work.

**Soft conflict**: Conflict that requires explicit review but may not block.

**Unlock**: A downstream task that becomes possible or less ambiguous after another task completes.

**Drift**: Any implementation choice that leaves the canonical work order boundary.
