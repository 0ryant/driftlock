//! Paths under `.driftlock/`.

use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// Resolved paths for Driftlock runtime state.
#[derive(Debug, Clone)]
pub struct StatePaths {
    /// Repository root.
    pub repo_root: PathBuf,
    /// `.driftlock` directory.
    pub state_dir: PathBuf,
}

/// Creates `.driftlock/` if missing.
pub fn init_state_dir(repo_root: impl AsRef<Path>) -> Result<StatePaths> {
    let repo_root = repo_root.as_ref().canonicalize().context("canonicalize repo root")?;
    let state_dir = repo_root.join(".driftlock");
    fs::create_dir_all(&state_dir).with_context(|| format!("create {}", state_dir.display()))?;
    Ok(StatePaths { repo_root, state_dir })
}

/// Returns path to `graph.json`.
pub fn graph_path(paths: &StatePaths) -> PathBuf {
    paths.state_dir.join("graph.json")
}

/// Returns path to `events.jsonl`.
pub fn events_path(paths: &StatePaths) -> PathBuf {
    paths.state_dir.join("events.jsonl")
}

/// Returns path to `claims.jsonl`.
pub fn claims_path(paths: &StatePaths) -> PathBuf {
    paths.state_dir.join("claims.jsonl")
}
