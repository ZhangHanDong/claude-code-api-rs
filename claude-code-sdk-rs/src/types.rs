//! Type definitions for the Claude Code SDK
//!
//! This module contains all the core types used throughout the SDK,
//! including messages, configuration options, and content blocks.

#![allow(missing_docs)]
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Callback type for stderr output handling.
/// Called with each line of stderr output from the CLI.
pub type StderrCallback = Arc<dyn Fn(&str) + Send + Sync>;

/// Permission mode for tool execution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PermissionMode {
    /// Default mode - CLI prompts for dangerous tools
    #[default]
    Default,
    /// Auto-accept file edits
    AcceptEdits,
    /// Plan mode - for planning tasks
    Plan,
    /// Allow all tools without prompting (use with caution)
    BypassPermissions,
}

// ============================================================================
// SDK Beta Features (matching Python SDK v0.1.12+)
// ============================================================================

/// SDK Beta features - see <https://docs.anthropic.com/en/api/beta-headers>
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SdkBeta {
    /// Extended context window (1M tokens)
    #[serde(rename = "context-1m-2025-08-07")]
    Context1M,
}

impl std::fmt::Display for SdkBeta {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SdkBeta::Context1M => write!(f, "context-1m-2025-08-07"),
        }
    }
}

// ============================================================================
// Tools Configuration (matching Python SDK v0.1.12+)
// ============================================================================

/// Tools configuration for controlling available tools
///
/// This controls the base set of tools available to Claude, distinct from
/// `allowed_tools` which only controls auto-approval permissions.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolsConfig {
    /// List of specific tool names to enable
    /// Example: `["Read", "Edit", "Bash"]`
    List(Vec<String>),
    /// Preset-based tools configuration
    Preset(ToolsPreset),
}

/// Tools preset configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsPreset {
    /// Type identifier (always "preset")
    #[serde(rename = "type")]
    pub preset_type: String,
    /// Preset name (e.g., "claude_code")
    pub preset: String,
}

impl ToolsConfig {
    /// Create a new tools list
    pub fn list(tools: Vec<String>) -> Self {
        ToolsConfig::List(tools)
    }

    /// Create an empty tools list (disables all built-in tools)
    pub fn none() -> Self {
        ToolsConfig::List(vec![])
    }

    /// Create the claude_code preset
    pub fn claude_code_preset() -> Self {
        ToolsConfig::Preset(ToolsPreset {
            preset_type: "preset".to_string(),
            preset: "claude_code".to_string(),
        })
    }
}

// ============================================================================
// Sandbox Configuration (matching Python SDK)
// ============================================================================

/// Network configuration for sandbox
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SandboxNetworkConfig {
    /// Unix socket paths accessible in sandbox (e.g., SSH agents)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_unix_sockets: Option<Vec<String>>,
    /// Allow all Unix sockets (less secure)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_all_unix_sockets: Option<bool>,
    /// Allow binding to localhost ports (macOS only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_local_binding: Option<bool>,
    /// HTTP proxy port if bringing your own proxy
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http_proxy_port: Option<u16>,
    /// SOCKS5 proxy port if bringing your own proxy
    #[serde(skip_serializing_if = "Option::is_none")]
    pub socks_proxy_port: Option<u16>,
}

/// Violations to ignore in sandbox
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SandboxIgnoreViolations {
    /// File paths for which violations should be ignored
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<Vec<String>>,
    /// Network hosts for which violations should be ignored
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<Vec<String>>,
}

/// Sandbox settings configuration
///
/// Controls how Claude Code sandboxes bash commands for filesystem
/// and network isolation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SandboxSettings {
    /// Enable bash sandboxing (macOS/Linux only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    /// Auto-approve bash commands when sandboxed (default: true)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_allow_bash_if_sandboxed: Option<bool>,
    /// Commands that should run outside the sandbox (e.g., ["git", "docker"])
    #[serde(skip_serializing_if = "Option::is_none")]
    pub excluded_commands: Option<Vec<String>>,
    /// Allow commands to bypass sandbox via dangerouslyDisableSandbox
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_unsandboxed_commands: Option<bool>,
    /// Network configuration for sandbox
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<SandboxNetworkConfig>,
    /// Violations to ignore
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ignore_violations: Option<SandboxIgnoreViolations>,
    /// Enable weaker sandbox for unprivileged Docker environments (Linux only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_weaker_nested_sandbox: Option<bool>,
}

// ============================================================================
// Plugin Configuration (matching Python SDK v0.1.5+)
// ============================================================================

/// SDK plugin configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum SdkPluginConfig {
    /// Local plugin loaded from filesystem path
    Local {
        /// Path to the plugin directory
        path: String,
    },
}

/// Control protocol format for sending messages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ControlProtocolFormat {
    /// Legacy format: {"type":"sdk_control_request","request":{...}}
    #[default]
    Legacy,
    /// New format: {"type":"control","control":{...}}
    Control,
    /// Auto-detect based on CLI capabilities (default to Legacy for compatibility)
    Auto,
}

/// MCP (Model Context Protocol) server configuration
#[derive(Clone)]
pub enum McpServerConfig {
    /// Standard I/O based MCP server
    Stdio {
        /// Command to execute
        command: String,
        /// Command arguments
        args: Option<Vec<String>>,
        /// Environment variables
        env: Option<HashMap<String, String>>,
    },
    /// Server-Sent Events based MCP server
    Sse {
        /// Server URL
        url: String,
        /// HTTP headers
        headers: Option<HashMap<String, String>>,
    },
    /// HTTP-based MCP server
    Http {
        /// Server URL
        url: String,
        /// HTTP headers
        headers: Option<HashMap<String, String>>,
    },
    /// SDK MCP server (in-process)
    Sdk {
        /// Server name
        name: String,
        /// Server instance
        instance: Arc<dyn std::any::Any + Send + Sync>,
    },
}

impl std::fmt::Debug for McpServerConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Stdio { command, args, env } => f
                .debug_struct("Stdio")
                .field("command", command)
                .field("args", args)
                .field("env", env)
                .finish(),
            Self::Sse { url, headers } => f
                .debug_struct("Sse")
                .field("url", url)
                .field("headers", headers)
                .finish(),
            Self::Http { url, headers } => f
                .debug_struct("Http")
                .field("url", url)
                .field("headers", headers)
                .finish(),
            Self::Sdk { name, .. } => f
                .debug_struct("Sdk")
                .field("name", name)
                .field("instance", &"<Arc<dyn Any>>")
                .finish(),
        }
    }
}

impl Serialize for McpServerConfig {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(None)?;

