//! Shared Driftlock MCP business logic (stdio and rmcp transports).
#![allow(
    clippy::too_many_lines,
    clippy::needless_pass_by_value,
    clippy::unused_self,
    clippy::unnecessary_wraps
)]

use anyhow::{Context, Result};
use driftlock_core::{
    build_task_graph, detect_graph_conflicts, extract_work_orders_from_adr, find_task,
    ready_tasks_for_base, render_agent_brief, verify_changed_files_with_gates, LaneManifest,
    TaskGraph, TaskStatus,
};
use driftlock_store::{
    build_checkpoint, complete_claim, init_state_dir, load_checkpoint, new_claim, record_claim,
    release_claim, resume_status, save_checkpoint, save_graph, StatePaths,
};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Returns MCP-compatible structured content.
///
/// Some hosts reject array/scalar `structuredContent`; preserve JSON objects and
/// wrap everything else in a record.
pub fn tool_structured_content(value: Value) -> Value {
    match value {
        Value::Object(_) => value,
        other => json!({"result": other}),
    }
}

/// Driftlock tool/resource implementation shared across MCP transports.
#[derive(Debug, Clone)]
pub struct DriftlockService {
    /// Repository root for relative paths.
    pub repo_root: PathBuf,
}

impl DriftlockService {
    /// Creates a service bound to a repository root.
    pub fn new(repo_root: PathBuf) -> Self {
        Self { repo_root }
    }

