#![allow(missing_docs)]
//! SDK MCP Server - In-process MCP server implementation
//!
//! This module provides an in-process MCP server that runs directly within your
//! Rust application, eliminating the need for separate processes.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;

use crate::errors::{Result, SdkError};

/// Tool input schema definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInputSchema {
    #[serde(rename = "type")]
    pub schema_type: String,
    pub properties: HashMap<String, Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,
}

/// Tool definition
#[derive(Clone)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: ToolInputSchema,
    pub handler: Arc<dyn ToolHandler>,
}

impl std::fmt::Debug for ToolDefinition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolDefinition")
            .field("name", &self.name)
            .field("description", &self.description)
            .field("input_schema", &self.input_schema)
            .field("handler", &"<Arc<dyn ToolHandler>>")
            .finish()
    }
}

/// Tool handler trait
#[async_trait]
pub trait ToolHandler: Send + Sync {
    async fn execute(&self, args: Value) -> Result<ToolResult>;
}

/// Tool execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub content: Vec<ToolResultContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

/// Tool result content types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ToolResultContent {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image")]
    Image {
        data: String,
        #[serde(rename = "mimeType")]
        mime_type: String,
    },
}

/// SDK MCP Server
pub struct SdkMcpServer {
    pub name: String,
    pub version: String,
    pub tools: Vec<ToolDefinition>,
}

impl SdkMcpServer {
    /// Create a new SDK MCP server
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            tools: Vec::new(),
        }
    }

    /// Add a tool to the server
    pub fn add_tool(&mut self, tool: ToolDefinition) {
        self.tools.push(tool);
    }

    /// Handle MCP protocol messages
    pub async fn handle_message(&self, message: Value) -> Result<Value> {
        let method = message
            .get("method")
            .and_then(|m| m.as_str())
            .ok_or_else(|| SdkError::InvalidState {
                message: "Missing method in MCP message".to_string(),
            })?;

        let id = message.get("id");

        match method {
            "initialize" => Ok(json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "protocolVersion": "2024-11-05",
                    "capabilities": {
                        "tools": {}
                    },
                    "serverInfo": {
                        "name": self.name,
                        "version": self.version
                    }
                }
            })),

            "tools/list" => {
                let tools: Vec<Value> = self
                    .tools
                    .iter()
                    .map(|tool| {
                        json!({
                            "name": tool.name,
                            "description": tool.description,
                            "inputSchema": tool.input_schema
                        })
                    })
                    .collect();

                Ok(json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "tools": tools
                    }
                }))
            },

            "tools/call" => {
                let params = message
                    .get("params")
                    .ok_or_else(|| SdkError::InvalidState {
                        message: "Missing params in tools/call".to_string(),
                    })?;

                let tool_name = params.get("name").and_then(|n| n.as_str()).ok_or_else(|| {
                    SdkError::InvalidState {
                        message: "Missing tool name in tools/call".to_string(),
                    }
                })?;

                let empty_args = json!({});
                let arguments = params.get("arguments").unwrap_or(&empty_args);

                // Find and execute the tool
                let tool = self
                    .tools
                    .iter()
                    .find(|t| t.name == tool_name)
                    .ok_or_else(|| SdkError::InvalidState {
                        message: format!("Tool not found: {tool_name}"),
                    })?;

                let result = tool.handler.execute(arguments.clone()).await?;

                Ok(json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "content": result.content,
                        "isError": result.is_error
                    }
                }))
            },

            "notifications/initialized" => {
                // Acknowledge initialization notification
                Ok(json!({
                    "jsonrpc": "2.0",
                    "result": {}
                }))
            },

            _ => Ok(json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {
                    "code": -32601,
                    "message": format!("Method '{}' not found", method)
                }
            })),
        }
    }
}

impl SdkMcpServer {
    /// Convert to McpServerConfig
    pub fn to_config(self) -> crate::types::McpServerConfig {
        use std::sync::Arc;
        crate::types::McpServerConfig::Sdk {
            name: self.name.clone(),
            instance: Arc::new(self),
        }
    }
}

/// Builder for creating SDK MCP servers
pub struct SdkMcpServerBuilder {
    name: String,
    version: String,
    tools: Vec<ToolDefinition>,
}

impl SdkMcpServerBuilder {
    /// Create a new builder
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: "1.0.0".to_string(),
            tools: Vec::new(),
        }
    }

    /// Set server version
    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }

    /// Add a tool
    pub fn tool(mut self, tool: ToolDefinition) -> Self {
        self.tools.push(tool);
        self
    }

    /// Build the server
    pub fn build(self) -> SdkMcpServer {
        SdkMcpServer {
            name: self.name,
            version: self.version,
            tools: self.tools,
        }
    }
}

/// Helper function to create a simple text-based tool
pub fn create_simple_tool<F, Fut>(
    name: impl Into<String>,
    description: impl Into<String>,
    schema: ToolInputSchema,
    handler: F,
) -> ToolDefinition
where
    F: Fn(Value) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = Result<String>> + Send + 'static,
{
    struct SimpleHandler<F, Fut>
    where
        F: Fn(Value) -> Fut + Send + Sync,
        Fut: std::future::Future<Output = Result<String>> + Send,
    {
        func: F,
    }

    #[async_trait]
    impl<F, Fut> ToolHandler for SimpleHandler<F, Fut>
    where
        F: Fn(Value) -> Fut + Send + Sync,
        Fut: std::future::Future<Output = Result<String>> + Send,
    {
        async fn execute(&self, args: Value) -> Result<ToolResult> {
            let text = (self.func)(args).await?;
            Ok(ToolResult {
                content: vec![ToolResultContent::Text { text }],
                is_error: None,
            })
        }
    }

    ToolDefinition {
        name: name.into(),
        description: description.into(),
        input_schema: schema,
        handler: Arc::new(SimpleHandler { func: handler }),
    }
}

/// Macro to define a tool with a simple syntax
#[macro_export]
macro_rules! tool {
    ($name:expr, $desc:expr, $schema:expr, $handler:expr) => {
        $crate::sdk_mcp::create_simple_tool($name, $desc, $schema, $handler)
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sdk_mcp_server() {
        let mut server = SdkMcpServer::new("test-server", "1.0.0");

        // Add a simple tool
        let tool = create_simple_tool(
            "greet",
            "Greet a user",
            ToolInputSchema {
                schema_type: "object".to_string(),
                properties: {
                    let mut props = HashMap::new();
                    props.insert(
                        "name".to_string(),
                        json!({"type": "string", "description": "Name to greet"}),
                    );
                    props
                },
                required: Some(vec!["name".to_string()]),
            },
            |args| async move {
                let name = args["name"].as_str().unwrap_or("stranger");
                Ok(format!("Hello, {name}!"))
            },
        );

        server.add_tool(tool);

        // Test initialize
        let init_msg = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize"
        });

        let response = server.handle_message(init_msg).await.unwrap();
        assert_eq!(response["result"]["serverInfo"]["name"], "test-server");

        // Test tools/list
        let list_msg = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list"
        });

        let response = server.handle_message(list_msg).await.unwrap();
        assert_eq!(response["result"]["tools"][0]["name"], "greet");

        // Test tools/call
        let call_msg = json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "greet",
                "arguments": {
                    "name": "Alice"
                }
            }
        });

        let response = server.handle_message(call_msg).await.unwrap();
        assert_eq!(response["result"]["content"][0]["text"], "Hello, Alice!");
    }
}
