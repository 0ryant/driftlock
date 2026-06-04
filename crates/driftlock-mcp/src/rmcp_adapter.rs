//! Official `rmcp` SDK transport (default).
#![allow(
    clippy::manual_async_fn,
    clippy::redundant_closure,
    clippy::map_unwrap_or,
    clippy::implicit_clone,
    clippy::needless_pass_by_value
)]

use crate::service::{tool_structured_content, DriftlockService};
use rmcp::{
    model::{
        Annotated, CallToolResult, Content, GetPromptResult, Implementation, ListPromptsResult,
        ListResourcesResult, ListToolsResult, Prompt, PromptMessage, PromptMessageRole,
        ProtocolVersion, RawResource, ReadResourceResult, ResourceContents, ServerCapabilities,
        ServerInfo,
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
        let capabilities = ServerCapabilities::builder()
            .enable_tools()
            .enable_resources()
            .enable_prompts()
            .build();
        ServerInfo::new(capabilities)
            .with_protocol_version(protocol_version())
            .with_instructions(DriftlockService::instructions())
            .with_server_info(Implementation::new(
                crate::SERVER_NAME.to_string(),
                crate::SERVER_VERSION.to_string(),
            ))
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
                Ok(value) => {
                    let structured = tool_structured_content(value);
                    Ok(CallToolResult::structured(structured))
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
            Ok(ReadResourceResult::new(vec![
                ResourceContents::text(text, uri).with_mime_type(mime.to_string())
            ]))
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
                    let name = p.get("name")?.as_str()?.to_string();
                    let description =
                        p.get("description").and_then(Value::as_str).map(str::to_string);
                    let mut prompt = Prompt::new(name, description, None);
                    if let Some(title) = p.get("title").and_then(Value::as_str) {
                        prompt = prompt.with_title(title);
                    }
                    Some(prompt)
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
            Ok(GetPromptResult::new(vec![PromptMessage::new_text(PromptMessageRole::User, body)])
                .with_description(name))
        }
    }
}

/// Builds the advertised [`ProtocolVersion`] from the shared crate constant so
/// the rmcp transport never drifts from the manual-stdio transport.
///
/// `ProtocolVersion` only exposes a string constructor through `Deserialize`,
/// so we round-trip the constant through serde. Falls back to the rmcp default
/// only if the constant is somehow unparseable (it is a compile-time literal,
/// so this branch is unreachable in practice and is asserted in tests).
fn protocol_version() -> ProtocolVersion {
    serde_json::from_value(Value::String(crate::MCP_PROTOCOL_VERSION.to_string()))
        .unwrap_or_default()
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
    let mut tool = rmcp::model::Tool::new_with_raw(
        Cow::Owned(name.to_string()),
        description.map(|d| Cow::Owned(d.to_string())),
        Arc::new(schema),
    );
    if let Some(title) = def.get("title").and_then(Value::as_str) {
        tool = tool.with_title(title);
    }
    Some(tool)
}
