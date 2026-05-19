//! Claim records in `claims.jsonl`.

use crate::events::{append_event, EventKind};
use crate::paths::StatePaths;
use anyhow::{bail, Context, Result};
use chrono::Utc;
use driftlock_core::{Claim, ClaimStatus};
use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

/// Loads all claim records (latest per task wins for active).
pub fn load_claims(paths: &StatePaths) -> Result<Vec<Claim>> {
    let path = crate::paths::claims_path(paths);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let file = File::open(&path)?;
    let mut claims = Vec::new();
    for line in BufReader::new(file).lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        claims.push(serde_json::from_str(&line)?);
    }
    Ok(claims)
}

/// Returns active claim for a task, if any.
pub fn active_claim_for_task(paths: &StatePaths, task_id: &str) -> Result<Option<Claim>> {
    Ok(load_claims(paths)?
        .into_iter()
        .rev()
        .find(|c| c.task == task_id && c.status == ClaimStatus::Active))
}

/// Records a new active claim.
pub fn record_claim(paths: &StatePaths, claim: &Claim, actor: &str) -> Result<()> {
    if active_claim_for_task(paths, &claim.task)?.is_some() {
        bail!("task {} already has an active claim", claim.task);
    }
    append_line(&crate::paths::claims_path(paths), claim)?;
    append_event(paths, EventKind::TaskClaimed, actor, Some(&claim.task), BTreeMap::new())?;
    Ok(())
}

/// Releases an active claim.
pub fn release_claim(paths: &StatePaths, task_id: &str, actor: &str) -> Result<()> {
    let Some(mut claim) = active_claim_for_task(paths, task_id)? else {
        bail!("no active claim for task {task_id}");
    };
    claim.status = ClaimStatus::Released;
    append_line(&crate::paths::claims_path(paths), &claim)?;
    append_event(paths, EventKind::TaskReleased, actor, Some(task_id), BTreeMap::new())?;
    Ok(())
}

/// Marks claim completed.
pub fn complete_claim(paths: &StatePaths, task_id: &str, actor: &str) -> Result<()> {
    let Some(mut claim) = active_claim_for_task(paths, task_id)? else {
        bail!("no active claim for task {task_id}");
    };
    claim.status = ClaimStatus::Completed;
    append_line(&crate::paths::claims_path(paths), &claim)?;
    append_event(paths, EventKind::TaskCompleted, actor, Some(task_id), BTreeMap::new())?;
    Ok(())
}

/// Builds a new claim for a task.
pub fn new_claim(
    task_id: impl Into<String>,
    agent: impl Into<String>,
    base_ref: impl Into<String>,
    write_set: Vec<String>,
) -> Claim {
    Claim {
        task: task_id.into(),
        agent: agent.into(),
        claimed_at: Utc::now().to_rfc3339(),
        base_ref: base_ref.into(),
        write_set,
        status: ClaimStatus::Active,
        expires_at: None,
    }
}

fn append_line(path: &Path, claim: &Claim) -> Result<()> {
    let json = serde_json::to_string(claim)?;
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(file, "{json}").context("append claim")
}
