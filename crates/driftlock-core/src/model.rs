//! Public contract types for Driftlock.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Current schema version emitted by this crate.
pub const SCHEMA_VERSION: &str = "0.1.0";

/// Current skill pack version paired with this contract version.
pub const SKILL_PACK_VERSION: &str = "0.1.0";

/// Evidence span tying a work order back to ADR source text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct EvidenceSpan {
    /// ADR path.
    pub adr: String,
    /// ADR revision, commit, or scaffold marker.
    pub adr_revision: String,
    /// ADR section title.
    pub section: String,
    /// First source line.
    pub start_line: u32,
    /// Last source line.
    pub end_line: u32,
    /// Short supporting text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence: Option<String>,
}

/// A single acceptance obligation attached to a work order.
///
/// Serialized untagged so that a bare JSON string deserializes to
/// [`AcceptanceGate::Advisory`]. This keeps every pre-existing
/// `acceptance: ["cargo test ..."]` array valid: an unstructured string is an
/// advisory, human-checked gate that Driftlock does NOT verify. Structured
/// variants ([`AcceptanceGate::FileExists`] and [`AcceptanceGate::FileContains`])
/// are deterministic, non-executing checks Driftlock evaluates itself.
/// [`AcceptanceGate::Command`] is a typed, machine-checkable obligation that
/// Driftlock surfaces but does NOT execute (Driftlock is not an execution
/// sandbox); a delegating runner (corcept Stop-gate, CI, or `--allow-exec`)
/// owns isolation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum AcceptanceGate {
    /// Deterministic check: the path must exist under the repo root.
    FileExists {
        /// Repo-relative path that must exist.
        file_exists: String,
    },
    /// Deterministic check: the file must exist and contain the needle.
    FileContains {
        /// Repo-relative path that must exist and be readable as UTF-8.
        file_contains: String,
        /// Substring that must be present in the file body.
        needle: String,
    },
    /// Typed, machine-checkable command obligation. Surfaced, never executed by
    /// Driftlock unless an explicit `--allow-exec` delegating runner opts in.
    Command {
        /// Command line a downstream runner must execute and pass.
        command: String,
    },
    /// Free-text, human-checked obligation. Advisory and unverified by
    /// Driftlock. Back-compat for the historical `Vec<String>` shape.
    Advisory(String),
}

impl AcceptanceGate {
    /// Whether Driftlock can verify this gate deterministically and offline.
    #[must_use]
    pub fn is_deterministic(&self) -> bool {
        matches!(self, AcceptanceGate::FileExists { .. } | AcceptanceGate::FileContains { .. })
    }

    /// Short, weak-model-legible label for the gate kind.
    #[must_use]
    pub fn kind_label(&self) -> &'static str {
        match self {
            AcceptanceGate::FileExists { .. } => "file_exists",
            AcceptanceGate::FileContains { .. } => "file_contains",
            AcceptanceGate::Command { .. } => "command",
            AcceptanceGate::Advisory(_) => "advisory",
        }
    }
}

/// Verdict for one evaluated [`AcceptanceGate`].
///
/// `Pass`/`Fail` are reserved for gates Driftlock actually verified. Gates it
/// cannot or will not verify are honestly reported as `Unverified` so the
/// completion contract never over-claims.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GateStatus {
    /// Driftlock evaluated the gate and it passed.
    Pass,
    /// Driftlock evaluated the gate and it failed (fails closed).
    Fail,
    /// Driftlock did not verify the gate (advisory text, or a surfaced-only
    /// command obligation). Treated as not-satisfied for blocking decisions.
    Unverified,
}

/// Result of evaluating one acceptance gate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct GateResult {
    /// Gate kind label (`file_exists`, `file_contains`, `command`, `advisory`).
    pub kind: String,
    /// The gate's primary subject (path, command, or advisory text).
    pub subject: String,
    /// Evaluation status.
    pub status: GateStatus,
    /// Human- and weak-model-legible reason for the status.
    pub detail: String,
}

/// Work order status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    /// Proposed but not canonical.
    Proposed,
    /// Needs human or maintainer review.
    NeedsReview,
    /// Safe to claim and implement.
    Ready,
    /// Claimed by an actor.
    Claimed,
    /// Complete and accepted.
    Complete,
    /// Blocked by dependency or policy.
    Blocked,
    /// Unsafe to execute.
    Unsafe,
}

/// Conflict kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ConflictKind {
    /// Same write path.
    SharedWrite,
    /// Same exclusive resource.
    ExclusiveResource,
    /// Generated artifact ownership risk.
    GeneratedArtifact,
    /// API contract coupling.
    ApiContract,
    /// Migration ordering risk.
    MigrationOrder,
    /// Shared test fixture or snapshot.
    TestFixture,
    /// Semantic coupling without same file.
    SemanticCoupling,
    /// Lane policy violation.
    LaneViolation,
    /// Scope could not be inferred.
    UnknownScope,
}

/// Conflict severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ConflictSeverity {
    /// Blocks readiness.
    Hard,
    /// Requires review but may be allowed.
    Soft,
    /// Blocks by default until classified.
    Unknown,
}

/// Conflict edge attached to a task.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Conflict {
    /// Other task ID.
    pub task: String,
    /// Conflict kind.
    pub kind: ConflictKind,
    /// Conflict severity.
    pub severity: ConflictSeverity,
    /// Human-readable reason.
    pub reason: String,
}

