//! Deterministic acceptance-gate evaluation.
//!
//! Driftlock evaluates only the gates it can verify offline and deterministically:
//! [`AcceptanceGate::FileExists`] and [`AcceptanceGate::FileContains`]. Every
//! other gate kind is reported honestly as [`GateStatus::Unverified`] so the
//! completion contract never over-claims.
//!
//! Boundary: Driftlock is not an execution sandbox. A [`AcceptanceGate::Command`]
//! is a typed obligation that Driftlock surfaces for a delegating runner
//! (corcept Stop-gate, CI, or an explicit `--allow-exec` path that hands
//! isolation to cellos). This module never spawns a process.
//!
//! Fail-closed: any path that escapes the repo root, any unreadable file, and
//! any unverifiable gate all resolve to a non-passing status.

use crate::model::{AcceptanceGate, GateResult, GateStatus};
use std::path::{Component, Path, PathBuf};

/// Outcome of evaluating a work order's acceptance gates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcceptanceOutcome {
    /// Per-gate results in input order.
    pub results: Vec<GateResult>,
}

impl AcceptanceOutcome {
    /// Whether every deterministic gate passed.
    ///
    /// Advisory and surfaced-only command gates are `Unverified` and are NOT
    /// counted as blocking here; they are reported, not enforced by Driftlock.
    /// A failed deterministic gate makes this false (fails closed).
    #[must_use]
    pub fn deterministic_gates_passed(&self) -> bool {
        !self.results.iter().any(|r| r.status == GateStatus::Fail)
    }

    /// Weak-model-legible one-line decision over the deterministic gates.
    ///
    /// Plain `DECISION:` prefix, no inverted convention: a human or a weak
    /// model can read the verdict without knowing a boolean polarity rule.
    #[must_use]
    pub fn decision_line(&self) -> String {
        let failed = self.results.iter().filter(|r| r.status == GateStatus::Fail).count();
        let unverified = self.results.iter().filter(|r| r.status == GateStatus::Unverified).count();
        if failed > 0 {
            format!("DECISION: BLOCKED — {failed} acceptance gate(s) FAILED")
        } else if unverified > 0 {
            format!(
                "DECISION: PASSED (verified gates) — {unverified} gate(s) ADVISORY/UNVERIFIED, not enforced by driftlock"
            )
        } else {
            "DECISION: PASSED — all acceptance gates verified".to_string()
        }
    }
}

/// Evaluates acceptance gates against the repo root.
///
/// `allow_exec` opts into running [`AcceptanceGate::Command`] gates. Driftlock
/// itself still does not execute them in core; when `allow_exec` is false (the
/// default) commands are surfaced as [`GateStatus::Unverified`] obligations.
/// When `allow_exec` is true the command is reported as a delegated obligation
/// the caller must run, NOT silently executed here — core stays process-free.
#[must_use]
pub fn evaluate_gates(
    gates: &[AcceptanceGate],
    repo_root: &Path,
    allow_exec: bool,
) -> AcceptanceOutcome {
    let results = gates.iter().map(|g| evaluate_one(g, repo_root, allow_exec)).collect();
    AcceptanceOutcome { results }
}

fn evaluate_one(gate: &AcceptanceGate, repo_root: &Path, allow_exec: bool) -> GateResult {
    match gate {
        AcceptanceGate::FileExists { file_exists } => {
            let kind = "file_exists".to_string();
            match contain_to_repo(repo_root, file_exists) {
                Some(resolved) if resolved.exists() => GateResult {
                    kind,
                    subject: file_exists.clone(),
                    status: GateStatus::Pass,
                    detail: format!("path `{file_exists}` exists"),
                },
                Some(_) => GateResult {
                    kind,
                    subject: file_exists.clone(),
                    status: GateStatus::Fail,
                    detail: format!("path `{file_exists}` does not exist"),
                },
                None => GateResult {
                    kind,
                    subject: file_exists.clone(),
                    status: GateStatus::Fail,
                    detail: format!("path `{file_exists}` escapes the repo root (rejected)"),
                },
            }
        }
        AcceptanceGate::FileContains { file_contains, needle } => {
            let kind = "file_contains".to_string();
            match contain_to_repo(repo_root, file_contains) {
                None => GateResult {
                    kind,
                    subject: file_contains.clone(),
                    status: GateStatus::Fail,
                    detail: format!("path `{file_contains}` escapes the repo root (rejected)"),
                },
                Some(resolved) => match std::fs::read_to_string(&resolved) {
                    Ok(body) if body.contains(needle) => GateResult {
                        kind,
                        subject: file_contains.clone(),
                        status: GateStatus::Pass,
                        detail: format!("`{file_contains}` contains expected needle"),
                    },
                    Ok(_) => GateResult {
                        kind,
                        subject: file_contains.clone(),
                        status: GateStatus::Fail,
                        detail: format!("`{file_contains}` does not contain expected needle"),
                    },
                    Err(err) => GateResult {
                        kind,
                        subject: file_contains.clone(),
                        status: GateStatus::Fail,
                        detail: format!("`{file_contains}` unreadable: {err}"),
                    },
                },
            }
        }
        AcceptanceGate::Command { command } => GateResult {
            kind: "command".to_string(),
            subject: command.clone(),
            status: GateStatus::Unverified,
            detail: if allow_exec {
                "command obligation delegated to --allow-exec runner; driftlock does not execute it"
                    .to_string()
            } else {
                "command obligation surfaced; driftlock does not execute (run via CI/corcept/--allow-exec)"
                    .to_string()
            },
        },
        AcceptanceGate::Advisory(text) => GateResult {
            kind: "advisory".to_string(),
            subject: text.clone(),
            status: GateStatus::Unverified,
            detail: "advisory, unverified — human-checked, not enforced by driftlock".to_string(),
        },
    }
}

