//! Per-work-order verification checkpoints (`.driftlock/checkpoints/<task>.json`).
//!
//! A *checkpoint* snapshots the last state for which a work order PASSED
//! diff-verification (write-set boundary plus deterministic acceptance gates).
//! It records the touched files together with their BLAKE3 content digests, the
//! gate verdicts, and the base ref at verification time.
//!
//! The load-bearing property is **resume-from-last-verified-state**: when a long
//! multi-agent run dies after a work order already passed verification, a
//! restart can read the checkpoint and learn which files were last verified (and
//! whether they are still byte-identical on disk) instead of re-doing the work
//! from scratch. The checkpoint is *not* a reconstruction of work that was never
//! verified — it is only ever written from a real prior PASS, so it never
//! over-claims.
//!
//! Scope honesty: a checkpoint records that the listed files passed verification
//! at the recorded digests. It does **not** snapshot file *contents* (it is a
//! manifest, not a backup) and it does not resurrect lost work. [`resume_status`]
//! re-hashes the on-disk files so a resumer can see, per file, whether the
//! verified bytes are still present (`Intact`), changed (`Drifted`), or gone
//! (`Missing`). What a checkpoint buys is skipping re-verification of the
//! still-intact prefix, not magic recovery.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::paths::StatePaths;

/// Checkpoint schema tag. Verifiers reject anything else.
pub const CHECKPOINT_SCHEMA: &str = "dev.driftlock.checkpoint.v1";

/// One verified file recorded in a checkpoint: its repo-relative path and the
/// BLAKE3 digest it had when the work order last passed verification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerifiedFile {
    /// Repo-relative path (forward slashes).
    pub path: String,
    /// Lowercase-hex BLAKE3 of the file content at verification time.
    pub blake3: String,
}

/// One PASSING diff-verification snapshot for a single work order.
///
/// Written only from a real prior PASS (`allowed == true`), so the presence of a
/// checkpoint is itself evidence that the recorded files cleared the boundary and
/// gate checks at `verified_at`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Checkpoint {
    /// Schema tag, always [`CHECKPOINT_SCHEMA`].
    pub schema: String,
    /// Work order id this checkpoint belongs to.
    pub task_id: String,
    /// Base ref the verification ran against (stale base detection on resume).
    pub base_ref: String,
    /// RFC 3339 timestamp of the passing verification.
    pub verified_at: String,
    /// Files that were in-scope and verified, with their content digests.
    pub verified_files: Vec<VerifiedFile>,
    /// Compact record of the gate verdicts at checkpoint time
    /// (`gate kind/subject -> status`), for resume legibility.
    #[serde(default)]
    pub gate_summary: BTreeMap<String, String>,
}

/// Per-file state of a checkpointed file relative to the current on-disk repo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileState {
    /// The file exists and its BLAKE3 still matches the checkpoint digest.
    Intact,
    /// The file exists but its content changed since the checkpoint.
    Drifted,
    /// The file no longer exists on disk.
    Missing,
}

/// Per-file resume verdict.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileResume {
    /// Repo-relative path.
    pub path: String,
    /// State relative to the recorded checkpoint digest.
    pub state: FileState,
}

/// What a resumer learns from the last checkpoint: which verified files are still
/// intact (re-verification can be skipped) and which drifted or vanished (must be
/// re-done). This is the honest "resume from last verified checkpoint, not from
/// scratch" surface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResumeStatus {
    /// Work order id.
    pub task_id: String,
    /// Whether the live base ref still matches the checkpoint base ref. A stale
    /// base ref invalidates the resume (the verified files may no longer apply),
    /// mirroring readiness' stale-base-ref invalidation.
    pub base_ref_matches: bool,
    /// Checkpoint base ref.
    pub checkpoint_base_ref: String,
    /// Per-file state versus the checkpoint.
    pub files: Vec<FileResume>,
    /// Files that are still byte-identical to the verified snapshot.
    pub intact_files: usize,
    /// Files that drifted or went missing and must be re-done.
    pub stale_files: usize,
    /// True when every verified file is still intact AND the base ref matches:
    /// the run can resume without re-doing any verified work.
    pub fully_resumable: bool,
}

/// Returns `.driftlock/checkpoints/`.
pub fn checkpoints_dir(paths: &StatePaths) -> PathBuf {
    paths.state_dir.join("checkpoints")
}

/// Returns `.driftlock/checkpoints/<task>.json` with the task id sanitised for a
/// filename (the raw id, e.g. `adr-0001:T01`, contains a `:` that is invalid on
/// Windows).
pub fn checkpoint_path(paths: &StatePaths, task_id: &str) -> PathBuf {
    let safe = sanitize_task_id(task_id);
    checkpoints_dir(paths).join(format!("{safe}.json"))
}

fn sanitize_task_id(task_id: &str) -> String {
    task_id.replace(|c: char| !c.is_ascii_alphanumeric() && c != '-' && c != '_', "-")
}

