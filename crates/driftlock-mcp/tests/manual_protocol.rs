#![allow(missing_docs)]

use serde_json::json;

#[test]
fn tool_manifest_contains_ready_tasks() {
    let tools = driftlock_mcp::tool_contracts::tool_definitions();
    assert!(tools.iter().any(|tool| tool["name"] == "ready_tasks"));
}

#[test]
fn tool_structured_content_wraps_non_object_values() {
    let array = driftlock_mcp::service::tool_structured_content(json!(["skill-a"]));
    assert_eq!(array, json!({"result": ["skill-a"]}));

    let object = driftlock_mcp::service::tool_structured_content(json!({"ok": true}));
    assert_eq!(object, json!({"ok": true}));
}