        match self {
            Self::Stdio { command, args, env } => {
                map.serialize_entry("type", "stdio")?;
                map.serialize_entry("command", command)?;
                if let Some(args) = args {
                    map.serialize_entry("args", args)?;
                }
                if let Some(env) = env {
                    map.serialize_entry("env", env)?;
                }
            },
            Self::Sse { url, headers } => {
                map.serialize_entry("type", "sse")?;
                map.serialize_entry("url", url)?;
                if let Some(headers) = headers {
                    map.serialize_entry("headers", headers)?;
                }
            },
            Self::Http { url, headers } => {
                map.serialize_entry("type", "http")?;
                map.serialize_entry("url", url)?;
                if let Some(headers) = headers {
                    map.serialize_entry("headers", headers)?;
                }
            },
            Self::Sdk { name, .. } => {
                map.serialize_entry("type", "sdk")?;
                map.serialize_entry("name", name)?;
            },
        }

        map.end()
    }
}

impl<'de> Deserialize<'de> for McpServerConfig {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(tag = "type", rename_all = "lowercase")]
        enum McpServerConfigHelper {
            Stdio {
                command: String,
                #[serde(skip_serializing_if = "Option::is_none")]
                args: Option<Vec<String>>,
                #[serde(skip_serializing_if = "Option::is_none")]
                env: Option<HashMap<String, String>>,
            },
            Sse {
                url: String,
                #[serde(skip_serializing_if = "Option::is_none")]
                headers: Option<HashMap<String, String>>,
            },
            Http {
                url: String,
                #[serde(skip_serializing_if = "Option::is_none")]
                headers: Option<HashMap<String, String>>,
            },
        }

        let helper = McpServerConfigHelper::deserialize(deserializer)?;
        Ok(match helper {
            McpServerConfigHelper::Stdio { command, args, env } => {
                McpServerConfig::Stdio { command, args, env }
            },
            McpServerConfigHelper::Sse { url, headers } => McpServerConfig::Sse { url, headers },
            McpServerConfigHelper::Http { url, headers } => McpServerConfig::Http { url, headers },
        })
    }
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

// ============================================================================
// Hook Input Types (Strongly-typed hook inputs for type safety)
// ============================================================================

/// Base hook input fields present across many hook events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseHookInput {
    /// Session ID for this conversation
    pub session_id: String,
    /// Path to the transcript file
    pub transcript_path: String,
    /// Current working directory
    pub cwd: String,
    /// Permission mode (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission_mode: Option<String>,
}

/// Input data for PreToolUse hook events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreToolUseHookInput {
    /// Session ID for this conversation
    pub session_id: String,
    /// Path to the transcript file
    pub transcript_path: String,
    /// Current working directory
    pub cwd: String,
    /// Permission mode (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission_mode: Option<String>,
    /// Name of the tool being used
    pub tool_name: String,
    /// Input parameters for the tool
    pub tool_input: serde_json::Value,
}

/// Input data for PostToolUse hook events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostToolUseHookInput {
    /// Session ID for this conversation
    pub session_id: String,
    /// Path to the transcript file
    pub transcript_path: String,
    /// Current working directory
    pub cwd: String,
    /// Permission mode (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission_mode: Option<String>,
    /// Name of the tool that was used
    pub tool_name: String,
    /// Input parameters that were passed to the tool
    pub tool_input: serde_json::Value,
    /// Response from the tool execution
    pub tool_response: serde_json::Value,
}

/// Input data for UserPromptSubmit hook events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPromptSubmitHookInput {
    /// Session ID for this conversation
    pub session_id: String,
    /// Path to the transcript file
    pub transcript_path: String,
    /// Current working directory
    pub cwd: String,
    /// Permission mode (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission_mode: Option<String>,
    /// The prompt submitted by the user
    pub prompt: String,
}

/// Input data for Stop hook events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StopHookInput {
    /// Session ID for this conversation
    pub session_id: String,
    /// Path to the transcript file
    pub transcript_path: String,
    /// Current working directory
    pub cwd: String,
    /// Permission mode (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission_mode: Option<String>,
    /// Whether stop hook is active
    pub stop_hook_active: bool,
}

/// Input data for SubagentStop hook events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubagentStopHookInput {
    /// Session ID for this conversation
    pub session_id: String,
    /// Path to the transcript file
    pub transcript_path: String,
    /// Current working directory
    pub cwd: String,
    /// Permission mode (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission_mode: Option<String>,
    /// Whether stop hook is active
    pub stop_hook_active: bool,
}

/// Input data for PreCompact hook events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreCompactHookInput {
    /// Session ID for this conversation
    pub session_id: String,
    /// Path to the transcript file
    pub transcript_path: String,
    /// Current working directory
    pub cwd: String,
    /// Permission mode (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission_mode: Option<String>,
    /// Trigger type: "manual" or "auto"
    pub trigger: String,
    /// Custom instructions for compaction (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_instructions: Option<String>,
}

/// Union type for all hook inputs (discriminated by hook_event_name)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "hook_event_name")]
pub enum HookInput {
    /// PreToolUse hook input
    #[serde(rename = "PreToolUse")]
    PreToolUse(PreToolUseHookInput),
    /// PostToolUse hook input
    #[serde(rename = "PostToolUse")]
    PostToolUse(PostToolUseHookInput),
    /// UserPromptSubmit hook input
    #[serde(rename = "UserPromptSubmit")]
    UserPromptSubmit(UserPromptSubmitHookInput),
    /// Stop hook input
    #[serde(rename = "Stop")]
    Stop(StopHookInput),
    /// SubagentStop hook input
    #[serde(rename = "SubagentStop")]
    SubagentStop(SubagentStopHookInput),
    /// PreCompact hook input
    #[serde(rename = "PreCompact")]
    PreCompact(PreCompactHookInput),
}

// ============================================================================
// Hook Output Types (Strongly-typed hook outputs for type safety)
// ============================================================================

/// Async hook output for deferred execution
///
/// When a hook returns this output, the hook execution is deferred and
/// Claude continues without waiting for the hook to complete.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsyncHookJSONOutput {
    /// Must be true to indicate async execution
    #[serde(rename = "async")]
    pub async_: bool,
    /// Optional timeout in milliseconds for async operation
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "asyncTimeout")]
    pub async_timeout: Option<u32>,
}

