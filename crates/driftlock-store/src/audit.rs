//! `axiom.audit.v1` append-only, BLAKE3-chained audit trail.
//!
//! Per ecosystem-catalog pattern 09 (audit-chain) and engineering-doctrine
//! audit-logging §2 (Integrity & Immutability — append-only, hash-chained,
//! tamper-evident), every mutating Driftlock operation appends a single JSONL row
//! to `<repo>/audit-trail.jsonl`. The row is chained: `row_hash =
//! BLAKE3(JCS(row_without_row_hash))` and `prev_hash` is the previous row's
//! `row_hash`. A break in the chain (mismatched link, gap in `seq`, a wrong
//! schema tag, or a `row_hash` that no longer recomputes) is a verification
//! failure.
//!
//! Phase-3 `axiom-*` keystone migration: the row type, append, and chain
//! verification live in the shared [`axiom_audit`] crate so the on-disk line and
//! `row_hash` are byte-identical to the three reference tools (tflip / tboundary /
//! tinterleave). This module keeps Driftlock's thin wrapper and re-exports the
//! row/verdict types.
//!
//! Relationship to `.driftlock/events.jsonl`: the signed-event ledger
//! ([`crate::signing`]) is Driftlock's domain ledger (Ed25519-signed work-order
//! lifecycle events). The `audit-trail.jsonl` here is the doctrine custody trail
//! that records *which Driftlock operation ran*, its outcome and exit code, and
//! links to the [`crate::receipt`] each operation emits. The two are
//! complementary, not duplicates.

use std::path::Path;

use anyhow::Result;

pub use axiom_audit::{
    AuditRow, ChainVerdict, ReceiptLink, AUDIT_SCHEMA, GENESIS_HASH, TRAIL_FILENAME,
};

/// Canonical tool name stamped on every audit row.
pub const TOOL_NAME: &str = "driftlock";

/// Tool version recorded in audit rows and receipts.
pub const TOOL_VERSION: &str = env!("CARGO_PKG_VERSION");

/// One operation to record on the trail. The chain bookkeeping
/// (`seq`/`prev_hash`/`row_hash`) is derived by [`append`].
#[derive(Debug, Clone)]
pub struct AuditEntry {
    /// `"<verb> <subverb?>"`, e.g. `"build-graph"`, `"complete"`.
    pub operation: String,
    /// `"ok" | "failed" | "degraded"`.
    pub outcome: String,
    /// Process exit code for the operation (pattern 11).
    pub exit_code: i32,
    /// Repo-relative path to the receipt this row links, if any.
    pub receipt_path: Option<String>,
    /// Lowercase-hex BLAKE3 of the receipt file, if any.
    pub receipt_blake3: Option<String>,
}

/// Returns `<repo>/audit-trail.jsonl`.
#[must_use]
pub fn trail_path(repo_root: &Path) -> std::path::PathBuf {
    repo_root.join(TRAIL_FILENAME)
}

/// Read all rows from `<repo>/audit-trail.jsonl`. Empty vec if absent.
pub fn read_trail(repo_root: &Path) -> Result<Vec<AuditRow>> {
    Ok(axiom_audit::read_rows(&trail_path(repo_root))?)
}

/// Append one `axiom.audit.v1` row to `<repo>/audit-trail.jsonl`, deriving
/// `seq`/`prev_hash`/`row_hash` from the existing tail. Returns the appended row.
///
/// Driftlock omits the two `receipt_*` keys when no receipt is linked
/// ([`axiom_audit::ReceiptLink::None`]) and writes their values when present.
pub fn append(repo_root: &Path, timestamp: &str, entry: &AuditEntry) -> Result<AuditRow> {
    let receipt = match (&entry.receipt_path, &entry.receipt_blake3) {
        (None, None) => ReceiptLink::None,
        (path, blake3) => ReceiptLink::Present {
            path: path.clone().unwrap_or_default(),
            blake3: blake3.clone().unwrap_or_default(),
        },
    };
    let row = axiom_audit::append(
        &trail_path(repo_root),
        &axiom_audit::AuditEntry {
            tool: TOOL_NAME.to_string(),
            tool_version: TOOL_VERSION.to_string(),
            operation: entry.operation.clone(),
            timestamp: timestamp.to_string(),
            outcome: entry.outcome.clone(),
            exit_code: entry.exit_code,
            receipt,
        },
    )?;
    Ok(row)
}

/// Verify the `<repo>/audit-trail.jsonl` chain end to end (pattern 09). Returns a
/// typed [`ChainVerdict`]; never panics on adversarial input.
pub fn verify_chain(repo_root: &Path) -> Result<ChainVerdict> {
    Ok(axiom_audit::verify_chain(&trail_path(repo_root))?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(op: &str, exit: i32) -> AuditEntry {
        AuditEntry {
            operation: op.to_string(),
            outcome: if exit == 0 { "ok" } else { "failed" }.to_string(),
            exit_code: exit,
            receipt_path: None,
            receipt_blake3: None,
        }
    }

    #[test]
    fn append_chains_and_verifies() {
        let dir = tempfile::tempdir().unwrap();
        let r0 = append(dir.path(), "2026-06-16T00:00:00+00:00", &entry("build-graph", 0)).unwrap();
        let r1 = append(dir.path(), "2026-06-16T00:00:01+00:00", &entry("complete", 0)).unwrap();
        assert_eq!(r0.seq, 0);
        assert_eq!(r0.prev_hash, GENESIS_HASH);
        assert_eq!(r0.schema, AUDIT_SCHEMA);
        assert_eq!(r1.seq, 1);
        assert_eq!(r1.prev_hash, r0.row_hash);

        match verify_chain(dir.path()).unwrap() {
            ChainVerdict::Valid { rows, head_hash } => {
                assert_eq!(rows, 2);
                assert_eq!(head_hash, r1.row_hash);
            }
            other @ ChainVerdict::Broken(_) => panic!("expected Valid, got {other:?}"),
        }
    }

    #[test]
    fn empty_trail_is_valid() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            verify_chain(dir.path()).unwrap(),
            ChainVerdict::Valid { rows: 0, head_hash: String::new() }
        );
    }

    #[test]
    fn tampered_row_breaks_chain() {
        let dir = tempfile::tempdir().unwrap();
        append(dir.path(), "2026-06-16T00:00:00+00:00", &entry("claim", 0)).unwrap();
        let mut rows = read_trail(dir.path()).unwrap();
        rows[0].exit_code = 99; // stale row_hash now.
        let line = serde_json::to_string(&rows[0]).unwrap();
        std::fs::write(trail_path(dir.path()), format!("{line}\n")).unwrap();
        assert!(matches!(verify_chain(dir.path()).unwrap(), ChainVerdict::Broken(_)));
    }
}
