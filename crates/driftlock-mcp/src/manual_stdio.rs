//! Minimal MCP-compatible stdio JSON-RPC server (legacy transport).
#![allow(
    clippy::unused_self,
    clippy::unnecessary_wraps,
    clippy::needless_pass_by_value,
    clippy::too_many_lines
)]
//!
//! This module intentionally avoids printing anything except JSON-RPC messages to stdout.

use crate::service::DriftlockService;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};
use std::path::PathBuf;

/// Runs the stdio server.
pub fn serve(repo_root: PathBuf) -> Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout().lock();
    let server = ManualMcpServer { service: DriftlockService::new(repo_root) };

    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<JsonRpcRequest>(&line) {
            Ok(req) => {
                if let Some(resp) = server.handle(req) {
                    serde_json::to_writer(&mut stdout, &resp)?;
                    stdout.write_all(b"\n")?;
                    stdout.flush()?;
                }
            }
            Err(err) => {
                let resp = JsonRpcResponse::error(None, -32700, format!("parse error: {err}"));
                serde_json::to_writer(&mut stdout, &resp)?;
                stdout.write_all(b"\n")?;
                stdout.flush()?;
            }
        }
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Option<Value>,
}

#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i64,
    message: String,
}

impl JsonRpcResponse {
    fn result(id: Option<Value>, result: Value) -> Self {
        Self { jsonrpc: "2.0", id, result: Some(result), error: None }
    }

    fn error(id: Option<Value>, code: i64, message: String) -> Self {
        Self { jsonrpc: "2.0", id, result: None, error: Some(JsonRpcError { code, message }) }
    }
}

#[derive(Debug, Clone)]
struct ManualMcpServer {
    service: DriftlockService,
}

impl ManualMcpServer {
    fn handle(&self, req: JsonRpcRequest) -> Option<JsonRpcResponse> {
        let is_notification = req.id.is_none();
        let id = req.id.clone();
        let result = match req.method.as_str() {
            "initialize" => self.initialize(),
            "notifications/initialized" => return None,
            "tools/list" => Ok(json!({"tools": self.service.tool_definitions()})),
            "tools/call" => self.call_tool(req.params.unwrap_or_else(|| json!({}))),
            "resources/list" => Ok(json!({"resources": self.service.resources()})),
            "resources/read" => self.read_resource(req.params.unwrap_or_else(|| json!({}))),
            "prompts/list" => Ok(json!({"prompts": self.service.prompts()})),
            "prompts/get" => self.get_prompt(req.params.unwrap_or_else(|| json!({}))),
            _ => Err(anyhow::anyhow!("unknown method: {}", req.method)),
        };

        if is_notification {
            return None;
        }

        Some(match result {
            Ok(value) => JsonRpcResponse::result(id, value),
            Err(err) => JsonRpcResponse::error(id, -32603, err.to_string()),
        })
    }

    fn initialize(&self) -> Result<Value> {
        Ok(json!({
            "protocolVersion": "2025-06-18",
            "capabilities": {
                "tools": {"listChanged": false},
                "resources": {"listChanged": false},
                "prompts": {"listChanged": false}
            },
            "serverInfo": {"name": "driftlock-mcp", "version": env!("CARGO_PKG_VERSION")},
            "instructions": DriftlockService::instructions()
        }))
    }

    fn call_tool(&self, params: Value) -> Result<Value> {
        let name = params
            .get("name")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("missing tool name"))?;
        let args = params.get("arguments").cloned().unwrap_or_else(|| json!({}));
        let structured = self.service.call_tool(name, args)?;
        Ok(json!({
            "content": [{"type": "text", "text": serde_json::to_string_pretty(&structured)?}],
            "structuredContent": structured,
            "isError": false
        }))
    }

    fn read_resource(&self, params: Value) -> Result<Value> {
        let uri = params
            .get("uri")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("missing uri"))?;
        let (text, mime) = self.service.read_resource(uri)?;
        Ok(json!({"contents": [{"uri": uri, "mimeType": mime, "text": text}]}))
    }

    fn get_prompt(&self, params: Value) -> Result<Value> {
        let name = params
            .get("name")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("missing prompt name"))?;
        let args = params.get("arguments").and_then(Value::as_object);
        let body = self.service.get_prompt(name, args)?;
        Ok(json!({
            "description": name,
            "messages": [{"role":"user", "content": {"type":"text", "text": body}}]
        }))
    }
}