    /// Server instructions returned at initialize.
    pub fn instructions() -> &'static str {
        "Use Driftlock work orders, not ADR prose, as delivery boundaries."
    }

    /// Executes a tool by name; returns structured JSON (not MCP envelope).
    pub fn call_tool(&self, name: &str, args: Value) -> Result<Value> {
        match name {
            "index_repo" => {
                // repo_root is agent-controlled; contain it to the bound
                // repository so it cannot be pointed at arbitrary directories.
                let repo = match args.get("repo_root").and_then(Value::as_str) {
                    Some(path) => self.resolve(path)?,
                    None => self.repo_root.clone(),
                };
                Ok(serde_json::to_value(driftlock_git::index_repo(repo)?)?)
            }
            "extract_tasks" => {
                let adr_path = required_str(&args, "adr_path")?;
                let lane = args.get("lane").and_then(Value::as_str).unwrap_or("core");
                let text = fs::read_to_string(self.resolve(adr_path)?)?;
                let lanes = default_lanes(&self.repo_root).ok();
                Ok(serde_json::to_value(extract_work_orders_from_adr(
                    adr_path,
                    "working-tree",
                    &text,
                    lane,
                    lanes.as_ref(),
                ))?)
            }
            "build_task_graph" => {
                let adr_path = required_str(&args, "adr_path")?;
                let lane = args.get("lane").and_then(Value::as_str).unwrap_or("core");
                let lane_manifest = match args.get("lane_manifest_path").and_then(Value::as_str) {
                    Some(path) => read_lanes(self.resolve(path)?)?,
                    None => default_lanes(&self.repo_root)?,
                };
                let text = fs::read_to_string(self.resolve(adr_path)?)?;
                let tasks = extract_work_orders_from_adr(
                    adr_path,
                    "working-tree",
                    &text,
                    lane,
                    Some(&lane_manifest),
                );
                Ok(serde_json::to_value(build_task_graph(
                    "mcp-generated",
                    self.repo_root.to_string_lossy().as_ref(),
                    "working-tree",
                    tasks,
                    lane_manifest,
                ))?)
            }
            "check_conflicts" => {
                let graph = self.read_graph(required_str(&args, "graph_path")?)?;
                Ok(serde_json::to_value(detect_graph_conflicts(&graph))?)
            }
            "ready_tasks" => {
                let graph = self.read_graph(required_str(&args, "graph_path")?)?;
                let lane = required_str(&args, "lane")?;
                let base = driftlock_git::current_head(&self.repo_root)
                    .unwrap_or_else(|_| graph.base_ref.clone());
                Ok(serde_json::to_value(ready_tasks_for_base(&graph, lane, &base))?)
            }
            "claim_task" => {
                let paths = init_state_dir(&self.repo_root)?;
                let graph_path = args
                    .get("graph_path")
                    .and_then(Value::as_str)
                    .unwrap_or(".driftlock/graph.json");
                let graph = self.read_graph(graph_path)?;
                let task_id = required_str(&args, "task_id")?;
                let actor = args.get("actor").and_then(Value::as_str).unwrap_or("mcp");
                let wo = find_task(&graph, task_id).context("task not found")?;
                let base = driftlock_git::current_head(&self.repo_root)
                    .unwrap_or_else(|_| graph.base_ref.clone());
                let claim = new_claim(task_id, actor, &base, wo.write_set.clone());
                record_claim(&paths, &claim, actor)?;
                Ok(serde_json::to_value(claim)?)
            }
            "release_task" => {
                let paths = init_state_dir(&self.repo_root)?;
                let task_id = required_str(&args, "task_id")?;
                let actor = args.get("actor").and_then(Value::as_str).unwrap_or("mcp");
                release_claim(&paths, task_id, actor)?;
                Ok(json!({"released": task_id}))
            }
            "complete_task" => {
                let paths = init_state_dir(&self.repo_root)?;
                let graph_path = args
                    .get("graph_path")
                    .and_then(Value::as_str)
                    .unwrap_or(".driftlock/graph.json");
                let mut graph = self.read_graph(graph_path)?;
                let task_id = required_str(&args, "task_id")?;
                let actor = args.get("actor").and_then(Value::as_str).unwrap_or("mcp");
                let wo = find_task(&graph, task_id).context("task not found")?;
                // No diff/changed_files supplied: fall back to git like the CLI
                // does, rather than verifying an empty (vacuously passing) change
                // set.
                let files = if let Some(files) = changed_files_from_args(&args)? {
                    files
                } else {
                    let base = driftlock_git::current_head(&self.repo_root)
                        .unwrap_or_else(|_| graph.base_ref.clone());
                    driftlock_git::git_changed_files(&self.repo_root, &base)
                        .context("no diff/changed_files provided and git fallback failed")?
                };
                // Evaluate deterministic acceptance gates alongside the
                // write-set check. Driftlock never executes command gates over
                // MCP (it is not an execution sandbox), so allow_exec is false:
                // commands are surfaced as obligations, not run.
                let report = verify_changed_files_with_gates(wo, &files, &self.repo_root, false);
                if !report.allowed {
                    anyhow::bail!(
                        "completion blocked: violations={:?} gate_results={:?}",
                        report.violations,
                        report.gate_results
                    );
                }
                // Snapshot the passing verification as the last-verified
                // checkpoint so a later resume_task can pick up from here rather
                // than re-verifying from scratch.
                let base = driftlock_git::current_head(&self.repo_root)
                    .unwrap_or_else(|_| graph.base_ref.clone());
                snapshot_checkpoint(&paths, task_id, &base, &report)?;
                complete_claim(&paths, task_id, actor)?;
                if let Some(t) = graph.tasks.iter_mut().find(|t| t.id == task_id) {
                    t.status = TaskStatus::Complete;
                }
                save_graph(&paths, &graph)?;
                Ok(serde_json::to_value(report)?)
            }
            "agent_brief" => {
                let graph = self.read_graph(required_str(&args, "graph_path")?)?;
                let task_id = required_str(&args, "task_id")?;
                let task = find_task(&graph, task_id).context("task not found")?;
                Ok(json!({"brief": render_agent_brief(task)}))
            }
            "verify_diff_against_task" => {
                let graph = self.read_graph(required_str(&args, "graph_path")?)?;
                let task_id = required_str(&args, "task_id")?;
                let task = find_task(&graph, task_id).context("task not found")?;
                let files = changed_files_from_args(&args)?.unwrap_or_default();
                let report = verify_changed_files_with_gates(task, &files, &self.repo_root, false);
                // A passing verification is a checkpointable last-verified state.
                if report.allowed {
                    let paths = init_state_dir(&self.repo_root)?;
                    let base = driftlock_git::current_head(&self.repo_root)
                        .unwrap_or_else(|_| graph.base_ref.clone());
                    snapshot_checkpoint(&paths, task_id, &base, &report)?;
                }
                Ok(serde_json::to_value(report)?)
            }
            "resume_task" => {
                let paths = init_state_dir(&self.repo_root)?;
                let task_id = required_str(&args, "task_id")?;
                let graph_path = args
                    .get("graph_path")
                    .and_then(Value::as_str)
                    .unwrap_or(".driftlock/graph.json");
                let base = match self.read_graph(graph_path) {
                    Ok(graph) => {
                        driftlock_git::current_head(&self.repo_root).unwrap_or(graph.base_ref)
                    }
                    Err(_) => driftlock_git::current_head(&self.repo_root)
                        .unwrap_or_else(|_| "working-tree".to_string()),
                };
                let Some(checkpoint) = load_checkpoint(&paths, task_id)? else {
                    // No verified checkpoint: honestly report there is nothing to
                    // resume from rather than fabricating a degraded state.
                    return Ok(json!({
                        "resumable": false,
                        "reason": "no verified checkpoint; run complete/verify first",
                        "task_id": task_id,
                    }));
                };
                let status = resume_status(&paths, &checkpoint, &base);
                Ok(serde_json::to_value(status)?)
            }
            "list_skills" => Ok(serde_json::to_value(
                driftlock_skills::skills()
                    .iter()
                    .map(|s| json!({"name": s.name, "uri": s.uri}))
                    .collect::<Vec<_>>(),
            )?),
            "get_skill" => {
                let name = required_str(&args, "name")?;
                let skill = driftlock_skills::find_skill(name).context("skill not found")?;
                Ok(json!({"name": skill.name, "uri": skill.uri, "body": skill.body}))
            }
            "export_schemas" => {
                let schemas = driftlock_contracts::schema_bundle()
                    .into_iter()
                    .map(|s| json!({"file_name": s.file_name, "schema": s.schema}))
                    .collect::<Vec<_>>();
                Ok(json!({"schemas": schemas}))
            }
            other => anyhow::bail!("unknown tool: {other}"),
        }
    }

    /// MCP tool list entries (JSON tool definitions).
    pub fn tool_definitions() -> Vec<Value> {
        crate::tool_contracts::tool_definitions()
    }

    /// Resource descriptors for `resources/list`.
    pub fn resources(&self) -> Vec<Value> {
        let mut resources = vec![
            json!({"uri":"driftlock://schemas/taskgraph","name":"taskgraph.schema.json","mimeType":"application/schema+json"}),
            json!({"uri":"driftlock://schemas/work-order","name":"work-order.schema.json","mimeType":"application/schema+json"}),
            json!({"uri":"driftlock://schemas/lane-manifest","name":"lane-manifest.schema.json","mimeType":"application/schema+json"}),
        ];
        resources.extend(
            driftlock_skills::skills()
                .iter()
                .map(|s| json!({"uri": s.uri, "name": s.name, "mimeType":"text/markdown"})),
        );
        resources.extend(
            driftlock_skills::prompts()
                .iter()
                .map(|p| json!({"uri": p.uri, "name": p.name, "mimeType":"text/markdown"})),
        );
        resources
    }

    /// Reads a resource by URI.
    pub fn read_resource(&self, uri: &str) -> Result<(String, &'static str)> {
        let (text, mime) = match uri {
            "driftlock://schemas/taskgraph" => (
                include_str!("../../../contracts/schemas/taskgraph.schema.json").to_string(),
                "application/schema+json",
            ),
            "driftlock://schemas/work-order" => (
                include_str!("../../../contracts/schemas/work-order.schema.json").to_string(),
                "application/schema+json",
            ),
            "driftlock://schemas/lane-manifest" => (
                include_str!("../../../contracts/schemas/lane-manifest.schema.json").to_string(),
                "application/schema+json",
            ),
            other => {
                if let Some(skill) = driftlock_skills::find_skill(other) {
                    (skill.body.to_string(), "text/markdown")
                } else if let Some(prompt) = driftlock_skills::find_prompt(other) {
                    (prompt.body.to_string(), "text/markdown")
                } else {
                    anyhow::bail!("resource not found: {other}");
                }
            }
        };
        Ok((text, mime))
    }

    /// Prompt descriptors for `prompts/list`.
    pub fn prompts(&self) -> Vec<Value> {
        driftlock_skills::prompts()
            .iter()
            .map(|p| {
                let arguments: Vec<Value> = placeholders_in(p.body)
                    .into_iter()
                    .map(|name| json!({"name": name, "required": false}))
                    .collect();
                json!({
                    "name": p.name,
                    "title": p.name,
                    "description": "Driftlock blessed workflow prompt",
                    "arguments": arguments,
                })
            })
            .collect()
    }

    /// Renders a prompt by name with optional argument substitution.
    ///
    /// Substitution is generic: every provided string argument `key` replaces
    /// `{{key}}` in the body. Any placeholder left unfilled is reported as an
    /// error rather than silently returned verbatim to the agent.
    pub fn get_prompt(
        &self,
        name: &str,
        arguments: Option<&serde_json::Map<String, Value>>,
    ) -> Result<String> {
        let prompt = driftlock_skills::find_prompt(name).context("prompt not found")?;
        let mut body = prompt.body.to_string();
        if let Some(args) = arguments {
            for (key, value) in args {
                if let Some(s) = value.as_str() {
                    body = body.replace(&format!("{{{{{key}}}}}"), s);
                }
            }
        }
        let unfilled = placeholders_in(&body);
        if !unfilled.is_empty() {
            anyhow::bail!("prompt {name} has unfilled placeholders: {unfilled:?}");
        }
        Ok(body)
    }

    /// Resolves an agent-supplied path against `repo_root`, rejecting any path
    /// that escapes the repository.
    ///
    /// MCP tool arguments are attacker-influenced (prompt injection), so this
    /// guard rejects absolute paths and `..` traversal, and verifies the
    /// resolved path stays within the canonicalized repository root. This is the
    /// single choke point for every agent-driven filesystem read.
    fn resolve(&self, path: &str) -> Result<PathBuf> {
        contained_path(&self.repo_root, path)
    }

    fn read_graph(&self, path: &str) -> Result<TaskGraph> {
        let text = fs::read_to_string(self.resolve(path)?)?;
        Ok(serde_json::from_str(&text)?)
    }
}