/// Synchronous hook output with control and decision fields
///
/// This defines the structure for hook callbacks to control execution and provide
/// feedback to Claude.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SyncHookJSONOutput {
    // Common control fields
    /// Whether Claude should proceed after hook execution (default: true)
    #[serde(rename = "continue", skip_serializing_if = "Option::is_none")]
    pub continue_: Option<bool>,
    /// Hide stdout from transcript mode (default: false)
    #[serde(rename = "suppressOutput", skip_serializing_if = "Option::is_none")]
    pub suppress_output: Option<bool>,
    /// Message shown when continue is false
    #[serde(rename = "stopReason", skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<String>,

    // Decision fields
    /// Set to "block" to indicate blocking behavior
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decision: Option<String>, // "block" or "approve" (deprecated)
    /// Warning message displayed to the user
    #[serde(rename = "systemMessage", skip_serializing_if = "Option::is_none")]
    pub system_message: Option<String>,
    /// Feedback message for Claude about the decision
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,

    // Hook-specific outputs
    /// Event-specific controls (e.g., permissionDecision for PreToolUse)
    #[serde(rename = "hookSpecificOutput", skip_serializing_if = "Option::is_none")]
    pub hook_specific_output: Option<HookSpecificOutput>,
}

/// Union type for hook outputs
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum HookJSONOutput {
    /// Async hook output (deferred execution)
    Async(AsyncHookJSONOutput),
    /// Sync hook output (immediate execution)
    Sync(SyncHookJSONOutput),
}

/// Hook-specific output for PreToolUse events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreToolUseHookSpecificOutput {
    /// Permission decision: "allow", "deny", or "ask"
    #[serde(rename = "permissionDecision", skip_serializing_if = "Option::is_none")]
    pub permission_decision: Option<String>,
    /// Reason for the permission decision
    #[serde(
        rename = "permissionDecisionReason",
        skip_serializing_if = "Option::is_none"
    )]
    pub permission_decision_reason: Option<String>,
    /// Updated input parameters for the tool
    #[serde(rename = "updatedInput", skip_serializing_if = "Option::is_none")]
    pub updated_input: Option<serde_json::Value>,
}

/// Hook-specific output for PostToolUse events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostToolUseHookSpecificOutput {
    /// Additional context to provide to Claude
    #[serde(rename = "additionalContext", skip_serializing_if = "Option::is_none")]
    pub additional_context: Option<String>,
}

/// Hook-specific output for UserPromptSubmit events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPromptSubmitHookSpecificOutput {
    /// Additional context to provide to Claude
    #[serde(rename = "additionalContext", skip_serializing_if = "Option::is_none")]
    pub additional_context: Option<String>,
}

/// Hook-specific output for SessionStart events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStartHookSpecificOutput {
    /// Additional context to provide to Claude
    #[serde(rename = "additionalContext", skip_serializing_if = "Option::is_none")]
    pub additional_context: Option<String>,
}

/// Union type for hook-specific outputs (discriminated by hookEventName)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "hookEventName")]
pub enum HookSpecificOutput {
    /// PreToolUse-specific output
    #[serde(rename = "PreToolUse")]
    PreToolUse(PreToolUseHookSpecificOutput),
    /// PostToolUse-specific output
    #[serde(rename = "PostToolUse")]
    PostToolUse(PostToolUseHookSpecificOutput),
    /// UserPromptSubmit-specific output
    #[serde(rename = "UserPromptSubmit")]
    UserPromptSubmit(UserPromptSubmitHookSpecificOutput),
    /// SessionStart-specific output
    #[serde(rename = "SessionStart")]
    SessionStart(SessionStartHookSpecificOutput),
}

// ============================================================================
// Hook Callback Trait (Updated for strong typing)
// ============================================================================

/// Hook callback trait with strongly-typed inputs and outputs
///
/// This trait is used to implement custom hook callbacks that can intercept
/// and modify Claude's behavior at various points in the conversation.
#[async_trait]
pub trait HookCallback: Send + Sync {
    /// Execute the hook with strongly-typed input and output
    ///
    /// # Arguments
    ///
    /// * `input` - Strongly-typed hook input (discriminated union)
    /// * `tool_use_id` - Optional tool use identifier
    /// * `context` - Hook context with abort signal support
    ///
    /// # Returns
    ///
    /// A `HookJSONOutput` that controls Claude's behavior
    async fn execute(
        &self,
        input: &HookInput,
        tool_use_id: Option<&str>,
        context: &HookContext,
    ) -> Result<HookJSONOutput, crate::errors::SdkError>;
}

/// Legacy hook callback trait for backward compatibility
///
/// This trait is deprecated and will be removed in v0.4.0.
/// Please migrate to the new `HookCallback` trait with strong typing.
#[deprecated(
    since = "0.3.0",
    note = "Use the new HookCallback trait with HookInput/HookJSONOutput instead"
)]
#[allow(dead_code)]
#[async_trait]
pub trait HookCallbackLegacy: Send + Sync {
    /// Execute the hook with JSON values (legacy)
    async fn execute_legacy(
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

/// Setting source for configuration loading
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SettingSource {
    /// User-level settings
    User,
    /// Project-level settings
    Project,
    /// Local settings
    Local,
}

/// Agent definition for programmatic agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDefinition {
    /// Agent description
    pub description: String,
    /// Agent prompt
    pub prompt: String,
    /// Allowed tools for this agent
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<String>>,
    /// Model to use
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

/// System prompt configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SystemPrompt {
    /// Simple string prompt
    String(String),
    /// Preset-based prompt with optional append
    Preset {
        #[serde(rename = "type")]
        preset_type: String, // "preset"
        preset: String, // e.g., "claude_code"
        #[serde(skip_serializing_if = "Option::is_none")]
        append: Option<String>,
    },
}

/// Configuration options for Claude Code SDK
#[derive(Clone, Default)]
pub struct ClaudeCodeOptions {
    /// System prompt configuration (simplified in v0.1.12+)
    /// Can be either a string or a preset configuration
    /// Replaces the old system_prompt and append_system_prompt fields
    pub system_prompt_v2: Option<SystemPrompt>,
    /// \[DEPRECATED\] System prompt to prepend to all messages
    /// Use system_prompt_v2 instead
    #[deprecated(since = "0.1.12", note = "Use system_prompt_v2 instead")]
    pub system_prompt: Option<String>,
    /// \[DEPRECATED\] Additional system prompt to append
    /// Use system_prompt_v2 instead
    #[deprecated(since = "0.1.12", note = "Use system_prompt_v2 instead")]
    pub append_system_prompt: Option<String>,
    /// List of allowed tools (auto-approval permissions only)
    ///
    /// **IMPORTANT**: This only controls which tool invocations are auto-approved
    /// (bypass permission prompts). It does NOT disable or restrict which tools
    /// the AI can use. Use `disallowed_tools` to completely disable tools.
    ///
    /// Example: `allowed_tools: vec!["Bash(git:*)".to_string()]` allows auto-approval
    /// for git commands in Bash, but doesn't prevent AI from using other tools.
    pub allowed_tools: Vec<String>,
    /// List of disallowed tools (completely disabled)
    ///
    /// **IMPORTANT**: This completely disables the specified tools. The AI will
    /// not be able to use these tools at all. Use this to restrict which tools
    /// the AI has access to.
    ///
    /// Example: `disallowed_tools: vec!["Bash".to_string(), "WebSearch".to_string()]`
    /// prevents the AI from using Bash or WebSearch tools entirely.
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
    /// Maximum output tokens per response (1-32000, overrides CLAUDE_CODE_MAX_OUTPUT_TOKENS env var)
    pub max_output_tokens: Option<u32>,
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
    /// Include partial assistant messages in streaming output
    pub include_partial_messages: bool,
    /// Tool permission callback
    pub can_use_tool: Option<Arc<dyn CanUseTool>>,
    /// Hook configurations
    pub hooks: Option<HashMap<String, Vec<HookMatcher>>>,
    /// Control protocol format (defaults to Legacy for compatibility)
    pub control_protocol_format: ControlProtocolFormat,

