//! Type definitions for the Claude Code SDK
//!
//! This module contains all the core types used throughout the SDK,
//! including messages, configuration options, and content blocks.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use async_trait::async_trait;
use std::io::Write;
use tokio::sync::Mutex;

/// Permission mode for tool execution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PermissionMode {
    /// Default mode - CLI prompts for dangerous tools
    Default,
    /// Auto-accept file edits
    AcceptEdits,
    /// Plan mode - for planning tasks
    Plan,
    /// Allow all tools without prompting (use with caution)
    BypassPermissions,
}

impl Default for PermissionMode {
    fn default() -> Self {
        Self::Default
    }
}

/// Control protocol format for sending messages
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlProtocolFormat {
    /// Legacy format: {"type":"sdk_control_request","request":{...}}
    Legacy,
    /// New format: {"type":"control","control":{...}}
    Control,
    /// Auto-detect based on CLI capabilities (default to Legacy for compatibility)
    Auto,
}

impl Default for ControlProtocolFormat {
    fn default() -> Self {
        // Default to Legacy for maximum compatibility
        Self::Legacy
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
    /// SDK MCP server (in-process)
    #[serde(rename = "sdk")]
    Sdk {
        /// Server name
        name: String,
        /// Server instance (will be skipped in serialization)
        #[serde(skip)]
        instance: Option<Arc<dyn std::any::Any + Send + Sync>>,
    },
}

/// Permission update destination
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PermissionUpdateDestination {
    /// User settings
    UserSettings,
    /// Project settings
    ProjectSettings,
    /// Local settings
    LocalSettings,
    /// Session
    Session,
}

/// Permission behavior
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PermissionBehavior {
    /// Allow the action
    Allow,
    /// Deny the action
    Deny,
    /// Ask the user
    Ask,
}

/// Permission rule value
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionRuleValue {
    /// Tool name
    pub tool_name: String,
    /// Rule content
    pub rule_content: Option<String>,
}

/// Permission update type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PermissionUpdateType {
    /// Add rules
    AddRules,
    /// Replace rules
    ReplaceRules,
    /// Remove rules
    RemoveRules,
    /// Set mode
    SetMode,
    /// Add directories
    AddDirectories,
    /// Remove directories
    RemoveDirectories,
}

/// Permission update
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionUpdate {
    /// Update type
    #[serde(rename = "type")]
    pub update_type: PermissionUpdateType,
    /// Rules to update
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rules: Option<Vec<PermissionRuleValue>>,
    /// Behavior to set
    #[serde(skip_serializing_if = "Option::is_none")]
    pub behavior: Option<PermissionBehavior>,
    /// Mode to set
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<PermissionMode>,
    /// Directories to add/remove
    #[serde(skip_serializing_if = "Option::is_none")]
    pub directories: Option<Vec<String>>,
    /// Destination for the update
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination: Option<PermissionUpdateDestination>,
}

/// Tool permission context
#[derive(Debug, Clone)]
pub struct ToolPermissionContext {
    /// Abort signal (future support)
    pub signal: Option<Arc<dyn std::any::Any + Send + Sync>>,
    /// Permission suggestions from CLI
    pub suggestions: Vec<PermissionUpdate>,
}

/// Permission result - Allow
#[derive(Debug, Clone)]
pub struct PermissionResultAllow {
    /// Updated input parameters
    pub updated_input: Option<serde_json::Value>,
    /// Updated permissions
    pub updated_permissions: Option<Vec<PermissionUpdate>>,
}

/// Permission result - Deny
#[derive(Debug, Clone)]
pub struct PermissionResultDeny {
    /// Denial message
    pub message: String,
    /// Whether to interrupt the conversation
    pub interrupt: bool,
}

/// Permission result
#[derive(Debug, Clone)]
pub enum PermissionResult {
    /// Allow the tool use
    Allow(PermissionResultAllow),
    /// Deny the tool use
    Deny(PermissionResultDeny),
}

