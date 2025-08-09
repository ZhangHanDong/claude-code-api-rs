//! Type definitions for the Claude Code SDK
//!
//! This module contains all the core types used throughout the SDK,
//! including messages, configuration options, and content blocks.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Permission mode for tool execution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PermissionMode {
    /// Default mode - CLI prompts for dangerous tools
    Default,
    /// Auto-accept file edits
    AcceptEdits,
    /// Allow all tools without prompting (use with caution)
    BypassPermissions,
}

impl Default for PermissionMode {
    fn default() -> Self {
        Self::Default
    }
}

/// MCP (Model Context Protocol) server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum McpServerConfig {
    /// Standard I/O based MCP server
    Stdio {
        /// Command to execute
        command: String,
        /// Command arguments
        #[serde(skip_serializing_if = "Option::is_none")]
        args: Option<Vec<String>>,
        /// Environment variables
        #[serde(skip_serializing_if = "Option::is_none")]
        env: Option<HashMap<String, String>>,
    },
    /// Server-Sent Events based MCP server
    Sse {
        /// Server URL
        url: String,
        /// HTTP headers
        #[serde(skip_serializing_if = "Option::is_none")]
        headers: Option<HashMap<String, String>>,
    },
    /// HTTP-based MCP server
    Http {
        /// Server URL
        url: String,
        /// HTTP headers
        #[serde(skip_serializing_if = "Option::is_none")]
        headers: Option<HashMap<String, String>>,
    },
}

/// Configuration options for Claude Code SDK
#[derive(Debug, Clone, Default)]
pub struct ClaudeCodeOptions {
    /// System prompt to prepend to all messages
    pub system_prompt: Option<String>,
    /// Additional system prompt to append
    pub append_system_prompt: Option<String>,
    /// List of allowed tools
    pub allowed_tools: Vec<String>,
    /// List of disallowed tools
    pub disallowed_tools: Vec<String>,
    /// Permission mode for tool execution
    pub permission_mode: PermissionMode,
    /// MCP server configurations
    pub mcp_servers: HashMap<String, McpServerConfig>,
    /// MCP tools to enable
    pub mcp_tools: Vec<String>,
    /// Maximum number of conversation turns
    pub max_turns: Option<i32>,
    /// Maximum thinking tokens
    pub max_thinking_tokens: i32,
    /// Model to use
    pub model: Option<String>,
    /// Working directory
    pub cwd: Option<PathBuf>,
    /// Continue from previous conversation
    pub continue_conversation: bool,
    /// Resume from a specific conversation ID
    pub resume: Option<String>,
    /// Custom permission prompt tool name
    pub permission_prompt_tool_name: Option<String>,
    /// Settings file path for Claude Code CLI
    pub settings: Option<String>,
    /// Additional directories to add as working directories
    pub add_dirs: Vec<PathBuf>,
}

impl ClaudeCodeOptions {
    /// Create a new options builder
    pub fn builder() -> ClaudeCodeOptionsBuilder {
        ClaudeCodeOptionsBuilder::default()
    }
}

/// Builder for ClaudeCodeOptions
#[derive(Debug, Default)]
pub struct ClaudeCodeOptionsBuilder {
    options: ClaudeCodeOptions,
}

impl ClaudeCodeOptionsBuilder {
    /// Set system prompt
    pub fn system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.options.system_prompt = Some(prompt.into());
        self
    }

    /// Set append system prompt
    pub fn append_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.options.append_system_prompt = Some(prompt.into());
        self
    }

    /// Add allowed tools
    pub fn allowed_tools(mut self, tools: Vec<String>) -> Self {
        self.options.allowed_tools = tools;
        self
    }

    /// Add a single allowed tool
    pub fn allow_tool(mut self, tool: impl Into<String>) -> Self {
        self.options.allowed_tools.push(tool.into());
        self
    }

    /// Add disallowed tools
    pub fn disallowed_tools(mut self, tools: Vec<String>) -> Self {
        self.options.disallowed_tools = tools;
        self
    }

    /// Add a single disallowed tool
    pub fn disallow_tool(mut self, tool: impl Into<String>) -> Self {
        self.options.disallowed_tools.push(tool.into());
        self
    }

    /// Set permission mode
    pub fn permission_mode(mut self, mode: PermissionMode) -> Self {
        self.options.permission_mode = mode;
        self
    }

    /// Add MCP server
    pub fn add_mcp_server(mut self, name: impl Into<String>, config: McpServerConfig) -> Self {
        self.options.mcp_servers.insert(name.into(), config);
        self
    }

    /// Set MCP tools
    pub fn mcp_tools(mut self, tools: Vec<String>) -> Self {
        self.options.mcp_tools = tools;
        self
    }

    /// Set max turns
    pub fn max_turns(mut self, turns: i32) -> Self {
        self.options.max_turns = Some(turns);
        self
    }

    /// Set max thinking tokens
    pub fn max_thinking_tokens(mut self, tokens: i32) -> Self {
        self.options.max_thinking_tokens = tokens;
        self
    }

    /// Set model
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.options.model = Some(model.into());
        self
    }

    /// Set working directory
    pub fn cwd(mut self, path: impl Into<PathBuf>) -> Self {
        self.options.cwd = Some(path.into());
        self
    }

    /// Enable continue conversation
    pub fn continue_conversation(mut self, enable: bool) -> Self {
        self.options.continue_conversation = enable;
        self
    }

    /// Set resume conversation ID
    pub fn resume(mut self, id: impl Into<String>) -> Self {
        self.options.resume = Some(id.into());
        self
    }
    
    /// Set permission prompt tool name
    pub fn permission_prompt_tool_name(mut self, name: impl Into<String>) -> Self {
        self.options.permission_prompt_tool_name = Some(name.into());
        self
    }

    /// Set settings file path
    pub fn settings(mut self, settings: impl Into<String>) -> Self {
        self.options.settings = Some(settings.into());
        self
    }

    /// Add directories as working directories
    pub fn add_dirs(mut self, dirs: Vec<PathBuf>) -> Self {
        self.options.add_dirs = dirs;
        self
    }

    /// Add a single directory as working directory
    pub fn add_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.options.add_dirs.push(dir.into());
        self
    }

    /// Build the options
    pub fn build(self) -> ClaudeCodeOptions {
        self.options
    }
}