    // ========== Phase 2 Enhancements ==========
    /// Setting sources to load (user, project, local)
    /// When None, no filesystem settings are loaded (matches Python SDK v0.1.0 behavior)
    pub setting_sources: Option<Vec<SettingSource>>,
    /// Fork session when resuming instead of continuing
    /// When true, creates a new branch from the resumed session
    pub fork_session: bool,
    /// Programmatic agent definitions
    /// Define agents inline without filesystem dependencies
    pub agents: Option<HashMap<String, AgentDefinition>>,
    /// CLI channel buffer size for internal communication channels
    /// Controls the size of message, control, and stdin buffers (default: 100)
    /// Increase for high-throughput scenarios to prevent message lag
    pub cli_channel_buffer_size: Option<usize>,

    // ========== Phase 3 Enhancements (Python SDK v0.1.12+ sync) ==========
    /// Tools configuration for controlling available tools
    ///
    /// This controls the base set of tools available to Claude, distinct from
    /// `allowed_tools` which only controls auto-approval permissions.
    ///
    /// # Examples
    /// ```rust
    /// use nexus_claude::{ClaudeCodeOptions, ToolsConfig};
    ///
    /// // Enable specific tools only
    /// let options = ClaudeCodeOptions::builder()
    ///     .tools(ToolsConfig::list(vec!["Read".into(), "Edit".into()]))
    ///     .build();
    ///
    /// // Disable all built-in tools
    /// let options = ClaudeCodeOptions::builder()
    ///     .tools(ToolsConfig::none())
    ///     .build();
    ///
    /// // Use claude_code preset
    /// let options = ClaudeCodeOptions::builder()
    ///     .tools(ToolsConfig::claude_code_preset())
    ///     .build();
    /// ```
    pub tools: Option<ToolsConfig>,
    /// SDK beta features to enable
    /// See <https://docs.anthropic.com/en/api/beta-headers>
    pub betas: Vec<SdkBeta>,
    /// Maximum spending limit in USD for the session
    /// When exceeded, the session will automatically terminate
    pub max_budget_usd: Option<f64>,
    /// Fallback model to use when primary model is unavailable
    pub fallback_model: Option<String>,
    /// Output format for structured outputs
    /// Example: `{"type": "json_schema", "schema": {"type": "object", "properties": {...}}}`
    pub output_format: Option<serde_json::Value>,
    /// Enable file checkpointing to track file changes during the session
    /// When enabled, files can be rewound to their state at any user message
    /// using `ClaudeSDKClient::rewind_files()`
    pub enable_file_checkpointing: bool,
    /// Sandbox configuration for bash command isolation
    /// Filesystem and network restrictions are derived from permission rules
    pub sandbox: Option<SandboxSettings>,
    /// Plugin configurations for custom plugins
    pub plugins: Vec<SdkPluginConfig>,
    /// Run the CLI subprocess as a specific OS user (Unix-only).
    ///
    /// This matches Python SDK behavior (`anyio.open_process(user=...)`).
    ///
    /// - Supported on Unix platforms only (non-Unix returns `SdkError::NotSupported`)
    /// - Typically requires elevated privileges to switch users
    /// - Accepts a username (e.g. `"nobody"`) or a numeric uid string (e.g. `"1000"`)
    pub user: Option<String>,
    /// Stderr callback (alternative to debug_stderr)
    /// Called with each line of stderr output from the CLI
    pub stderr_callback: Option<StderrCallback>,
    /// Automatically download Claude Code CLI if not found
    ///
    /// When enabled, the SDK will automatically download and cache the Claude Code
    /// CLI binary if it's not found in the system PATH or common installation locations.
    ///
    /// The CLI is cached in:
    /// - macOS: `~/Library/Caches/cc-sdk/cli/`
    /// - Linux: `~/.cache/cc-sdk/cli/`
    /// - Windows: `%LOCALAPPDATA%\cc-sdk\cli\`
    ///
    /// # Example
    ///
    /// ```rust
    /// # use nexus_claude::ClaudeCodeOptions;
    /// let options = ClaudeCodeOptions::builder()
    ///     .auto_download_cli(true)
    ///     .build();
    /// ```
    pub auto_download_cli: bool,

    // ========== Memory System Options ==========
    /// Enable persistent memory for cross-conversation context
    ///
    /// When enabled, the SDK will:
    /// 1. Store messages in Meilisearch for later retrieval
    /// 2. Retrieve relevant context before each request
    /// 3. Inject context into the system prompt
    ///
    /// Requires the `memory` feature and a running Meilisearch instance.
    /// Default: false
    pub memory_enabled: bool,

    /// Minimum relevance score for context injection (0.0-1.0)
    ///
    /// Messages with scores below this threshold are not included in context.
    /// Higher values = more selective, fewer irrelevant results.
    /// Default: 0.3
    pub memory_threshold: Option<f64>,

    /// Maximum number of context items to inject per request
    ///
    /// Limits how many previous messages are included in context.
    /// Default: 5
    pub max_context_items: Option<usize>,

