//! Tool schemas and manifests for MCP exposure.

use serde_json::{json, Value};

/// Returns MCP tool definitions.
pub fn tool_definitions() -> Vec<Value> {
    vec![
        tool(
            "index_repo",
            "Index repository files and return a conservative inventory.",
            &json!({"type":"object","properties":{"repo_root":{"type":"string"}},"required":[]}),
        ),
        tool(
            "extract_tasks",
            "Extract proposed work orders from an ADR file.",
            &json!({"type":"object","properties":{"adr_path":{"type":"string"},"lane":{"type":"string"}},"required":["adr_path"]}),
        ),
        tool(
            "build_task_graph",
            "Build a task graph from an ADR and lane manifest.",
            &json!({"type":"object","properties":{"adr_path":{"type":"string"},"lane_manifest_path":{"type":"string"},"lane":{"type":"string"}},"required":["adr_path"]}),
        ),
        tool(
            "check_conflicts",
            "Return conflict report for a task graph.",
            &json!({"type":"object","properties":{"graph_path":{"type":"string"}},"required":["graph_path"]}),
        ),
        tool(
            "ready_tasks",
            "Return tasks ready for a lane.",
            &json!({"type":"object","properties":{"graph_path":{"type":"string"},"lane":{"type":"string"}},"required":["graph_path","lane"]}),
        ),
        tool(
            "agent_brief",
            "Render a bounded implementation brief for one work order.",
            &json!({"type":"object","properties":{"graph_path":{"type":"string"},"task_id":{"type":"string"}},"required":["graph_path","task_id"]}),
        ),
        tool(
            "verify_diff_against_task",
            "Check changed files against a work order write set.",
            &json!({"type":"object","properties":{"graph_path":{"type":"string"},"task_id":{"type":"string"},"changed_files":{"type":"array","items":{"type":"string"}},"diff":{"type":"string"}},"required":["graph_path","task_id"]}),
        ),
        tool(
            "list_skills",
            "List embedded Driftlock skills.",
            &json!({"type":"object","properties":{},"required":[]}),
        ),
        tool(
            "get_skill",
            "Return one embedded Driftlock skill.",
            &json!({"type":"object","properties":{"name":{"type":"string"}},"required":["name"]}),
        ),
        tool(
            "export_schemas",
            "Return the contract schema bundle.",
            &json!({"type":"object","properties":{},"required":[]}),
        ),
        tool(
            "claim_task",
            "Claim a ready work order (writes .driftlock/).",
            &json!({"type":"object","properties":{"graph_path":{"type":"string"},"task_id":{"type":"string"},"actor":{"type":"string"}},"required":["task_id"]}),
        ),
        tool(
            "release_task",
            "Release a task claim.",
            &json!({"type":"object","properties":{"task_id":{"type":"string"},"actor":{"type":"string"}},"required":["task_id"]}),
        ),
        tool(
            "complete_task",
            "Complete a claimed task after diff verification.",
            &json!({"type":"object","properties":{"graph_path":{"type":"string"},"task_id":{"type":"string"},"actor":{"type":"string"},"changed_files":{"type":"array","items":{"type":"string"}},"diff":{"type":"string"}},"required":["task_id"]}),
        ),
    ]
}

fn tool(name: &str, description: &str, input_schema: &Value) -> Value {
    json!({"name": name, "title": name, "description": description, "inputSchema": input_schema})
}
