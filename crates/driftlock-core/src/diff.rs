//! Diff verification.

use crate::acceptance::evaluate_gates;
use crate::model::{DiffReport, DiffViolation, WorkOrder};
use globset::{Glob, GlobSetBuilder};
use std::path::Path;

/// Verifies touched files against the work order write set.
///
/// An empty `touched_files` set is treated as a verification failure: completing
/// a task with zero evidence of an in-scope change must never pass. Malformed
/// write-set patterns are surfaced in `DiffReport.warnings` rather than silently
/// dropped.
pub fn verify_changed_files(task: &WorkOrder, touched_files: &[String]) -> DiffReport {
    let mut builder = GlobSetBuilder::new();
    let mut warnings = Vec::new();
    for pattern in &task.write_set {
        if let Ok(glob) = Glob::new(pattern) {
            builder.add(glob);
        } else if let Ok(glob) = Glob::new(&glob_escape_literal(pattern)) {
            builder.add(glob);
        } else {
            warnings
                .push(format!("write-set pattern failed to compile and was skipped: {pattern}"));
        }
    }
    let set = builder.build().ok();

    let mut violations = Vec::new();
    for file in touched_files {
        let allowed = set.as_ref().is_some_and(|s| s.is_match(file));
        if !allowed {
            violations.push(DiffViolation {
                path: file.clone(),
                reason: "Path is outside the work order write set".to_string(),
            });
        }
    }

    // Fail closed when there is no evidence of any change. Verifying an empty
    // diff would otherwise vacuously "pass" and let a task be marked complete
    // without any in-scope work being done.
    let has_evidence = !touched_files.is_empty();
    if !has_evidence {
        violations.push(DiffViolation {
            path: String::new(),
            reason: "No changed files provided; cannot verify task completion".to_string(),
        });
    }

    DiffReport {
        task_id: task.id.clone(),
        allowed: violations.is_empty() && has_evidence,
        touched_files: touched_files.to_vec(),
        violations,
        warnings,
        gate_results: Vec::new(),
    }
}

/// Verifies the write-set boundary AND evaluates deterministic acceptance gates.
///
/// This is the completion-path entry point: it folds the gate results into the
/// returned [`DiffReport`] and tightens `allowed` so that any FAILED
/// deterministic gate ([`crate::model::AcceptanceGate::FileExists`] /
/// [`crate::model::AcceptanceGate::FileContains`]) blocks completion, in
/// addition to the existing write-set and empty-diff checks. Advisory and
/// surfaced-only command gates are reported but never silently block; they are
/// honestly marked unverified.
///
/// `repo_root` roots the deterministic file checks; `allow_exec` only changes
/// how command obligations are described (driftlock-core never spawns a
/// process — see [`crate::acceptance`]).
pub fn verify_changed_files_with_gates(
    task: &WorkOrder,
    touched_files: &[String],
    repo_root: &Path,
    allow_exec: bool,
) -> DiffReport {
    let mut report = verify_changed_files(task, touched_files);
    let outcome = evaluate_gates(&task.acceptance, repo_root, allow_exec);
    // Fail closed: a failed deterministic gate blocks completion even if the
    // write-set check passed.
    report.allowed = report.allowed && outcome.deterministic_gates_passed();
    report.gate_results = outcome.results;
    report
}

fn glob_escape_literal(path: &str) -> String {
    path.replace('[', "[[]").replace(']', "[]]")
}

#[cfg(test)]
mod tests {
    use super::{verify_changed_files, verify_changed_files_with_gates};
    use crate::model::*;
    use std::collections::BTreeMap;
    use std::fs;

    #[test]
    fn rejects_out_of_scope_file() {
        let task = WorkOrder {
            id: "adr-0001:T01".into(),
            title: "t".into(),
            source: EvidenceSpan {
                adr: "a".into(),
                adr_revision: "r".into(),
                section: "s".into(),
                start_line: 1,
                end_line: 1,
                evidence: None,
            },
            intent: "i".into(),
            lane: "core".into(),
            status: TaskStatus::Ready,
            write_set: vec!["src/**".into()],
            read_set: vec![],
            exclusive_resources: vec![],
            deps: vec![],
            unlocks: vec![],
            conflicts: vec![],
            acceptance: vec![],
            non_goals: vec![],
            confidence: Confidence::high(),
            metadata: BTreeMap::new(),
        };
        let report = verify_changed_files(&task, &["docs/readme.md".into()]);
        assert!(!report.allowed);
    }

