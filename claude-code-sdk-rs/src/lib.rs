//! # Claude Code SDK for Rust
//!
//! A Rust SDK for interacting with the Claude Code CLI, providing both simple query
//! and interactive client interfaces.
//!
//! ## Features
//!
//! - **Simple Query Interface**: One-shot queries with the `query` function
//! - **Interactive Client**: Stateful conversations with `ClaudeSDKClient`
//! - **Streaming Support**: Async streaming of responses
//! - **Type Safety**: Strongly typed messages and errors
//! - **Flexible Configuration**: Extensive options for customization
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use cc_sdk::{query, Result};
//! use futures::StreamExt;
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     let mut messages = query("What is 2 + 2?", None).await?;
//!     
//!     while let Some(msg) = messages.next().await {
//!         println!("{:?}", msg?);
//!     }
//!     
//!     Ok(())
//! }
//! ```

#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]

/// CLI download and management utilities
pub mod cli_download;
mod client;
mod client_working;
mod errors;
mod interactive;
mod internal_query;
mod message_parser;
pub mod model_recommendation;
mod optimized_client;
/// Session history API
pub mod sessions;
mod perf_utils;
mod query;
mod sdk_mcp;
pub mod token_tracker;
pub mod transport;
mod types;

/// LLM Proxy — use Claude Code subscription as a direct LLM interface.
///
/// Strips away the CC agent layer (tools, system prompt, hooks) and provides
/// a simple "send prompt, get text" API.
pub mod llm;

// Re-export main types and functions
pub use client::ClaudeSDKClient;
pub use client_working::ClaudeSDKClientWorking;
pub use errors::{Result, SdkError};
pub use interactive::InteractiveClient;
pub use internal_query::Query;
pub use query::query;
// Keep the old name as an alias for backward compatibility
pub use interactive::InteractiveClient as SimpleInteractiveClient;
pub use model_recommendation::ModelRecommendation;
pub use optimized_client::{ClientMode, OptimizedClient};
pub use sessions::{SessionInfo, SessionMessage, list_sessions, get_session_messages, rename_session, tag_session, delete_session, fork_session};
pub use perf_utils::{MessageBatcher, PerformanceMetrics, RetryConfig};
pub use token_tracker::{BudgetLimit, BudgetManager, BudgetStatus, TokenUsageTracker};
/// Default interactive client - the recommended client for interactive use
pub type ClaudeSDKClientDefault = InteractiveClient;
pub use types::{
    AssistantContent, AssistantMessage, ClaudeCodeOptions, ContentBlock, ContentValue,
    ControlProtocolFormat, ControlRequest, ControlResponse, McpServerConfig, Message,
    PermissionMode, ResultMessage, SystemMessage, TextContent, ThinkingContent,
    ToolResultContent, ToolUseContent, UserContent, UserMessage,
    // Permission types
    PermissionBehavior, PermissionResult, PermissionResultAllow, PermissionResultDeny,
    PermissionRuleValue, PermissionUpdate, PermissionUpdateDestination, PermissionUpdateType,
    ToolPermissionContext, CanUseTool,
    // Hook types (v0.3.0 - strongly-typed hooks)
    HookCallback, HookContext, HookMatcher,
    // Hook Input types (strongly-typed)
    BaseHookInput, HookInput, PreToolUseHookInput, PostToolUseHookInput,
    PostToolUseFailureHookInput, UserPromptSubmitHookInput, StopHookInput,
    SubagentStopHookInput, PreCompactHookInput,
    NotificationHookInput, SubagentStartHookInput, PermissionRequestHookInput,
    // Hook Output types (strongly-typed)
    HookJSONOutput, AsyncHookJSONOutput, SyncHookJSONOutput,
    HookSpecificOutput, PreToolUseHookSpecificOutput, PostToolUseHookSpecificOutput,
    PostToolUseFailureHookSpecificOutput, UserPromptSubmitHookSpecificOutput,
    SessionStartHookSpecificOutput, NotificationHookSpecificOutput,
    SubagentStartHookSpecificOutput, PermissionRequestHookSpecificOutput,
    // SDK Control Protocol types
    SDKControlInitializeRequest, SDKControlInterruptRequest, SDKControlMcpMessageRequest,
    SDKControlPermissionRequest, SDKControlRequest, SDKControlSetPermissionModeRequest,
    SDKHookCallbackRequest, SDKControlRewindFilesRequest,
    SDKControlGetContextUsageRequest, SDKControlStopTaskRequest,
    SDKControlMcpStatusRequest, SDKControlMcpReconnectRequest, SDKControlMcpToggleRequest,
    // Phase 2 enhancements
    SettingSource, AgentDefinition, SystemPrompt,
    // Task message types (Python SDK parity)
    TaskUsage, TaskStatus, TaskStartedMessage, TaskProgressMessage, TaskNotificationMessage,
    // Phase 3 enhancements (Python SDK v0.1.12+ sync)
    ToolsConfig, ToolsPreset, SdkBeta,
    // v0.7.0 enhancements (Python SDK parity)
    Effort, RateLimitStatus, RateLimitType, RateLimitInfo, AssistantMessageError,
    McpConnectionStatus, McpToolAnnotations, McpToolInfo, McpServerInfo, McpServerStatus,
    ThinkingConfig,
    SandboxSettings, SandboxNetworkConfig, SandboxIgnoreViolations,
    SdkPluginConfig,
    // v0.8.0 enhancements (Python SDK full parity)
    ContextUsageResponse, ContextUsageCategory, ApiUsage,
    TaskBudget, ForkSessionResult,
};

// Phase 3: Type aliases for naming consistency
/// Alias for ClaudeCodeOptions (matches Python SDK naming)
pub type ClaudeAgentOptions = ClaudeCodeOptions;
/// Alias for ClaudeCodeOptionsBuilder (matches Python SDK naming)
pub type ClaudeAgentOptionsBuilder = ClaudeCodeOptionsBuilder;

// Re-export builder
pub use types::ClaudeCodeOptionsBuilder;

// Re-export transport types for convenience
pub use transport::SubprocessTransport;
#[cfg(feature = "websocket")]
pub use transport::websocket::{WebSocketTransport, WebSocketConfig};

// Re-export SDK MCP types
pub use sdk_mcp::{
    SdkMcpServer, SdkMcpServerBuilder, ToolDefinition, ToolHandler, ToolInputSchema,
    ToolResult, create_simple_tool,
    ToolResultContent as SdkToolResultContent,
};

// Re-export LLM proxy types
pub use llm::{LlmOptions, LlmOptionsBuilder, LlmResponse};

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::{
        ClaudeCodeOptions, ClaudeSDKClient, ClaudeSDKClientWorking, Message, PermissionMode,
        Result, SdkError, query,
    };
}