/// Resolves `path` under `root` and rejects anything that escapes it.
///
/// Rejects absolute paths and any `..` / `.` traversal components, joins under
/// `root`, then canonicalizes both and verifies containment so that symlinks or
/// normalization cannot be used to escape the repository.
fn contained_path(root: &Path, path: &str) -> Result<PathBuf> {
    let candidate = Path::new(path);
    if candidate.is_absolute() {
        anyhow::bail!("absolute paths are not allowed: {path}");
    }
    for component in candidate.components() {
        match component {
            std::path::Component::Normal(_) | std::path::Component::CurDir => {}
            other => anyhow::bail!("path component not allowed in {path}: {other:?}"),
        }
    }

    let root_canon = root
        .canonicalize()
        .with_context(|| format!("canonicalizing repo root {}", root.display()))?;
    let joined = root_canon.join(candidate);

    // Canonicalize the resolved target when it exists (defeats symlink escapes);
    // for not-yet-existing targets, canonicalize the parent and re-append the
    // file name, then verify containment lexically.
    let resolved = if let Ok(p) = joined.canonicalize() {
        p
    } else {
        let parent = joined
            .parent()
            .context("resolved path has no parent")?
            .canonicalize()
            .with_context(|| format!("canonicalizing parent of {}", joined.display()))?;
        let name = joined.file_name().context("resolved path has no file name")?;
        parent.join(name)
    };

    if !resolved.starts_with(&root_canon) {
        anyhow::bail!("path escapes repository root: {path}");
    }
    Ok(resolved)
}