    /// Token budget for injected context (~4 chars per token)
    ///
    /// Limits the total size of injected context to avoid overwhelming the prompt.
    /// Default: 2000
    pub memory_token_budget: Option<usize>,
}

impl std::fmt::Debug for ClaudeCodeOptions {
    #[allow(deprecated)]
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
            .field("max_output_tokens", &self.max_output_tokens)
            .field("model", &self.model)
            .field("cwd", &self.cwd)
            .field("continue_conversation", &self.continue_conversation)
            .field("resume", &self.resume)
            .field(
                "permission_prompt_tool_name",
                &self.permission_prompt_tool_name,
            )
            .field("settings", &self.settings)
            .field("add_dirs", &self.add_dirs)
            .field("extra_args", &self.extra_args)
            .field("env", &self.env)
            .field("debug_stderr", &self.debug_stderr.is_some())
            .field("include_partial_messages", &self.include_partial_messages)
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
    #[allow(deprecated)]
    pub fn system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.options.system_prompt = Some(prompt.into());
        self
    }

    /// Set append system prompt
    #[allow(deprecated)]
    pub fn append_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.options.append_system_prompt = Some(prompt.into());
        self
    }

    /// Set allowed tools (auto-approval permissions only)
    ///
    /// **IMPORTANT**: This only controls which tool invocations bypass permission
    /// prompts. It does NOT disable or restrict which tools the AI can use.
    /// To completely disable tools, use `disallowed_tools()` instead.
    ///
    /// Example: `vec!["Bash(git:*)".to_string()]` auto-approves git commands.
    pub fn allowed_tools(mut self, tools: Vec<String>) -> Self {
        self.options.allowed_tools = tools;
        self
    }

    /// Add a single allowed tool (auto-approval permission)
    ///
    /// See `allowed_tools()` for important usage notes.
    pub fn allow_tool(mut self, tool: impl Into<String>) -> Self {
        self.options.allowed_tools.push(tool.into());
        self
    }

    /// Set disallowed tools (completely disabled)
    ///
    /// **IMPORTANT**: This completely disables the specified tools. The AI will
    /// not be able to use these tools at all. This is the correct way to restrict
    /// which tools the AI has access to.
    ///
    /// Example: `vec!["Bash".to_string(), "WebSearch".to_string()]` prevents
    /// the AI from using Bash or WebSearch entirely.
    pub fn disallowed_tools(mut self, tools: Vec<String>) -> Self {
        self.options.disallowed_tools = tools;
        self
    }

    /// Add a single disallowed tool (completely disabled)
    ///
    /// See `disallowed_tools()` for important usage notes.
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

    /// Set all MCP servers from a map
    pub fn mcp_servers(mut self, servers: HashMap<String, McpServerConfig>) -> Self {
        self.options.mcp_servers = servers;
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

    /// Set max output tokens (1-32000, overrides CLAUDE_CODE_MAX_OUTPUT_TOKENS env var)
    pub fn max_output_tokens(mut self, tokens: u32) -> Self {
        self.options.max_output_tokens = Some(tokens.clamp(1, 32000));
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

    /// Include partial assistant messages in streaming output
    pub fn include_partial_messages(mut self, include: bool) -> Self {
        self.options.include_partial_messages = include;
        self
    }

    /// Enable fork_session behavior
    pub fn fork_session(mut self, fork: bool) -> Self {
        self.options.fork_session = fork;
        self
    }

    /// Set setting sources
    pub fn setting_sources(mut self, sources: Vec<SettingSource>) -> Self {
        self.options.setting_sources = Some(sources);
        self
    }

    /// Define programmatic agents
    pub fn agents(mut self, agents: HashMap<String, AgentDefinition>) -> Self {
        self.options.agents = Some(agents);
        self
    }

    /// Set CLI channel buffer size
    ///
    /// Controls the size of internal communication channels (message, control, stdin buffers).
    /// Default is 100. Increase for high-throughput scenarios to prevent message lag.
    ///
    /// # Arguments
    ///
    /// * `size` - Buffer size (number of messages that can be queued)
    ///
    /// # Example
    ///
    /// ```rust
    /// # use nexus_claude::ClaudeCodeOptions;
    /// let options = ClaudeCodeOptions::builder()
    ///     .cli_channel_buffer_size(500)
    ///     .build();
    /// ```
    pub fn cli_channel_buffer_size(mut self, size: usize) -> Self {
        self.options.cli_channel_buffer_size = Some(size);
        self
    }

    // ========== Phase 3 Builder Methods (Python SDK v0.1.12+ sync) ==========

    /// Set tools configuration
    ///
    /// Controls the base set of tools available to Claude. This is distinct from
    /// `allowed_tools` which only controls auto-approval permissions.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use nexus_claude::{ClaudeCodeOptions, ToolsConfig};
    /// // Enable specific tools only
    /// let options = ClaudeCodeOptions::builder()
    ///     .tools(ToolsConfig::list(vec!["Read".into(), "Edit".into()]))
    ///     .build();
    /// ```
    pub fn tools(mut self, config: ToolsConfig) -> Self {
        self.options.tools = Some(config);
        self
    }

    /// Add SDK beta features
    ///
    /// Enable Anthropic API beta features like extended context window.
    pub fn betas(mut self, betas: Vec<SdkBeta>) -> Self {
        self.options.betas = betas;
        self
    }

    /// Add a single SDK beta feature
    pub fn add_beta(mut self, beta: SdkBeta) -> Self {
        self.options.betas.push(beta);
        self
    }

    /// Set maximum spending limit in USD
    ///
    /// When the budget is exceeded, the session will automatically terminate.
    pub fn max_budget_usd(mut self, budget: f64) -> Self {
        self.options.max_budget_usd = Some(budget);
        self
    }

    /// Set fallback model
    ///
    /// Used when the primary model is unavailable.
    pub fn fallback_model(mut self, model: impl Into<String>) -> Self {
        self.options.fallback_model = Some(model.into());
        self
    }

    /// Set output format for structured outputs
    ///
    /// Enables JSON schema validation for Claude's responses.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use nexus_claude::ClaudeCodeOptions;
    /// let options = ClaudeCodeOptions::builder()
    ///     .output_format(serde_json::json!({
    ///         "type": "json_schema",
    ///         "schema": {
    ///             "type": "object",
    ///             "properties": {
    ///                 "answer": {"type": "string"}
    ///             }
    ///         }
    ///     }))
    ///     .build();
    /// ```
    pub fn output_format(mut self, format: serde_json::Value) -> Self {
        self.options.output_format = Some(format);
        self
    }

    /// Enable file checkpointing
    ///
    /// When enabled, file changes are tracked and can be rewound to any
    /// user message using `ClaudeSDKClient::rewind_files()`.
    pub fn enable_file_checkpointing(mut self, enable: bool) -> Self {
        self.options.enable_file_checkpointing = enable;
        self
    }

    /// Set sandbox configuration
    ///
    /// Controls bash command sandboxing for filesystem and network isolation.
    pub fn sandbox(mut self, settings: SandboxSettings) -> Self {
        self.options.sandbox = Some(settings);
        self
    }

    /// Set plugin configurations
    pub fn plugins(mut self, plugins: Vec<SdkPluginConfig>) -> Self {
        self.options.plugins = plugins;
        self
    }

    /// Add a single plugin
    pub fn add_plugin(mut self, plugin: SdkPluginConfig) -> Self {
        self.options.plugins.push(plugin);
        self
    }

    /// Set user identifier
    pub fn user(mut self, user: impl Into<String>) -> Self {
        self.options.user = Some(user.into());
        self
    }

    /// Set stderr callback
    ///
    /// Called with each line of stderr output from the CLI.
    pub fn stderr_callback(mut self, callback: Arc<dyn Fn(&str) + Send + Sync>) -> Self {
        self.options.stderr_callback = Some(callback);
        self
    }

    /// Enable automatic CLI download
    ///
    /// When enabled, the SDK will automatically download and cache the Claude Code
    /// CLI binary if it's not found in the system PATH or common installation locations.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use nexus_claude::ClaudeCodeOptions;
    /// let options = ClaudeCodeOptions::builder()
    ///     .auto_download_cli(true)
    ///     .build();
    /// ```
    pub fn auto_download_cli(mut self, enable: bool) -> Self {
        self.options.auto_download_cli = enable;
        self
    }

    // ========== Memory System Options ==========

    /// Enable persistent memory for cross-conversation context
    ///
    /// When enabled, the SDK will store and retrieve context across sessions.
    /// Requires the `memory` feature and a running Meilisearch instance.
    pub fn memory_enabled(mut self, enabled: bool) -> Self {
        self.options.memory_enabled = enabled;
        self
    }

    /// Set the minimum relevance score for context injection (0.0-1.0)
    ///
    /// Messages with scores below this threshold are not included.
    pub fn memory_threshold(mut self, threshold: f64) -> Self {
        self.options.memory_threshold = Some(threshold.clamp(0.0, 1.0));
        self
    }

    /// Set the maximum number of context items to inject per request
    pub fn max_context_items(mut self, max: usize) -> Self {
        self.options.max_context_items = Some(max);
        self
    }

    /// Set the token budget for injected context (~4 chars per token)
    pub fn memory_token_budget(mut self, budget: usize) -> Self {
        self.options.memory_token_budget = Some(budget);
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
        /// Structured output (when output_format is set)
        /// Contains the validated JSON response matching the schema
        #[serde(skip_serializing_if = "Option::is_none", alias = "structuredOutput")]
        structured_output: Option<serde_json::Value>,
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
    pub subtype: String, // "interrupt"
}

/// SDK Control Protocol - Permission request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SDKControlPermissionRequest {
    /// Subtype
    pub subtype: String, // "can_use_tool"
    /// Tool name
    #[serde(alias = "toolName")]
    pub tool_name: String,
    /// Tool input
    pub input: serde_json::Value,
    /// Permission suggestions
    #[serde(
        skip_serializing_if = "Option::is_none",
        alias = "permissionSuggestions"
    )]
    pub permission_suggestions: Option<Vec<PermissionUpdate>>,
    /// Blocked path
    #[serde(skip_serializing_if = "Option::is_none", alias = "blockedPath")]
    pub blocked_path: Option<String>,
}

