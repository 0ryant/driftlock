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
    ready_tasks_for_base, render_agent_brief, verify_changed_files, LaneManifest, TaskGraph,
    TaskStatus,
};
use driftlock_store::{
    complete_claim, init_state_dir, new_claim, record_claim, release_claim, save_graph,
};
use serde_json::{json, Value};
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
                let repo = args
                    .get("repo_root")
                    .and_then(Value::as_str)
                    .map_or(self.repo_root.clone(), PathBuf::from);
                Ok(serde_json::to_value(driftlock_git::index_repo(repo)?)?)
            }
            "extract_tasks" => {
                let adr_path = required_str(&args, "adr_path")?;
                let lane = args.get("lane").and_then(Value::as_str).unwrap_or("core");
                let text = fs::read_to_string(self.resolve(adr_path))?;
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
                    Some(path) => read_lanes(self.resolve(path))?,
                    None => default_lanes(&self.repo_root)?,
                };
                let text = fs::read_to_string(self.resolve(adr_path))?;
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
                let files = changed_files_from_args(&args)?;
                let report = verify_changed_files(wo, &files);
                if !report.allowed {
                    anyhow::bail!("diff verification failed");
                }
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
                let files = changed_files_from_args(&args)?;
                Ok(serde_json::to_value(verify_changed_files(task, &files))?)
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
                json!({"name": p.name, "title": p.name, "description": "Driftlock blessed workflow prompt", "arguments": []})
            })
            .collect()
    }

    /// Renders a prompt by name with optional argument substitution.
    pub fn get_prompt(
        &self,
        name: &str,
        arguments: Option<&serde_json::Map<String, Value>>,
    ) -> Result<String> {
        let prompt = driftlock_skills::find_prompt(name).context("prompt not found")?;
        let mut body = prompt.body.to_string();
        if let Some(args) = arguments {
            if let Some(task_id) = args.get("task_id").and_then(Value::as_str) {
                body = body.replace("{{task_id}}", task_id);
            }
            if let Some(graph_path) = args.get("graph_path").and_then(Value::as_str) {
                body = body.replace("{{graph_path}}", graph_path);
            }
        }
        Ok(body)
    }

    fn resolve(&self, path: &str) -> PathBuf {
        let path = PathBuf::from(path);
        if path.is_absolute() {
            path
        } else {
            self.repo_root.join(path)
        }
    }

    fn read_graph(&self, path: &str) -> Result<TaskGraph> {
        let text = fs::read_to_string(self.resolve(path))?;
        Ok(serde_json::from_str(&text)?)
    }
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

fn changed_files_from_args(args: &Value) -> Result<Vec<String>> {
    if let Some(diff) = args.get("diff").and_then(Value::as_str) {
        return Ok(driftlock_git::changed_files_from_diff(diff));
    }
    Ok(args
        .get("changed_files")
        .and_then(Value::as_array)
        .map(|items| items.iter().filter_map(Value::as_str).map(ToOwned::to_owned).collect())
        .unwrap_or_default())
}