/// Main message type enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Message {
    /// User message
    User {
        /// Message content
        message: UserMessage,
    },
    /// Assistant message
    Assistant {
        /// Message content
        message: AssistantMessage,
    },
    /// System message
    System {
        /// Subtype of system message
        subtype: String,
        /// Additional data
        data: serde_json::Value,
    },
    /// Result message indicating end of turn
    Result {
        /// Result subtype
        subtype: String,
        /// Duration in milliseconds
        duration_ms: i64,
        /// API duration in milliseconds
        duration_api_ms: i64,
        /// Whether an error occurred
        is_error: bool,
        /// Number of turns
        num_turns: i32,
        /// Session ID
        session_id: String,
        /// Total cost in USD
        #[serde(skip_serializing_if = "Option::is_none")]
        total_cost_usd: Option<f64>,
        /// Usage statistics
        #[serde(skip_serializing_if = "Option::is_none")]
        usage: Option<serde_json::Value>,
        /// Result message
        #[serde(skip_serializing_if = "Option::is_none")]
        result: Option<String>,
    },
}

/// User message content
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UserMessage {
    /// Message content
    pub content: String,
}

/// Assistant message content
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssistantMessage {
    /// Content blocks
    pub content: Vec<ContentBlock>,
}

/// System message (re-export for convenience)
pub use Message::System as SystemMessage;
/// Result message (re-export for convenience)  
pub use Message::Result as ResultMessage;

/// Content block types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum ContentBlock {
    /// Text content
    Text(TextContent),
    /// Tool use request
    ToolUse(ToolUseContent),
    /// Tool result
    ToolResult(ToolResultContent),
}

/// Text content block
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TextContent {
    /// Text content
    pub text: String,
}

/// Tool use content block
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolUseContent {
    /// Tool use ID
    pub id: String,
    /// Tool name
    pub name: String,
    /// Tool input parameters
    pub input: serde_json::Value,
}

/// Tool result content block
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolResultContent {
    /// Tool use ID this result corresponds to
    pub tool_use_id: String,
    /// Result content
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<ContentValue>,
    /// Whether this is an error result
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

/// Content value for tool results
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum ContentValue {
    /// Text content
    Text(String),
    /// Structured content
    Structured(Vec<serde_json::Value>),
}

/// User content structure for internal use
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserContent {
    /// Role (always "user")
    pub role: String,
    /// Message content
    pub content: String,
}

/// Assistant content structure for internal use
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantContent {
    /// Role (always "assistant")
    pub role: String,
    /// Content blocks
    pub content: Vec<ContentBlock>,
}

/// Control request types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ControlRequest {
    /// Interrupt the current operation
    Interrupt {
        /// Request ID
        request_id: String,
    },
}

/// Control response types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ControlResponse {
    /// Interrupt acknowledged
    InterruptAck {
        /// Request ID
        request_id: String,
        /// Whether interrupt was successful
        success: bool,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_mode_serialization() {
        let mode = PermissionMode::AcceptEdits;
        let json = serde_json::to_string(&mode).unwrap();
        assert_eq!(json, r#""acceptEdits""#);

        let deserialized: PermissionMode = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, mode);
    }

    #[test]
    fn test_message_serialization() {
        let msg = Message::User {
            message: UserMessage {
                content: "Hello".to_string(),
            },
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains(r#""type":"user""#));
        assert!(json.contains(r#""content":"Hello""#));

        let deserialized: Message = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, msg);
    }

    #[test]
    fn test_options_builder() {
        let options = ClaudeCodeOptions::builder()
            .system_prompt("Test prompt")
            .model("claude-3-opus")
            .permission_mode(PermissionMode::AcceptEdits)
            .allow_tool("read")
            .allow_tool("write")
            .max_turns(10)
            .build();

        assert_eq!(options.system_prompt, Some("Test prompt".to_string()));
        assert_eq!(options.model, Some("claude-3-opus".to_string()));
        assert_eq!(options.permission_mode, PermissionMode::AcceptEdits);
        assert_eq!(options.allowed_tools, vec!["read", "write"]);
        assert_eq!(options.max_turns, Some(10));
    }
}