/// Returns the distinct `{{placeholder}}` names found in `body`, in order.
fn placeholders_in(body: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut rest = body;
    while let Some(open) = rest.find("{{") {
        let after = &rest[open + 2..];
        if let Some(close) = after.find("}}") {
            let name = after[..close].trim();
            if !name.is_empty() && !names.iter().any(|n: &String| n == name) {
                names.push(name.to_string());
            }
            rest = &after[close + 2..];
        } else {
            break;
        }
    }
    names
}

/// Snapshot a PASSING diff-verification as the work order's last-verified
/// checkpoint. Mirrors the CLI's `support::snapshot_checkpoint`: only ever called
/// when `report.allowed` is true, so a checkpoint is never written for unverified
/// work. Enables `resume_task` to resume from the last verified state.
fn snapshot_checkpoint(
    paths: &StatePaths,
    task_id: &str,
    base_ref: &str,
    report: &driftlock_core::DiffReport,
) -> Result<()> {
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
    save_checkpoint(paths, &checkpoint).context("save checkpoint")?;
    Ok(())
}

fn required_str<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .with_context(|| format!("missing string argument: {key}"))
}

fn read_lanes(path: impl AsRef<Path>) -> Result<LaneManifest> {
    let text = fs::read_to_string(path)?;
    Ok(toml::from_str(&text)?)
}

fn default_lanes(repo_root: &Path) -> Result<LaneManifest> {
    read_lanes(repo_root.join("lanes/default.toml"))
}

/// Extracts the caller-supplied change set.
///
/// Returns `None` when neither `diff` nor `changed_files` is provided so callers
/// can distinguish "no evidence supplied" from "an explicitly empty list", and
/// decide whether to fall back to git rather than verifying a vacuous diff.
fn changed_files_from_args(args: &Value) -> Result<Option<Vec<String>>> {
    if let Some(diff) = args.get("diff").and_then(Value::as_str) {
        return Ok(Some(driftlock_git::changed_files_from_diff(diff)));
    }
    Ok(args
        .get("changed_files")
        .and_then(Value::as_array)
        .map(|items| items.iter().filter_map(Value::as_str).map(ToOwned::to_owned).collect()))
}