/// SDK Control Protocol - Initialize request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SDKControlInitializeRequest {
    /// Subtype
    pub subtype: String, // "initialize"
    /// Hooks configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hooks: Option<HashMap<String, serde_json::Value>>,
}

/// SDK Control Protocol - Set permission mode request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SDKControlSetPermissionModeRequest {
    /// Subtype
    pub subtype: String, // "set_permission_mode"
    /// Permission mode
    pub mode: String,
}

/// SDK Control Protocol - Set model request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SDKControlSetModelRequest {
    /// Subtype
    pub subtype: String, // "set_model"
    /// Model to set (None to clear)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

/// SDK Hook callback request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SDKHookCallbackRequest {
    /// Subtype
    pub subtype: String, // "hook_callback"
    /// Callback ID
    #[serde(alias = "callbackId")]
    pub callback_id: String,
    /// Input data
    pub input: serde_json::Value,
    /// Tool use ID
    #[serde(skip_serializing_if = "Option::is_none", alias = "toolUseId")]
    pub tool_use_id: Option<String>,
}

/// SDK Control Protocol - MCP message request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SDKControlMcpMessageRequest {
    /// Subtype
    pub subtype: String, // "mcp_message"
    /// MCP server name
    #[serde(
        rename = "server_name",
        alias = "mcpServerName",
        alias = "mcp_server_name"
    )]
    pub mcp_server_name: String,
    /// Message to send
    pub message: serde_json::Value,
}

/// SDK Control Protocol - Rewind files request (Python SDK v0.1.14+)
///
/// Rewinds tracked files to their state at a specific user message.
/// Requires `enable_file_checkpointing` to be enabled.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SDKControlRewindFilesRequest {
    /// Subtype (always "rewind_files")
    pub subtype: String,
    /// UUID of the user message to rewind to
    #[serde(alias = "userMessageId")]
    pub user_message_id: String,
}

impl SDKControlRewindFilesRequest {
    /// Create a new rewind files request
    pub fn new(user_message_id: impl Into<String>) -> Self {
        Self {
            subtype: "rewind_files".to_string(),
            user_message_id: user_message_id.into(),
        }
    }
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
    /// Set model
    #[serde(rename = "set_model")]
    SetModel(SDKControlSetModelRequest),
    /// Hook callback
    #[serde(rename = "hook_callback")]
    HookCallback(SDKHookCallbackRequest),
    /// MCP message
    #[serde(rename = "mcp_message")]
    McpMessage(SDKControlMcpMessageRequest),
    /// Rewind files (Python SDK v0.1.14+)
    #[serde(rename = "rewind_files")]
    RewindFiles(SDKControlRewindFilesRequest),
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
    #[allow(deprecated)]
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
        assert_eq!(
            options.extra_args.get("custom-flag"),
            Some(&Some("value".to_string()))
        );
        assert_eq!(options.extra_args.get("boolean-flag"), Some(&None));
        assert_eq!(
            options.extra_args.get("another-flag"),
            Some(&Some("another-value".to_string()))
        );
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

    // ============== v0.4.0 New Feature Tests ==============

