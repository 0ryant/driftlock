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
        meta.insert("correlationid".into(), Value::String(v));
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

    // Redact secrets across every metadata value, not just the correlation id.
    // Provenance env vars and arbitrary caller metadata both land in the
    // persistent, shareable ledger, so redaction must cover all string fields.
    for value in meta.values_mut() {
        redact_value(value);
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

/// Recursively redacts secret-shaped substrings in any JSON value.
fn redact_value(value: &mut Value) {
    match value {
        Value::String(s) => *s = redact_secrets(s),
        Value::Array(items) => items.iter_mut().for_each(redact_value),
        Value::Object(map) => map.values_mut().for_each(redact_value),
        _ => {}
    }
}

/// Best-effort redaction of common credential formats.
///
/// This is heuristic, not exhaustive: callers must still avoid placing secrets
/// in metadata. It covers the formats most likely to appear in provenance and
/// correlation fields (bearer/JWT, OpenAI-style `sk-`, AWS access keys,
/// basic-auth in URLs, and generic key=value secret assignments).
fn redact_secrets(input: &str) -> String {
    use std::sync::OnceLock;

    use regex::Regex;

    static PATTERNS: OnceLock<Vec<Regex>> = OnceLock::new();
    let patterns = PATTERNS.get_or_init(|| {
        [
            // Bearer / authorization tokens.
            r"(?i)bearer\s+[A-Za-z0-9._\-+/=]+",
            // JWTs (three base64url segments).
            r"\beyJ[A-Za-z0-9_\-]+\.[A-Za-z0-9_\-]+\.[A-Za-z0-9_\-]+\b",
            // OpenAI-style and similar prefixed secret keys.
            r"\b(?:sk|pk|rk|api|key|tok|ghp|gho|github_pat)[-_][A-Za-z0-9_\-]{8,}\b",
            // AWS access key ids.
            r"\b(?:AKIA|ASIA)[A-Z0-9]{16}\b",
            // basic-auth credentials embedded in URLs.
            r"://[^/\s:@]+:[^/\s@]+@",
            // generic secret/password/token assignments.
            r"(?i)\b(?:secret|password|passwd|token|api[_-]?key)\b\s*[=:]\s*\S+",
        ]
        .iter()
        .filter_map(|p| Regex::new(p).ok())
        .collect()
    });

    let mut out = input.to_string();
    for re in patterns {
        out = re.replace_all(&out, "[REDACTED]").into_owned();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::redact_secrets;

    #[test]
    fn redacts_common_credential_formats() {
        assert!(!redact_secrets("Bearer abc123XYZ").contains("abc123XYZ"));
        assert!(!redact_secrets("sk-livesecret12345678").contains("livesecret"));
        assert!(!redact_secrets("AKIAIOSFODNN7EXAMPLE").contains("AKIAIOSFODNN7EXAMPLE"));
        assert!(!redact_secrets("https://user:hunter2@host/x").contains("hunter2"));
        assert!(!redact_secrets("password=topsecret").contains("topsecret"));
        let jwt = "eyJhbGciOi.eyJzdWIiOi.SflKxwRJSM";
        assert!(!redact_secrets(jwt).contains("SflKxwRJSM"));
    }

    #[test]
    fn leaves_benign_text_alone() {
        assert_eq!(redact_secrets("ordinary repo name"), "ordinary repo name");
    }
}