/// Build a [`Checkpoint`] from a PASSING verification snapshot.
///
/// `touched_files` are the repo-relative paths the verification covered; each is
/// content-addressed (BLAKE3) from disk under `repo_root`. A file listed in the
/// diff but absent on disk is recorded with an empty digest rather than failing
/// the snapshot, so a checkpoint can still be written for a partially-flushed
/// tree (the resumer sees it as `Missing`).
pub fn build_checkpoint(
    repo_root: &Path,
    task_id: &str,
    base_ref: &str,
    verified_at: &str,
    touched_files: &[String],
    gate_summary: BTreeMap<String, String>,
) -> Checkpoint {
    let verified_files = touched_files
        .iter()
        .map(|rel| {
            let abs = repo_root.join(rel);
            let blake3 = axiom_hash::blake3_file(&abs).unwrap_or_default();
            VerifiedFile { path: rel.clone(), blake3 }
        })
        .collect();
    Checkpoint {
        schema: CHECKPOINT_SCHEMA.to_string(),
        task_id: task_id.to_string(),
        base_ref: base_ref.to_string(),
        verified_at: verified_at.to_string(),
        verified_files,
        gate_summary,
    }
}

/// Persist a checkpoint to `.driftlock/checkpoints/<task>.json` atomically.
///
/// Overwrites any prior checkpoint for the task: the checkpoint is "last verified
/// state", a single most-recent snapshot, not an append log. The append-only,
/// hash-chained audit ledger (`events.jsonl`) and the doctrine trail
/// (`audit-trail.jsonl`) remain the durable history; this file is a derived,
/// rewritable resume hint.
pub fn save_checkpoint(paths: &StatePaths, checkpoint: &Checkpoint) -> Result<PathBuf> {
    let dir = checkpoints_dir(paths);
    std::fs::create_dir_all(&dir).with_context(|| format!("create {}", dir.display()))?;
    let path = checkpoint_path(paths, &checkpoint.task_id);
    let body = serde_json::to_string_pretty(checkpoint)?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, body.as_bytes()).with_context(|| format!("write {}", tmp.display()))?;
    std::fs::rename(&tmp, &path).with_context(|| format!("rename into {}", path.display()))?;
    Ok(path)
}

/// Load the last checkpoint for a task, or `None` if none exists.
///
/// A checkpoint whose `schema` tag is unrecognised is rejected fail-closed (a
/// resumer must not trust a snapshot it cannot interpret).
pub fn load_checkpoint(paths: &StatePaths, task_id: &str) -> Result<Option<Checkpoint>> {
    let path = checkpoint_path(paths, task_id);
    if !path.exists() {
        return Ok(None);
    }
    let text =
        std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let checkpoint: Checkpoint =
        serde_json::from_str(&text).with_context(|| format!("parse {}", path.display()))?;
    if checkpoint.schema != CHECKPOINT_SCHEMA {
        anyhow::bail!("checkpoint {} has unsupported schema {}", path.display(), checkpoint.schema);
    }
    Ok(Some(checkpoint))
}

