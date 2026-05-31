#![allow(missing_docs)]

use serde_json::json;

#[test]
fn tool_manifest_contains_ready_tasks() {
    let tools = driftlock_mcp::tool_contracts::tool_definitions();
    assert!(tools.iter().any(|tool| tool["name"] == "ready_tasks"));
}

#[test]
fn tool_manifest_contains_claim_lifecycle() {
    let tools = driftlock_mcp::tool_contracts::tool_definitions();
    for expected in ["claim_task", "release_task", "complete_task"] {
        assert!(
            tools.iter().any(|tool| tool["name"] == expected),
            "tool manifest missing {expected}"
        );
    }
}

#[test]
fn tool_structured_content_wraps_non_object_values() {
    let array = driftlock_mcp::service::tool_structured_content(json!(["skill-a"]));
    assert_eq!(array, json!({"result": ["skill-a"]}));

    let object = driftlock_mcp::service::tool_structured_content(json!({"ok": true}));
    assert_eq!(object, json!({"ok": true}));
}

#[test]
fn server_identity_constants_are_pinned() {
    // serverInfo identity must be Driftlock's, not the rmcp SDK default ("rmcp").
    assert_eq!(driftlock_mcp::SERVER_NAME, "driftlock-mcp");
    assert_eq!(driftlock_mcp::SERVER_VERSION, env!("CARGO_PKG_VERSION"));
    assert_eq!(driftlock_mcp::MCP_PROTOCOL_VERSION, "2025-06-18");
}

#[cfg(feature = "rmcp-sdk")]
#[test]
fn rmcp_get_info_advertises_driftlock_identity_and_shared_protocol() {
    use rmcp::ServerHandler;

    let server = driftlock_mcp::rmcp_adapter::DriftlockRmcp::new(std::path::PathBuf::from("."));
    let info = server.get_info();

    // Item 1: serverInfo name/version come from the shared constants, not "rmcp".
    assert_eq!(info.server_info.name, driftlock_mcp::SERVER_NAME);
    assert_eq!(info.server_info.version, driftlock_mcp::SERVER_VERSION);

    // Item 3: protocolVersion is derived from the shared constant (serializes to the string).
    let advertised = serde_json::to_value(&info.protocol_version).expect("serialize version");
    assert_eq!(advertised, json!(driftlock_mcp::MCP_PROTOCOL_VERSION));

    // Both transports must advertise the same protocol version.
    assert_eq!(advertised.as_str(), Some(driftlock_mcp::MCP_PROTOCOL_VERSION));
}