/// Extraction and inference confidence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Confidence {
    /// Confidence that this task follows from the ADR.
    pub task_extraction: f32,
    /// Confidence in file scope.
    pub file_scope: f32,
    /// Confidence in dependency edges.
    pub dependency_edges: f32,
}

impl Confidence {
    /// Conservative complete confidence used in deterministic tests.
    pub fn high() -> Self {
        Self { task_extraction: 0.95, file_scope: 0.90, dependency_edges: 0.85 }
    }

    /// Whether confidence is high enough for ready status.
    pub fn is_ready_grade(&self) -> bool {
        self.task_extraction >= 0.80 && self.file_scope >= 0.75 && self.dependency_edges >= 0.70
    }
}

/// Canonical bounded implementation task.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct WorkOrder {
    /// Stable ID, normally `adr-0000:T00`.
    pub id: String,
    /// Short title.
    pub title: String,
    /// ADR evidence span.
    pub source: EvidenceSpan,
    /// Implementation intent.
    pub intent: String,
    /// Lane ID.
    pub lane: String,
    /// Status.
    pub status: TaskStatus,
    /// Files/globs this task may write.
    pub write_set: Vec<String>,
    /// Files/globs this task may read as context.
    pub read_set: Vec<String>,
    /// Exclusive resources claimed by this task.
    #[serde(default)]
    pub exclusive_resources: Vec<String>,
    /// Task dependencies.
    #[serde(default)]
    pub deps: Vec<String>,
    /// Tasks unlocked by completion.
    #[serde(default)]
    pub unlocks: Vec<String>,
    /// Known conflicts.
    #[serde(default)]
    pub conflicts: Vec<Conflict>,
    /// Acceptance gates.
    #[serde(default)]
    pub acceptance: Vec<AcceptanceGate>,
    /// Explicit non-goals.
    #[serde(default)]
    pub non_goals: Vec<String>,
    /// Extraction/inference confidence.
    pub confidence: Confidence,
    /// Extensible metadata.
    #[serde(default)]
    pub metadata: BTreeMap<String, serde_json::Value>,
}

impl WorkOrder {
    /// Returns true when the task has blocking conflicts.
    pub fn has_blocking_conflict(&self) -> bool {
        self.conflicts
            .iter()
            .any(|c| matches!(c.severity, ConflictSeverity::Hard | ConflictSeverity::Unknown))
    }
}

/// Lane policy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Lane {
    /// Lane ID.
    pub id: String,
    /// Description.
    pub description: String,
    /// Write allowlist globs.
    pub write_allow: Vec<String>,
    /// Read allowlist globs.
    pub read_allow: Vec<String>,
    /// Exclusive resources.
    pub exclusive: Vec<String>,
    /// Extensible metadata.
    #[serde(default)]
    pub metadata: BTreeMap<String, serde_json::Value>,
}

/// Lane manifest.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct LaneManifest {
    /// Schema version.
    pub schema_version: String,
    /// Lanes.
    pub lanes: Vec<Lane>,
}

/// Edge kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EdgeKind {
    /// To depends on from.
    DependsOn,
    /// From unlocks to.
    Unlocks,
    /// From produces contract needed by to.
    ProducesContract,
    /// From blocks to.
    Blocks,
    /// From conflicts with to.
    ConflictsWith,
}

/// `TaskGraph` edge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct GraphEdge {
    /// Source task ID.
    pub from: String,
    /// Target task ID.
    pub to: String,
    /// Edge kind.
    pub kind: EdgeKind,
    /// Reason.
    pub reason: String,
}

/// Canonical work graph.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TaskGraph {
    /// Schema version.
    pub schema_version: String,
    /// Stable graph ID.
    pub graph_id: String,
    /// Repo root used for inference.
    pub repo_root: String,
    /// Base ref used for safety.
    pub base_ref: String,
    /// Generation timestamp.
    pub generated_at: String,
    /// Work orders.
    pub tasks: Vec<WorkOrder>,
    /// Graph edges.
    #[serde(default)]
    pub edges: Vec<GraphEdge>,
    /// Lane policies included with the graph.
    #[serde(default)]
    pub lanes: Vec<Lane>,
    /// Extensible metadata.
    #[serde(default)]
    pub metadata: BTreeMap<String, serde_json::Value>,
}

/// Diff violation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DiffViolation {
    /// Path that violates the task boundary.
    pub path: String,
    /// Reason.
    pub reason: String,
}

/// Diff verification report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DiffReport {
    /// Task ID.
    pub task_id: String,
    /// Whether the diff is allowed.
    pub allowed: bool,
    /// Touched files.
    pub touched_files: Vec<String>,
    /// Violations.
    pub violations: Vec<DiffViolation>,
    /// Warnings.
    #[serde(default)]
    pub warnings: Vec<String>,
    /// Per-gate acceptance results. Empty when no gates were evaluated.
    #[serde(default)]
    pub gate_results: Vec<GateResult>,
}

/// Claim state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Claim {
    /// Task ID.
    pub task: String,
    /// Agent or actor ID.
    pub agent: String,
    /// Claim timestamp.
    pub claimed_at: String,
    /// Base ref.
    pub base_ref: String,
    /// Claimed write set.
    pub write_set: Vec<String>,
    /// Claim status.
    pub status: ClaimStatus,
    /// Optional expiry.
    #[serde(default)]
    pub expires_at: Option<String>,
}

/// Claim status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ClaimStatus {
    /// Active claim.
    Active,
    /// Released claim.
    Released,
    /// Completed claim.
    Completed,
    /// Expired claim.
    Expired,
}