#[cfg(test)]
mod tests {
    use super::{changed_files_from_args, contained_path};
    use serde_json::json;
    use std::fs;

    #[test]
    fn contained_path_rejects_parent_traversal() {
        let root = std::env::temp_dir().join(format!("dl-contain-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        assert!(contained_path(&root, "../../../etc/passwd").is_err());
        assert!(contained_path(&root, "a/../../escape").is_err());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn contained_path_rejects_absolute() {
        let root = std::env::temp_dir().join(format!("dl-contain-abs-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        #[cfg(unix)]
        assert!(contained_path(&root, "/etc/passwd").is_err());
        #[cfg(windows)]
        assert!(contained_path(&root, "C:\\Windows\\win.ini").is_err());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn contained_path_accepts_in_repo_relative() {
        let root = std::env::temp_dir().join(format!("dl-contain-ok-{}", std::process::id()));
        fs::create_dir_all(root.join("sub")).unwrap();
        fs::write(root.join("sub/file.txt"), "x").unwrap();
        let resolved = contained_path(&root, "sub/file.txt").unwrap();
        assert!(resolved.ends_with("file.txt"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn changed_files_from_args_distinguishes_absent_from_empty() {
        assert!(changed_files_from_args(&json!({})).unwrap().is_none());
        assert_eq!(changed_files_from_args(&json!({"changed_files": []})).unwrap(), Some(vec![]));
    }

    #[test]
    fn resume_task_without_checkpoint_reports_not_resumable() {
        let root = std::env::temp_dir().join(format!("dl-resume-none-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let service = super::DriftlockService::new(root.clone());
        let out = service.call_tool("resume_task", json!({"task_id": "adr-0001:T01"})).unwrap();
        assert_eq!(out["resumable"], json!(false));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn verify_then_resume_round_trips_over_mcp() {
        // A passing verify_diff_against_task snapshots a checkpoint; resume_task
        // then reports the verified file as intact and fully resumable.
        let nonce =
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let root =
            std::env::temp_dir().join(format!("dl-resume-rt-{}-{nonce}", std::process::id()));
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("src/lib.rs"), "verified body").unwrap();

        let graph = json!({
            "schema_version": "0.1.0",
            "graph_id": "g",
            "repo_root": ".",
            "base_ref": "working-tree",
            "generated_at": "2026-06-21T00:00:00+00:00",
            "tasks": [{
                "id": "adr-0001:T01",
                "title": "t",
                "source": {"adr":"a","adr_revision":"r","section":"s","start_line":1,"end_line":1},
                "intent": "i",
                "lane": "core",
                "status": "ready",
                "write_set": ["src/**"],
                "read_set": [],
                "confidence": {"task_extraction":0.95,"file_scope":0.9,"dependency_edges":0.85}
            }],
            "edges": [], "lanes": [], "metadata": {}
        });
        fs::create_dir_all(root.join(".driftlock")).unwrap();
        fs::write(root.join(".driftlock/graph.json"), graph.to_string()).unwrap();

        let service = super::DriftlockService::new(root.clone());
        let verify = service
            .call_tool(
                "verify_diff_against_task",
                json!({
                    "graph_path": ".driftlock/graph.json",
                    "task_id": "adr-0001:T01",
                    "changed_files": ["src/lib.rs"]
                }),
            )
            .unwrap();
        assert_eq!(verify["allowed"], json!(true), "verify must pass to checkpoint");

        let resume = service
            .call_tool(
                "resume_task",
                json!({"graph_path": ".driftlock/graph.json", "task_id": "adr-0001:T01"}),
            )
            .unwrap();
        assert_eq!(resume["intact_files"], json!(1));
        assert_eq!(resume["stale_files"], json!(0));
        assert_eq!(resume["fully_resumable"], json!(true));

        // Corrupt the verified file: resume now flags it stale (drifted).
        fs::write(root.join("src/lib.rs"), "CHANGED").unwrap();
        let resume2 = service
            .call_tool(
                "resume_task",
                json!({"graph_path": ".driftlock/graph.json", "task_id": "adr-0001:T01"}),
            )
            .unwrap();
        assert_eq!(resume2["stale_files"], json!(1));
        assert_eq!(resume2["fully_resumable"], json!(false));

        let _ = fs::remove_dir_all(&root);
    }
}