    fn sample_task(write_set: Vec<String>) -> WorkOrder {
        WorkOrder {
            id: "adr-0001:T01".into(),
            title: "t".into(),
            source: EvidenceSpan {
                adr: "a".into(),
                adr_revision: "r".into(),
                section: "s".into(),
                start_line: 1,
                end_line: 1,
                evidence: None,
            },
            intent: "i".into(),
            lane: "core".into(),
            status: TaskStatus::Ready,
            write_set,
            read_set: vec![],
            exclusive_resources: vec![],
            deps: vec![],
            unlocks: vec![],
            conflicts: vec![],
            acceptance: vec![],
            non_goals: vec![],
            confidence: Confidence::high(),
            metadata: BTreeMap::new(),
        }
    }

    #[test]
    fn empty_touched_files_fails_closed() {
        let task = sample_task(vec!["src/**".into()]);
        let report = verify_changed_files(&task, &[]);
        assert!(!report.allowed, "empty diff must not pass verification");
        assert!(!report.violations.is_empty());
    }

    #[test]
    fn in_scope_file_is_allowed() {
        let task = sample_task(vec!["src/**".into()]);
        let report = verify_changed_files(&task, &["src/lib.rs".into()]);
        assert!(report.allowed);
    }

    #[test]
    fn gate_results_empty_by_default() {
        let task = sample_task(vec!["src/**".into()]);
        let report = verify_changed_files(&task, &["src/lib.rs".into()]);
        assert!(report.gate_results.is_empty());
    }

    #[test]
    fn passing_file_exists_gate_allows_completion() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("src")).unwrap();
        fs::write(dir.path().join("src/lib.rs"), "x").unwrap();
        let mut task = sample_task(vec!["src/**".into()]);
        task.acceptance = vec![AcceptanceGate::FileExists { file_exists: "src/lib.rs".into() }];
        let report =
            verify_changed_files_with_gates(&task, &["src/lib.rs".into()], dir.path(), false);
        assert!(report.allowed);
        assert_eq!(report.gate_results.len(), 1);
        assert_eq!(report.gate_results[0].status, GateStatus::Pass);
    }

    #[test]
    fn failing_gate_blocks_otherwise_clean_diff() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("src")).unwrap();
        fs::write(dir.path().join("src/lib.rs"), "x").unwrap();
        let mut task = sample_task(vec!["src/**".into()]);
        // Write-set check passes (src/lib.rs is in scope) but the required
        // CHANGELOG entry is missing -> completion must fail closed.
        task.acceptance = vec![AcceptanceGate::FileContains {
            file_contains: "src/lib.rs".into(),
            needle: "ABSENT-MARKER".into(),
        }];
        let clean = verify_changed_files(&task, &["src/lib.rs".into()]);
        assert!(clean.allowed, "precondition: write-set check alone passes");
        let report =
            verify_changed_files_with_gates(&task, &["src/lib.rs".into()], dir.path(), false);
        assert!(!report.allowed, "failed acceptance gate must block completion");
        assert_eq!(report.gate_results[0].status, GateStatus::Fail);
    }

    #[test]
    fn advisory_string_gate_back_compat_does_not_block() {
        // Legacy Vec<String> acceptance entries deserialize to Advisory and are
        // surfaced as unverified, never blocking on their own.
        let json = r#"["cargo test -p driftlock-core"]"#;
        let acceptance: Vec<AcceptanceGate> = serde_json::from_str(json).unwrap();
        assert_eq!(acceptance[0], AcceptanceGate::Advisory("cargo test -p driftlock-core".into()));

        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("src")).unwrap();
        fs::write(dir.path().join("src/lib.rs"), "x").unwrap();
        let mut task = sample_task(vec!["src/**".into()]);
        task.acceptance = acceptance;
        let report =
            verify_changed_files_with_gates(&task, &["src/lib.rs".into()], dir.path(), false);
        assert!(report.allowed, "advisory gate must not block completion");
        assert_eq!(report.gate_results[0].status, GateStatus::Unverified);
    }
}
