#![allow(missing_docs)]

#[test]
fn tool_manifest_contains_ready_tasks() {
    let tools = driftlock_mcp::tool_contracts::tool_definitions();
    assert!(tools.iter().any(|tool| tool["name"] == "ready_tasks"));
}