/// Compare the last checkpoint for a task against the current on-disk repo and
/// the live base ref, returning a [`ResumeStatus`].
///
/// Each verified file is re-hashed from disk: an identical digest is `Intact`, a
/// different digest is `Drifted`, an absent file is `Missing`. The base-ref
/// comparison mirrors readiness' stale-base-ref invalidation: a moved base ref
/// makes the resume not fully trustworthy even if every file is byte-identical.
pub fn resume_status(
    paths: &StatePaths,
    checkpoint: &Checkpoint,
    current_base_ref: &str,
) -> ResumeStatus {
    let mut files = Vec::with_capacity(checkpoint.verified_files.len());
    let mut intact = 0usize;
    for vf in &checkpoint.verified_files {
        let abs = paths.repo_root.join(&vf.path);
        let state = if abs.exists() {
            match axiom_hash::blake3_file(&abs) {
                Ok(digest) if digest == vf.blake3 && !vf.blake3.is_empty() => {
                    intact += 1;
                    FileState::Intact
                }
                _ => FileState::Drifted,
            }
        } else {
            FileState::Missing
        };
        files.push(FileResume { path: vf.path.clone(), state });
    }
    let stale = files.len() - intact;
    let base_ref_matches = checkpoint.base_ref == current_base_ref;
    ResumeStatus {
        task_id: checkpoint.task_id.clone(),
        base_ref_matches,
        checkpoint_base_ref: checkpoint.base_ref.clone(),
        files,
        intact_files: intact,
        stale_files: stale,
        fully_resumable: base_ref_matches && stale == 0 && intact > 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::init_state_dir;

    fn write(repo: &Path, rel: &str, body: &str) {
        let abs = repo.join(rel);
        std::fs::create_dir_all(abs.parent().unwrap()).unwrap();
        std::fs::write(abs, body).unwrap();
    }

    #[test]
    fn save_then_load_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let paths = init_state_dir(dir.path()).unwrap();
        write(&paths.repo_root, "src/lib.rs", "fn main() {}");
        let cp = build_checkpoint(
            &paths.repo_root,
            "adr-0001:T01",
            "abc123",
            "2026-06-21T00:00:00+00:00",
            &["src/lib.rs".to_string()],
            BTreeMap::new(),
        );
        save_checkpoint(&paths, &cp).unwrap();
        let loaded = load_checkpoint(&paths, "adr-0001:T01").unwrap().unwrap();
        assert_eq!(loaded, cp);
        assert_eq!(loaded.verified_files.len(), 1);
        assert!(!loaded.verified_files[0].blake3.is_empty());
    }

    #[test]
    fn absent_checkpoint_is_none() {
        let dir = tempfile::tempdir().unwrap();
        let paths = init_state_dir(dir.path()).unwrap();
        assert!(load_checkpoint(&paths, "nope").unwrap().is_none());
    }

    #[test]
    fn intact_file_is_fully_resumable() {
        let dir = tempfile::tempdir().unwrap();
        let paths = init_state_dir(dir.path()).unwrap();
        write(&paths.repo_root, "src/lib.rs", "verified body");
        let cp = build_checkpoint(
            &paths.repo_root,
            "t1",
            "base-1",
            "2026-06-21T00:00:00+00:00",
            &["src/lib.rs".to_string()],
            BTreeMap::new(),
        );
        let status = resume_status(&paths, &cp, "base-1");
        assert!(status.fully_resumable);
        assert_eq!(status.intact_files, 1);
        assert_eq!(status.stale_files, 0);
        assert_eq!(status.files[0].state, FileState::Intact);
    }

    #[test]
    fn drifted_file_is_not_fully_resumable() {
        let dir = tempfile::tempdir().unwrap();
        let paths = init_state_dir(dir.path()).unwrap();
        write(&paths.repo_root, "src/lib.rs", "verified body");
        let cp = build_checkpoint(
            &paths.repo_root,
            "t1",
            "base-1",
            "2026-06-21T00:00:00+00:00",
            &["src/lib.rs".to_string()],
            BTreeMap::new(),
        );
        // Mutate the file after the checkpoint.
        write(&paths.repo_root, "src/lib.rs", "changed body");
        let status = resume_status(&paths, &cp, "base-1");
        assert!(!status.fully_resumable);
        assert_eq!(status.stale_files, 1);
        assert_eq!(status.files[0].state, FileState::Drifted);
    }

    #[test]
    fn missing_file_is_reported_missing() {
        let dir = tempfile::tempdir().unwrap();
        let paths = init_state_dir(dir.path()).unwrap();
        write(&paths.repo_root, "src/lib.rs", "verified body");
        let cp = build_checkpoint(
            &paths.repo_root,
            "t1",
            "base-1",
            "2026-06-21T00:00:00+00:00",
            &["src/lib.rs".to_string()],
            BTreeMap::new(),
        );
        std::fs::remove_file(paths.repo_root.join("src/lib.rs")).unwrap();
        let status = resume_status(&paths, &cp, "base-1");
        assert_eq!(status.files[0].state, FileState::Missing);
        assert!(!status.fully_resumable);
    }

    #[test]
    fn stale_base_ref_blocks_full_resume_even_when_intact() {
        let dir = tempfile::tempdir().unwrap();
        let paths = init_state_dir(dir.path()).unwrap();
        write(&paths.repo_root, "src/lib.rs", "verified body");
        let cp = build_checkpoint(
            &paths.repo_root,
            "t1",
            "base-1",
            "2026-06-21T00:00:00+00:00",
            &["src/lib.rs".to_string()],
            BTreeMap::new(),
        );
        let status = resume_status(&paths, &cp, "base-2-moved");
        assert_eq!(status.intact_files, 1);
        assert!(!status.base_ref_matches);
        assert!(!status.fully_resumable, "moved base ref must block full resume");
    }

    #[test]
    fn unsupported_schema_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let paths = init_state_dir(dir.path()).unwrap();
        let dir2 = checkpoints_dir(&paths);
        std::fs::create_dir_all(&dir2).unwrap();
        std::fs::write(
            checkpoint_path(&paths, "t1"),
            r#"{"schema":"bogus.v0","task_id":"t1","base_ref":"b","verified_at":"t","verified_files":[]}"#,
        )
        .unwrap();
        assert!(load_checkpoint(&paths, "t1").is_err());
    }

    #[test]
    fn task_id_with_colon_is_filename_safe() {
        let dir = tempfile::tempdir().unwrap();
        let paths = init_state_dir(dir.path()).unwrap();
        let path = checkpoint_path(&paths, "adr-0001:T01");
        assert!(
            !path.to_string_lossy().contains(':')
                || cfg!(windows) && path.to_string_lossy().matches(':').count() == 1,
            "colon from task id must be sanitised"
        );
        // The sanitised stem must not contain the raw colon.
        assert!(path.file_name().unwrap().to_string_lossy().starts_with("adr-0001-T01"));
    }
}
