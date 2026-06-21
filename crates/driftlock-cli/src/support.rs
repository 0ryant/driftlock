//! Exit-code classification (pattern 11) and audit/receipt emission for the CLI.
//!
//! Every fallible CLI path returns [`CliError`], which carries an
//! [`axiom_exit::Exit`] so `main` maps process status through the shared
//! pattern-11 state machine: `0` ok, `1` verify-mismatch ONLY, `2` usage, `3`
//! preflight (IO / parse / write-set escape), `4` degraded, `>=64` tool-specific.
//! Codes `5..=63` are never emitted.

use std::collections::BTreeMap;
use std::path::Path;

use axiom_exit::Exit;
use driftlock_core::DiffReport;
use driftlock_store::{
    append_audit, build_checkpoint, build_receipt, save_checkpoint, AuditEntry, AuditLink, Receipt,
    ReceiptInput, StatePaths,
};

/// A classified CLI error: a human message plus the pattern-11 [`Exit`] code it
/// maps to. `main` never collapses every failure to `1`.
#[derive(Debug)]
pub struct CliError {
    /// Operator-facing message printed to stderr.
    pub message: String,
    /// The pattern-11 exit classification.
    pub exit: Exit,
}

impl CliError {
    /// A `FAILED_PREFLIGHT` (exit 3): IO, parse, missing input, or a write-set
    /// escape detected before/at execution.
    pub fn preflight(msg: impl Into<String>) -> Self {
        Self { message: msg.into(), exit: Exit::Preflight }
    }

    /// A `USAGE_ERROR` (exit 2): the operator invoked the verb incorrectly in a
    /// way clap could not catch (e.g. a referenced task does not exist).
    pub fn usage(msg: impl Into<String>) -> Self {
        Self { message: msg.into(), exit: Exit::Usage }
    }

    /// An `ASSERTION_FAILED` (exit 1): a verify / chain mismatch ONLY.
    pub fn assertion(msg: impl Into<String>) -> Self {
        Self { message: msg.into(), exit: Exit::AssertionFailed }
    }

    /// A `DEGRADED` (exit 4): the operation completed but in a reduced mode.
    ///
    /// Part of the pattern-11 taxonomy for completeness. Driftlock has no
    /// genuine degraded execution path today (every verb either fully succeeds or
    /// hits a classified failure), so this constructor is currently unused rather
    /// than wired to a fabricated degraded mode.
    #[allow(dead_code)]
    pub fn degraded(msg: impl Into<String>) -> Self {
        Self { message: msg.into(), exit: Exit::Degraded }
    }
}

impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for CliError {}

/// Map an [`std::io::Error`] to a preflight failure (exit 3).
pub fn io_err(context: &str) -> impl FnOnce(std::io::Error) -> CliError + '_ {
    move |e| CliError::preflight(format!("{context}: {e}"))
}

/// Map a `serde_json` error to a preflight failure (exit 3).
pub fn json_err(context: &str) -> impl FnOnce(serde_json::Error) -> CliError + '_ {
    move |e| CliError::preflight(format!("{context}: {e}"))
}

/// Convenience result alias for CLI verbs.
pub type CliResult<T> = Result<T, CliError>;

/// Record one mutating operation: append an `axiom.audit.v1` row to
/// `<repo>/audit-trail.jsonl`, then emit a signed `axiom.receipt.v1` receipt to
/// `.driftlock/receipts/<seq>-<operation>.json` linked to that row.
///
/// Returns the written receipt. Audit/receipt write failures are surfaced as
/// preflight errors (exit 3) so a custody-write failure never passes silently.
pub fn record_operation(
    paths: &StatePaths,
    operation: &str,
    outcome: &str,
    exit_code: i32,
    inputs: Vec<driftlock_store::Artifact>,
    outputs: Vec<driftlock_store::Artifact>,
    created_by: &str,
) -> CliResult<Receipt> {
    let timestamp = chrono::Utc::now().to_rfc3339();
    let row = append_audit(
        &paths.repo_root,
        &timestamp,
        &AuditEntry {
            operation: operation.to_string(),
            outcome: outcome.to_string(),
            exit_code,
            receipt_path: None,
            receipt_blake3: None,
        },
    )
    .map_err(|e| CliError::preflight(format!("append audit trail: {e}")))?;

    let link = AuditLink {
        trail_path: driftlock_store::TRAIL_FILENAME.to_string(),
        seq: row.seq,
        row_hash: row.row_hash.clone(),
    };
    let receipt = build_receipt(
        &paths.repo_root,
        ReceiptInput {
            operation: operation.to_string(),
            outcome: outcome.to_string(),
            exit_code,
            inputs,
            outputs,
            audit_chain: Some(link),
            created_by: created_by.to_string(),
        },
    )
    .map_err(|e| CliError::preflight(format!("build receipt: {e}")))?;

    write_receipt(&paths.state_dir, row.seq, operation, &receipt)?;
    Ok(receipt)
}

/// Snapshot a PASSING diff-verification as the work order's last-verified
/// checkpoint under `.driftlock/checkpoints/<task>.json`.
///
/// Called only when `report.allowed` is true, so a checkpoint is never written
/// for work that did not clear the boundary + gate checks. The snapshot records
/// the verified files (content-addressed from disk) and a compact gate summary so
/// a later [`resume`](crate) can determine which files are still intact and skip
/// re-verifying them — resume-from-last-verified-state, not reconstruction.
///
/// A checkpoint write failure is surfaced as a preflight error (exit 3): a
/// silently-dropped checkpoint would leave a resumer with stale state.
pub fn snapshot_checkpoint(
    paths: &StatePaths,
    task_id: &str,
    base_ref: &str,
    report: &DiffReport,
) -> CliResult<()> {
    debug_assert!(report.allowed, "checkpoint must only be written from a passing verification");
    let timestamp = chrono::Utc::now().to_rfc3339();
    let gate_summary: BTreeMap<String, String> = report
        .gate_results
        .iter()
        .map(|g| (format!("{}:{}", g.kind, g.subject), format!("{:?}", g.status)))
        .collect();
    let checkpoint = build_checkpoint(
        &paths.repo_root,
        task_id,
        base_ref,
        &timestamp,
        &report.touched_files,
        gate_summary,
    );
    save_checkpoint(paths, &checkpoint)
        .map_err(|e| CliError::preflight(format!("save checkpoint: {e}")))?;
    Ok(())
}

/// Write a receipt under `.driftlock/receipts/`.
fn write_receipt(state_dir: &Path, seq: u64, operation: &str, receipt: &Receipt) -> CliResult<()> {
    let dir = state_dir.join("receipts");
    std::fs::create_dir_all(&dir).map_err(io_err("create receipts dir"))?;
    let safe_op = operation.replace(|c: char| !c.is_ascii_alphanumeric(), "-");
    let file = dir.join(format!("{seq:08}-{safe_op}.json"));
    let json =
        receipt.to_json().map_err(|e| CliError::preflight(format!("serialize receipt: {e}")))?;
    std::fs::write(&file, json).map_err(io_err("write receipt"))?;
    Ok(())
}
