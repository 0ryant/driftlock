//! Diff verification.

use crate::model::{DiffReport, DiffViolation, WorkOrder};
use globset::{Glob, GlobSetBuilder};

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
            warnings.push(format!(
                "write-set pattern failed to compile and was skipped: {pattern}"
            ));
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
    }
}

fn glob_escape_literal(path: &str) -> String {
    path.replace('[', "[[]").replace(']', "[]]")
}

#[cfg(test)]
mod tests {
    use super::verify_changed_files;
    use crate::model::*;
    use std::collections::BTreeMap;

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
}