    #[test]
    fn test_tools_config_list_serialization() {
        let tools = ToolsConfig::List(vec![
            "Read".to_string(),
            "Write".to_string(),
            "Bash".to_string(),
        ]);
        let json = serde_json::to_string(&tools).unwrap();

        // List variant serializes as JSON array
        assert!(json.contains("Read"));
        assert!(json.contains("Write"));
        assert!(json.contains("Bash"));

        let deserialized: ToolsConfig = serde_json::from_str(&json).unwrap();
        match deserialized {
            ToolsConfig::List(list) => {
                assert_eq!(list.len(), 3);
                assert!(list.contains(&"Read".to_string()));
            },
            _ => panic!("Expected List variant"),
        }
    }

    #[test]
    fn test_tools_config_preset_serialization() {
        // Test claude_code preset using the helper method
        let preset = ToolsConfig::claude_code_preset();
        let json = serde_json::to_string(&preset).unwrap();
        assert!(json.contains("preset"));
        assert!(json.contains("claude_code"));

        // Test Preset variant with custom values
        let custom_preset = ToolsConfig::Preset(ToolsPreset {
            preset_type: "preset".to_string(),
            preset: "custom".to_string(),
        });
        let json = serde_json::to_string(&custom_preset).unwrap();
        assert!(json.contains("custom"));

        // Test deserialization
        let deserialized: ToolsConfig = serde_json::from_str(&json).unwrap();
        match deserialized {
            ToolsConfig::Preset(p) => assert_eq!(p.preset, "custom"),
            _ => panic!("Expected Preset variant"),
        }
    }

    #[test]
    fn test_tools_config_helper_methods() {
        // Test list() helper
        let tools = ToolsConfig::list(vec!["Read".to_string(), "Write".to_string()]);
        match tools {
            ToolsConfig::List(list) => assert_eq!(list.len(), 2),
            _ => panic!("Expected List variant"),
        }

        // Test none() helper (empty list)
        let empty = ToolsConfig::none();
        match empty {
            ToolsConfig::List(list) => assert!(list.is_empty()),
            _ => panic!("Expected empty List variant"),
        }

        // Test claude_code_preset() helper
        let preset = ToolsConfig::claude_code_preset();
        match preset {
            ToolsConfig::Preset(p) => {
                assert_eq!(p.preset_type, "preset");
                assert_eq!(p.preset, "claude_code");
            },
            _ => panic!("Expected Preset variant"),
        }
    }