/// Tool permission callback trait
#[async_trait]
pub trait CanUseTool: Send + Sync {
    /// Check if a tool can be used
    async fn can_use_tool(
        &self,
        tool_name: &str,
        input: &serde_json::Value,
        context: &ToolPermissionContext,
    ) -> PermissionResult;
}

/// Hook context
#[derive(Debug, Clone)]
pub struct HookContext {
    /// Abort signal (future support)
    pub signal: Option<Arc<dyn std::any::Any + Send + Sync>>,
}

/// Hook callback trait
#[async_trait]
pub trait HookCallback: Send + Sync {
    /// Execute the hook
    async fn execute(
        &self,
        input: &serde_json::Value,
        tool_use_id: Option<&str>,
        context: &HookContext,
    ) -> serde_json::Value;
}

/// Hook matcher configuration
#[derive(Clone)]
pub struct HookMatcher {
    /// Matcher criteria
    pub matcher: Option<serde_json::Value>,
    /// Callbacks to invoke
    pub hooks: Vec<Arc<dyn HookCallback>>,
}

/// Configuration options for Claude Code SDK
#[derive(Clone, Default)]
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
    /// Extra arbitrary CLI flags
    pub extra_args: HashMap<String, Option<String>>,
    /// Environment variables to pass to the process
    pub env: HashMap<String, String>,
    /// Debug output stream (e.g., stderr)
    pub debug_stderr: Option<Arc<Mutex<dyn Write + Send + Sync>>>,
    /// Tool permission callback
    pub can_use_tool: Option<Arc<dyn CanUseTool>>,
    /// Hook configurations
    pub hooks: Option<HashMap<String, Vec<HookMatcher>>>,
    /// Control protocol format (defaults to Legacy for compatibility)
    pub control_protocol_format: ControlProtocolFormat,
}

impl std::fmt::Debug for ClaudeCodeOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClaudeCodeOptions")
            .field("system_prompt", &self.system_prompt)
            .field("append_system_prompt", &self.append_system_prompt)
            .field("allowed_tools", &self.allowed_tools)
            .field("disallowed_tools", &self.disallowed_tools)
            .field("permission_mode", &self.permission_mode)
            .field("mcp_servers", &self.mcp_servers)
            .field("mcp_tools", &self.mcp_tools)
            .field("max_turns", &self.max_turns)
            .field("max_thinking_tokens", &self.max_thinking_tokens)
            .field("model", &self.model)
            .field("cwd", &self.cwd)
            .field("continue_conversation", &self.continue_conversation)
            .field("resume", &self.resume)
            .field("permission_prompt_tool_name", &self.permission_prompt_tool_name)
            .field("settings", &self.settings)
            .field("add_dirs", &self.add_dirs)
            .field("extra_args", &self.extra_args)
            .field("env", &self.env)
            .field("debug_stderr", &self.debug_stderr.is_some())
            .field("can_use_tool", &self.can_use_tool.is_some())
            .field("hooks", &self.hooks.is_some())
            .field("control_protocol_format", &self.control_protocol_format)
            .finish()
    }
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

    /// Add extra CLI arguments
    pub fn extra_args(mut self, args: HashMap<String, Option<String>>) -> Self {
        self.options.extra_args = args;
        self
    }

    /// Add a single extra CLI argument
    pub fn add_extra_arg(mut self, key: impl Into<String>, value: Option<String>) -> Self {
        self.options.extra_args.insert(key.into(), value);
        self
    }

    /// Set control protocol format
    pub fn control_protocol_format(mut self, format: ControlProtocolFormat) -> Self {
        self.options.control_protocol_format = format;
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

/// Result message (re-export for convenience)  
pub use Message::Result as ResultMessage;
/// System message (re-export for convenience)
pub use Message::System as SystemMessage;

/// Content block types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum ContentBlock {
    /// Text content
    Text(TextContent),
    /// Thinking content
    Thinking(ThinkingContent),
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

/// Thinking content block
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ThinkingContent {
    /// Thinking content
    pub thinking: String,
    /// Signature
    pub signature: String,
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

/// SDK Control Protocol - Interrupt request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SDKControlInterruptRequest {
    /// Subtype
    pub subtype: String,  // "interrupt"
}

/// SDK Control Protocol - Permission request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SDKControlPermissionRequest {
    /// Subtype
    pub subtype: String,  // "can_use_tool"
    /// Tool name
    pub tool_name: String,
    /// Tool input
    pub input: serde_json::Value,
    /// Permission suggestions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission_suggestions: Option<Vec<PermissionUpdate>>,
    /// Blocked path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocked_path: Option<String>,
}

