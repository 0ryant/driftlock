//! MCP server library for Driftlock.

pub mod service;
pub mod tool_contracts;

/// MCP `serverInfo.name` reported by every transport.
///
/// The official `rmcp` SDK defaults this to its own crate name (`"rmcp"`) via
/// `Implementation::from_build_env()`; Driftlock pins it to a stable identity so
/// the manual-stdio and rmcp transports advertise the same server.
pub const SERVER_NAME: &str = "driftlock-mcp";

/// MCP `serverInfo.version` reported by every transport (the crate version).
pub const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

/// MCP `protocolVersion` advertised at `initialize` by every transport.
///
/// Both the manual-stdio and rmcp transports derive their advertised protocol
/// version from this single constant so they cannot drift apart. `rmcp`'s own
/// default (`ProtocolVersion::LATEST`) lags the spec, so we pin it here.
pub const MCP_PROTOCOL_VERSION: &str = "2025-06-18";

#[cfg(feature = "manual-stdio")]
pub mod manual_stdio;

#[cfg(feature = "rmcp-sdk")]
pub mod rmcp_adapter;
