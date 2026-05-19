//! MCP server library for Driftlock.

pub mod service;
pub mod tool_contracts;

#[cfg(feature = "manual-stdio")]
pub mod manual_stdio;

#[cfg(feature = "rmcp-sdk")]
pub mod rmcp_adapter;