/// SDK Control Protocol - Initialize request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SDKControlInitializeRequest {
    /// Subtype
    pub subtype: String,  // "initialize"
    /// Hooks configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hooks: Option<HashMap<String, serde_json::Value>>,
}

/// SDK Control Protocol - Set permission mode request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SDKControlSetPermissionModeRequest {
    /// Subtype
    pub subtype: String,  // "set_permission_mode"
    /// Permission mode
    pub mode: String,
}

/// SDK Hook callback request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SDKHookCallbackRequest {
    /// Subtype
    pub subtype: String,  // "hook_callback"
    /// Callback ID
    pub callback_id: String,
    /// Input data
    pub input: serde_json::Value,
    /// Tool use ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_use_id: Option<String>,
}

/// SDK Control Protocol - MCP message request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SDKControlMcpMessageRequest {
    /// Subtype
    pub subtype: String,  // "mcp_message"
    /// MCP server name
    pub mcp_server_name: String,
    /// Message to send
    pub message: serde_json::Value,
}

/// SDK Control Protocol request types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SDKControlRequest {
    /// Interrupt request
    #[serde(rename = "interrupt")]
    Interrupt(SDKControlInterruptRequest),
    /// Permission request
    #[serde(rename = "can_use_tool")]
    CanUseTool(SDKControlPermissionRequest),
    /// Initialize request
    #[serde(rename = "initialize")]
    Initialize(SDKControlInitializeRequest),
    /// Set permission mode
    #[serde(rename = "set_permission_mode")]
    SetPermissionMode(SDKControlSetPermissionModeRequest),
    /// Hook callback
    #[serde(rename = "hook_callback")]
    HookCallback(SDKHookCallbackRequest),
    /// MCP message
    #[serde(rename = "mcp_message")]
    McpMessage(SDKControlMcpMessageRequest),
}

/// Control request types (legacy, keeping for compatibility)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ControlRequest {
    /// Interrupt the current operation
    Interrupt {
        /// Request ID
        request_id: String,
    },
}

/// Control response types (legacy, keeping for compatibility)
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

        // Test Plan mode
        let plan_mode = PermissionMode::Plan;
        let plan_json = serde_json::to_string(&plan_mode).unwrap();
        assert_eq!(plan_json, r#""plan""#);

        let plan_deserialized: PermissionMode = serde_json::from_str(&plan_json).unwrap();
        assert_eq!(plan_deserialized, plan_mode);
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

    #[test]
    fn test_extra_args() {
        let mut extra_args = HashMap::new();
        extra_args.insert("custom-flag".to_string(), Some("value".to_string()));
        extra_args.insert("boolean-flag".to_string(), None);

        let options = ClaudeCodeOptions::builder()
            .extra_args(extra_args.clone())
            .add_extra_arg("another-flag", Some("another-value".to_string()))
            .build();

        assert_eq!(options.extra_args.len(), 3);
        assert_eq!(options.extra_args.get("custom-flag"), Some(&Some("value".to_string())));
        assert_eq!(options.extra_args.get("boolean-flag"), Some(&None));
        assert_eq!(options.extra_args.get("another-flag"), Some(&Some("another-value".to_string())));
    }

    #[test]
    fn test_thinking_content_serialization() {
        let thinking = ThinkingContent {
            thinking: "Let me think about this...".to_string(),
            signature: "sig123".to_string(),
        };

        let json = serde_json::to_string(&thinking).unwrap();
        assert!(json.contains(r#""thinking":"Let me think about this...""#));
        assert!(json.contains(r#""signature":"sig123""#));

        let deserialized: ThinkingContent = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.thinking, thinking.thinking);
        assert_eq!(deserialized.signature, thinking.signature);
    }
}
