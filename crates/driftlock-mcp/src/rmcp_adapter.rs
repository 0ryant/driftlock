//! Official `rmcp` SDK transport (default).
#![allow(
    clippy::manual_async_fn,
    clippy::redundant_closure,
    clippy::map_unwrap_or,
    clippy::implicit_clone,
    clippy::needless_pass_by_value
)]

use crate::service::DriftlockService;
use rmcp::{
    model::{
        Annotated, CallToolResult, Content, GetPromptResult, ListPromptsResult,
        ListResourcesResult, ListToolsResult, Prompt, PromptMessage, PromptMessageRole,
        RawResource, ReadResourceResult, ResourceContents, ServerCapabilities, ServerInfo,
    },
    transport::stdio,
    ErrorData as McpError, ServerHandler, ServiceExt,
};
use serde_json::{json, Value};
use std::borrow::Cow;
use std::path::PathBuf;
use std::sync::Arc;

/// Driftlock MCP server using the official rmcp SDK.
#[derive(Clone)]
pub struct DriftlockRmcp {
    service: DriftlockService,
}

impl DriftlockRmcp {
    /// Creates a server for `repo_root`.
    pub fn new(repo_root: PathBuf) -> Self {
        Self { service: DriftlockService::new(repo_root) }
    }
}

impl ServerHandler for DriftlockRmcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(DriftlockService::instructions().into()),
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .enable_prompts()
                .build(),
            ..Default::default()
        }
    }

    fn list_tools(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParams>,
        _context: rmcp::service::RequestContext<rmcp::service::RoleServer>,
    ) -> impl std::future::Future<Output = Result<ListToolsResult, McpError>> + Send + '_ {
        async move {
            let tools = DriftlockService::tool_definitions()
                .into_iter()
                .filter_map(|def| json_tool_to_rmcp(def))
                .collect();
            Ok(ListToolsResult { tools, next_cursor: None, meta: None })
        }
    }

    fn call_tool(
        &self,
        request: rmcp::model::CallToolRequestParams,
        _context: rmcp::service::RequestContext<rmcp::service::RoleServer>,
    ) -> impl std::future::Future<Output = Result<CallToolResult, McpError>> + Send + '_ {
        async move {
            let args = request.arguments.map(Value::Object).unwrap_or_else(|| json!({}));
            match self.service.call_tool(&request.name, args) {
                Ok(structured) => {
                    let text = serde_json::to_string_pretty(&structured)
                        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                    Ok(CallToolResult {
                        content: vec![Content::text(text)],
                        structured_content: Some(structured),
                        is_error: Some(false),
                        meta: None,
                    })
                }
                Err(err) => Ok(CallToolResult::error(vec![Content::text(err.to_string())])),
            }
        }
    }

    fn list_resources(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParams>,
        _context: rmcp::service::RequestContext<rmcp::service::RoleServer>,
    ) -> impl std::future::Future<Output = Result<ListResourcesResult, McpError>> + Send + '_ {
        async move {
            let resources = self
                .service
                .resources()
                .into_iter()
                .filter_map(|r| {
                    let uri = r.get("uri")?.as_str()?.to_string();
                    let name = r.get("name")?.as_str()?.to_string();
                    let mime = r.get("mimeType").and_then(Value::as_str).map(str::to_string);
                    Some(Annotated::new(
                        RawResource {
                            uri,
                            name,
                            title: None,
                            description: None,
                            mime_type: mime,
                            size: None,
                            icons: None,
                            meta: None,
                        },
                        None,
                    ))
                })
                .collect();
            Ok(ListResourcesResult { resources, next_cursor: None, meta: None })
        }
    }

    fn read_resource(
        &self,
        request: rmcp::model::ReadResourceRequestParams,
        _context: rmcp::service::RequestContext<rmcp::service::RoleServer>,
    ) -> impl std::future::Future<Output = Result<ReadResourceResult, McpError>> + Send + '_ {
        async move {
            let uri = request.uri.to_string();
            let (text, mime) = self
                .service
                .read_resource(&uri)
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
            Ok(ReadResourceResult {
                contents: vec![ResourceContents::TextResourceContents {
                    uri,
                    mime_type: Some(mime.to_string()),
                    text,
                    meta: None,
                }],
            })
        }
    }

    fn list_prompts(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParams>,
        _context: rmcp::service::RequestContext<rmcp::service::RoleServer>,
    ) -> impl std::future::Future<Output = Result<ListPromptsResult, McpError>> + Send + '_ {
        async move {
            let prompts = self
                .service
                .prompts()
                .into_iter()
                .filter_map(|p| {
                    Some(Prompt {
                        name: p.get("name")?.as_str()?.to_string(),
                        title: p.get("title").and_then(Value::as_str).map(str::to_string),
                        description: p
                            .get("description")
                            .and_then(Value::as_str)
                            .map(str::to_string),
                        arguments: None,
                        icons: None,
                        meta: None,
                    })
                })
                .collect();
            Ok(ListPromptsResult { prompts, next_cursor: None, meta: None })
        }
    }

    fn get_prompt(
        &self,
        request: rmcp::model::GetPromptRequestParams,
        _context: rmcp::service::RequestContext<rmcp::service::RoleServer>,
    ) -> impl std::future::Future<Output = Result<GetPromptResult, McpError>> + Send + '_ {
        async move {
            let name = request.name.to_string();
            let body = self
                .service
                .get_prompt(&name, request.arguments.as_ref())
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
            Ok(GetPromptResult {
                description: Some(name),
                messages: vec![PromptMessage::new_text(PromptMessageRole::User, body)],
            })
        }
    }
}

/// Runs the rmcp adapter over stdio.
pub async fn serve_rmcp_stdio(repo_root: PathBuf) -> anyhow::Result<()> {
    let service = DriftlockRmcp::new(repo_root).serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}

fn json_tool_to_rmcp(def: Value) -> Option<rmcp::model::Tool> {
    let name = def.get("name")?.as_str()?;
    let description = def.get("description").and_then(Value::as_str);
    let schema = def.get("inputSchema")?.as_object()?.clone();
    Some(rmcp::model::Tool {
        name: Cow::Owned(name.to_string()),
        title: def.get("title").and_then(Value::as_str).map(str::to_string),
        description: description.map(|d| Cow::Owned(d.to_string())),
        input_schema: Arc::new(schema),
        output_schema: None,
        annotations: None,
        execution: None,
        icons: None,
        meta: None,
    })
}