    #[test]
    fn test_sdk_beta_serialization() {
        let beta = SdkBeta::Context1M;
        let json = serde_json::to_string(&beta).unwrap();
        // The enum uses rename = "context-1m-2025-08-07"
        assert_eq!(json, r#""context-1m-2025-08-07""#);

        // Test Display trait
        let display = format!("{}", beta);
        assert_eq!(display, "context-1m-2025-08-07");

        // Test deserialization
        let deserialized: SdkBeta = serde_json::from_str(r#""context-1m-2025-08-07""#).unwrap();
        assert!(matches!(deserialized, SdkBeta::Context1M));
    }

    #[test]
    fn test_sandbox_settings_serialization() {
        let sandbox = SandboxSettings {
            enabled: Some(true),
            auto_allow_bash_if_sandboxed: Some(true),
            excluded_commands: Some(vec!["git".to_string(), "docker".to_string()]),
            allow_unsandboxed_commands: Some(false),
            network: Some(SandboxNetworkConfig {
                allow_unix_sockets: Some(vec!["/tmp/ssh-agent.sock".to_string()]),
                allow_all_unix_sockets: Some(false),
                allow_local_binding: Some(true),
                http_proxy_port: Some(8080),
                socks_proxy_port: Some(1080),
            }),
            ignore_violations: Some(SandboxIgnoreViolations {
                file: Some(vec!["/tmp".to_string(), "/var/log".to_string()]),
                network: Some(vec!["localhost".to_string()]),
            }),
            enable_weaker_nested_sandbox: Some(false),
        };

        let json = serde_json::to_string(&sandbox).unwrap();
        assert!(json.contains("enabled"));
        assert!(json.contains("autoAllowBashIfSandboxed")); // camelCase
        assert!(json.contains("excludedCommands"));
        assert!(json.contains("httpProxyPort"));
        assert!(json.contains("8080"));

        let deserialized: SandboxSettings = serde_json::from_str(&json).unwrap();
        assert!(deserialized.enabled.unwrap());
        assert!(deserialized.network.is_some());
        assert_eq!(
            deserialized.network.as_ref().unwrap().http_proxy_port,
            Some(8080)
        );
    }

    #[test]
    fn test_sandbox_network_config() {
        let config = SandboxNetworkConfig {
            allow_unix_sockets: Some(vec!["/run/user/1000/keyring/ssh".to_string()]),
            allow_all_unix_sockets: Some(false),
            allow_local_binding: Some(true),
            http_proxy_port: Some(3128),
            socks_proxy_port: Some(1080),
        };

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: SandboxNetworkConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.http_proxy_port, Some(3128));
        assert_eq!(deserialized.socks_proxy_port, Some(1080));
        assert_eq!(deserialized.allow_local_binding, Some(true));
    }

    #[test]
    fn test_sandbox_ignore_violations() {
        let violations = SandboxIgnoreViolations {
            file: Some(vec!["/tmp".to_string(), "/var/cache".to_string()]),
            network: Some(vec!["127.0.0.1".to_string(), "localhost".to_string()]),
        };

        let json = serde_json::to_string(&violations).unwrap();
        assert!(json.contains("file"));
        assert!(json.contains("/tmp"));

        let deserialized: SandboxIgnoreViolations = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.file.as_ref().unwrap().len(), 2);
        assert_eq!(deserialized.network.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn test_sandbox_settings_default() {
        let sandbox = SandboxSettings::default();
        assert!(sandbox.enabled.is_none());
        assert!(sandbox.network.is_none());
        assert!(sandbox.ignore_violations.is_none());
    }

    #[test]
    fn test_sdk_plugin_config_serialization() {
        let plugin = SdkPluginConfig::Local {
            path: "/path/to/plugin".to_string(),
        };

        let json = serde_json::to_string(&plugin).unwrap();
        assert!(json.contains("local")); // lowercase due to rename_all
        assert!(json.contains("/path/to/plugin"));

        let deserialized: SdkPluginConfig = serde_json::from_str(&json).unwrap();
        match deserialized {
            SdkPluginConfig::Local { path } => {
                assert_eq!(path, "/path/to/plugin");
            },
        }
    }

    #[test]
    fn test_sdk_control_rewind_files_request() {
        let request = SDKControlRewindFilesRequest {
            subtype: "rewind_files".to_string(),
            user_message_id: "msg_12345".to_string(),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("user_message_id"));
        assert!(json.contains("msg_12345"));
        assert!(json.contains("subtype"));
        assert!(json.contains("rewind_files"));

        let deserialized: SDKControlRewindFilesRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.user_message_id, "msg_12345");
        assert_eq!(deserialized.subtype, "rewind_files");
    }

    #[test]
    fn test_options_builder_with_new_fields() {
        let options = ClaudeCodeOptions::builder()
            .tools(ToolsConfig::claude_code_preset())
            .add_beta(SdkBeta::Context1M)
            .max_budget_usd(10.0)
            .fallback_model("claude-3-haiku")
            .output_format(serde_json::json!({"type": "object"}))
            .enable_file_checkpointing(true)
            .sandbox(SandboxSettings::default())
            .add_plugin(SdkPluginConfig::Local {
                path: "/plugin".to_string(),
            })
            .auto_download_cli(true)
            .build();

        // Verify tools
        assert!(options.tools.is_some());
        match options.tools.as_ref().unwrap() {
            ToolsConfig::Preset(preset) => assert_eq!(preset.preset, "claude_code"),
            _ => panic!("Expected Preset variant"),
        }

        // Verify betas
        assert_eq!(options.betas.len(), 1);
        assert!(matches!(options.betas[0], SdkBeta::Context1M));

        // Verify max_budget_usd
        assert_eq!(options.max_budget_usd, Some(10.0));

        // Verify fallback_model
        assert_eq!(options.fallback_model, Some("claude-3-haiku".to_string()));

        // Verify output_format
        assert!(options.output_format.is_some());

        // Verify enable_file_checkpointing
        assert!(options.enable_file_checkpointing);

        // Verify sandbox
        assert!(options.sandbox.is_some());

        // Verify plugins
        assert_eq!(options.plugins.len(), 1);

        // Verify auto_download_cli
        assert!(options.auto_download_cli);
    }

    #[test]
    fn test_options_builder_with_tools_list() {
        let options = ClaudeCodeOptions::builder()
            .tools(ToolsConfig::List(vec![
                "Read".to_string(),
                "Bash".to_string(),
            ]))
            .build();

        match options.tools.as_ref().unwrap() {
            ToolsConfig::List(list) => {
                assert_eq!(list.len(), 2);
                assert!(list.contains(&"Read".to_string()));
                assert!(list.contains(&"Bash".to_string()));
            },
            _ => panic!("Expected List variant"),
        }
    }

    #[test]
    fn test_options_builder_multiple_betas() {
        let options = ClaudeCodeOptions::builder()
            .add_beta(SdkBeta::Context1M)
            .betas(vec![SdkBeta::Context1M])
            .build();

        // betas() replaces, add_beta() appends - so only 1 from betas()
        assert_eq!(options.betas.len(), 1);
    }

    #[test]
    fn test_options_builder_add_beta_accumulates() {
        let options = ClaudeCodeOptions::builder()
            .add_beta(SdkBeta::Context1M)
            .add_beta(SdkBeta::Context1M)
            .build();

        // add_beta() accumulates
        assert_eq!(options.betas.len(), 2);
    }

    #[test]
    fn test_options_builder_multiple_plugins() {
        let options = ClaudeCodeOptions::builder()
            .add_plugin(SdkPluginConfig::Local {
                path: "/plugin1".to_string(),
            })
            .add_plugin(SdkPluginConfig::Local {
                path: "/plugin2".to_string(),
            })
            .plugins(vec![SdkPluginConfig::Local {
                path: "/plugin3".to_string(),
            }])
            .build();

        // plugins() replaces previous, so only 1
        assert_eq!(options.plugins.len(), 1);
    }

    #[test]
    fn test_options_builder_add_plugin_accumulates() {
        let options = ClaudeCodeOptions::builder()
            .add_plugin(SdkPluginConfig::Local {
                path: "/plugin1".to_string(),
            })
            .add_plugin(SdkPluginConfig::Local {
                path: "/plugin2".to_string(),
            })
            .add_plugin(SdkPluginConfig::Local {
                path: "/plugin3".to_string(),
            })
            .build();

        // add_plugin() accumulates
        assert_eq!(options.plugins.len(), 3);
    }

    #[test]
    fn test_message_result_with_structured_output() {
        // Test parsing result message with structured_output (snake_case)
        let json = r#"{
            "type": "result",
            "subtype": "success",
            "cost_usd": 0.05,
            "duration_ms": 1500,
            "duration_api_ms": 1200,
            "is_error": false,
            "num_turns": 3,
            "session_id": "session_123",
            "structured_output": {"answer": 42}
        }"#;

        let msg: Message = serde_json::from_str(json).unwrap();
        match msg {
            Message::Result {
                structured_output, ..
            } => {
                assert!(structured_output.is_some());
                let output = structured_output.unwrap();
                assert_eq!(output["answer"], 42);
            },
            _ => panic!("Expected Result message"),
        }
    }

    #[test]
    fn test_message_result_with_structured_output_camel_case() {
        // Test parsing result message with structuredOutput (camelCase alias)
        let json = r#"{
            "type": "result",
            "subtype": "success",
            "cost_usd": 0.05,
            "duration_ms": 1500,
            "duration_api_ms": 1200,
            "is_error": false,
            "num_turns": 3,
            "session_id": "session_123",
            "structuredOutput": {"name": "test", "value": true}
        }"#;

        let msg: Message = serde_json::from_str(json).unwrap();
        match msg {
            Message::Result {
                structured_output, ..
            } => {
                assert!(structured_output.is_some());
                let output = structured_output.unwrap();
                assert_eq!(output["name"], "test");
                assert_eq!(output["value"], true);
            },
            _ => panic!("Expected Result message"),
        }
    }

    #[test]
    fn test_default_options_new_fields() {
        let options = ClaudeCodeOptions::default();

        // Verify defaults for new fields
        assert!(options.tools.is_none());
        assert!(options.betas.is_empty());
        assert!(options.max_budget_usd.is_none());
        assert!(options.fallback_model.is_none());
        assert!(options.output_format.is_none());
        assert!(!options.enable_file_checkpointing);
        assert!(options.sandbox.is_none());
        assert!(options.plugins.is_empty());
        assert!(options.user.is_none());
        // Note: auto_download_cli defaults to false (Rust bool default)
        // Users should explicitly enable it with .auto_download_cli(true)
        assert!(!options.auto_download_cli);
    }
}