/// Resolves a repo-relative gate path and confirms it stays under the root.
///
/// Returns `None` when the path is absolute or normalizes to escape the root
/// (e.g. `..` traversal). This is a lexical containment check that does not
/// require the path to exist, so a missing file still reports `Fail` (not an
/// escape) while a `../secret` style path is rejected.
fn contain_to_repo(repo_root: &Path, rel: &str) -> Option<PathBuf> {
    let candidate = Path::new(rel);
    if candidate.is_absolute() {
        return None;
    }
    let mut depth: i32 = 0;
    for comp in candidate.components() {
        match comp {
            Component::CurDir => {}
            Component::Normal(_) => depth += 1,
            Component::ParentDir => {
                depth -= 1;
                if depth < 0 {
                    return None;
                }
            }
            // Absolute roots / prefixes (e.g. drive letters) escape containment.
            Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    Some(repo_root.join(candidate))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn write(dir: &Path, rel: &str, body: &str) {
        let p = dir.join(rel);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(p, body).unwrap();
    }

    #[test]
    fn file_exists_pass_and_fail() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "src/lib.rs", "fn main() {}");
        let gates = vec![
            AcceptanceGate::FileExists { file_exists: "src/lib.rs".into() },
            AcceptanceGate::FileExists { file_exists: "src/missing.rs".into() },
        ];
        let outcome = evaluate_gates(&gates, dir.path(), false);
        assert_eq!(outcome.results[0].status, GateStatus::Pass);
        assert_eq!(outcome.results[1].status, GateStatus::Fail);
        assert!(!outcome.deterministic_gates_passed());
    }

    #[test]
    fn file_contains_pass_and_fail() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "CHANGELOG.md", "## 0.2.0\nAdded acceptance gates.\n");
        let pass = vec![AcceptanceGate::FileContains {
            file_contains: "CHANGELOG.md".into(),
            needle: "acceptance gates".into(),
        }];
        let fail = vec![AcceptanceGate::FileContains {
            file_contains: "CHANGELOG.md".into(),
            needle: "not present".into(),
        }];
        assert_eq!(evaluate_gates(&pass, dir.path(), false).results[0].status, GateStatus::Pass);
        assert_eq!(evaluate_gates(&fail, dir.path(), false).results[0].status, GateStatus::Fail);
    }

    #[test]
    fn missing_file_contains_fails_closed() {
        let dir = tempfile::tempdir().unwrap();
        let gates = vec![AcceptanceGate::FileContains {
            file_contains: "nope.md".into(),
            needle: "x".into(),
        }];
        let outcome = evaluate_gates(&gates, dir.path(), false);
        assert_eq!(outcome.results[0].status, GateStatus::Fail);
    }

    #[test]
    fn path_traversal_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let gates = vec![
            AcceptanceGate::FileExists { file_exists: "../escape.txt".into() },
            AcceptanceGate::FileContains {
                file_contains: "../../secret".into(),
                needle: "x".into(),
            },
        ];
        let outcome = evaluate_gates(&gates, dir.path(), false);
        assert_eq!(outcome.results[0].status, GateStatus::Fail);
        assert!(outcome.results[0].detail.contains("escapes the repo root"));
        assert_eq!(outcome.results[1].status, GateStatus::Fail);
    }

    #[test]
    fn command_is_surfaced_not_executed() {
        let dir = tempfile::tempdir().unwrap();
        let gates = vec![AcceptanceGate::Command { command: "cargo test".into() }];
        let outcome = evaluate_gates(&gates, dir.path(), false);
        assert_eq!(outcome.results[0].status, GateStatus::Unverified);
        assert!(outcome.results[0].detail.contains("does not execute"));
        // Surfaced commands are not failures: they do not block on their own.
        assert!(outcome.deterministic_gates_passed());
    }

    #[test]
    fn advisory_is_unverified() {
        let dir = tempfile::tempdir().unwrap();
        let gates = vec![AcceptanceGate::Advisory("Run the acceptance gates.".into())];
        let outcome = evaluate_gates(&gates, dir.path(), false);
        assert_eq!(outcome.results[0].status, GateStatus::Unverified);
        assert_eq!(outcome.results[0].kind, "advisory");
        assert!(outcome.results[0].detail.contains("advisory, unverified"));
    }

    #[test]
    fn decision_line_is_weak_model_legible() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "a.txt", "ok");
        // All pass.
        let pass = evaluate_gates(
            &[AcceptanceGate::FileExists { file_exists: "a.txt".into() }],
            dir.path(),
            false,
        );
        assert!(pass.decision_line().starts_with("DECISION: PASSED"));
        // A failure blocks.
        let blocked = evaluate_gates(
            &[AcceptanceGate::FileExists { file_exists: "missing.txt".into() }],
            dir.path(),
            false,
        );
        assert!(blocked.decision_line().starts_with("DECISION: BLOCKED"));
        // Advisory-only is passed-with-unverified, not blocked.
        let advisory = evaluate_gates(&[AcceptanceGate::Advisory("x".into())], dir.path(), false);
        let line = advisory.decision_line();
        assert!(line.starts_with("DECISION: PASSED"));
        assert!(line.contains("UNVERIFIED"));
    }
}
