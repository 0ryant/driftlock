//! Append-only audit events (JSONL).

use crate::paths::StatePaths;
use crate::signing::{load_active_signing_key, sign_event_line};
use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs::OpenOptions;
use std::io::Write;

/// Known event kinds (`CloudEvents` `type` values).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    /// Graph written or rebuilt.
    GraphBuilt,
    /// Task claimed.
    TaskClaimed,
    /// Claim released.
    TaskReleased,
    /// Task completed.
    TaskCompleted,
    /// Conflicts computed.
    ConflictDetected,
}

impl EventKind {
    /// `CloudEvents` type string.
    pub fn type_id(self) -> &'static str {
        match self {
            Self::GraphBuilt => "dev.driftlock.graph.built.v1",
            Self::TaskClaimed => "dev.driftlock.task.claimed.v1",
            Self::TaskReleased => "dev.driftlock.task.released.v1",
            Self::TaskCompleted => "dev.driftlock.task.completed.v1",
            Self::ConflictDetected => "dev.driftlock.conflict.detected.v1",
        }
    }
}

/// Ecosystem provenance extensions (seam-freeze v1).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Provenance {
    /// Correlation id for cross-tool joins.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correlationid: Option<String>,
    /// Repository identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provenancerepo: Option<String>,
    /// Producer name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provenanceproducer: Option<String>,
    /// Producer version.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provenanceversion: Option<String>,
    /// Kind of provenance record.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provenancekind: Option<String>,
}

/// One audit event line.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftlockEvent {
    /// `CloudEvents` type.
    pub event: String,
    /// RFC3339 timestamp.
    pub at: String,
    /// Actor (agent, user, cli).
    pub actor: String,
    /// Related task id, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task: Option<String>,
    /// Extra payload.
    #[serde(default)]
    pub metadata: BTreeMap<String, Value>,
}

/// Reads provenance from environment (`DRIFTLOCK_*`).
pub fn provenance_from_env() -> Provenance {
    Provenance {
        correlationid: std::env::var("DRIFTLOCK_CORRELATION_ID").ok(),
        provenancerepo: std::env::var("DRIFTLOCK_PROVENANCE_REPO").ok(),
        provenanceproducer: std::env::var("DRIFTLOCK_PROVENANCE_PRODUCER")
            .ok()
            .or_else(|| Some("driftlock".into())),
        provenanceversion: std::env::var("DRIFTLOCK_PROVENANCE_VERSION").ok(),
        provenancekind: std::env::var("DRIFTLOCK_PROVENANCE_KIND").ok(),
    }
}

/// Appends one redacted event to `events.jsonl`.
pub fn append_event(
    paths: &StatePaths,
    kind: EventKind,
    actor: &str,
    task: Option<&str>,
    metadata: BTreeMap<String, Value>,
) -> Result<()> {
    let mut meta = metadata;
    let prov = provenance_from_env();
    if let Some(v) = prov.correlationid {
        meta.insert("correlationid".into(), Value::String(redact_secrets(&v)));
    }
    if let Some(v) = prov.provenancerepo {
        meta.insert("provenancerepo".into(), Value::String(v));
    }
    if let Some(v) = prov.provenanceproducer {
        meta.insert("provenanceproducer".into(), Value::String(v));
    }
    if let Some(v) = prov.provenanceversion {
        meta.insert("provenanceversion".into(), Value::String(v));
    }
    if let Some(v) = prov.provenancekind {
        meta.insert("provenancekind".into(), Value::String(v));
    }

    let line = DriftlockEvent {
        event: kind.type_id().to_string(),
        at: Utc::now().to_rfc3339(),
        actor: actor.to_string(),
        task: task.map(str::to_string),
        metadata: meta,
    };
    let path = crate::paths::events_path(paths);
    let json = if let Some(key) = load_active_signing_key(&paths.repo_root)? {
        serde_json::to_string(&sign_event_line(&line, &key)?)?
    } else {
        serde_json::to_string(&line)?
    };
    let mut file = OpenOptions::new().create(true).append(true).open(&path)?;
    writeln!(file, "{json}")?;
    Ok(())
}

fn redact_secrets(input: &str) -> String {
    input.replace("Bearer ", "Bearer [REDACTED] ").replace("sk-", "sk-[REDACTED]")
}
