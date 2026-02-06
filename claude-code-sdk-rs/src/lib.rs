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
//! use nexus_claude::{query, Result};
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
// mod client_v2;  // Has compilation errors
// mod client_final;  // Has compilation errors
mod client_working;
mod errors;
mod interactive;
mod internal_query;
mod message_parser;
pub mod model_recommendation;
mod optimized_client;
mod perf_utils;
mod query;
mod sdk_mcp;
pub mod token_tracker;
pub mod transport;
mod types;

/// Memory module for persistent conversation context
pub mod memory;

// Re-export main types and functions
pub use client::ClaudeSDKClient;
// pub use client_v2::ClaudeSDKClientV2;  // Has compilation errors
// pub use client_final::ClaudeSDKClientFinal;  // Has compilation errors
pub use client_working::ClaudeSDKClientWorking;
pub use errors::{Result, SdkError};
pub use interactive::InteractiveClient;
pub use internal_query::Query;
pub use query::query;
// Keep the old name as an alias for backward compatibility
pub use interactive::InteractiveClient as SimpleInteractiveClient;
pub use model_recommendation::ModelRecommendation;
pub use optimized_client::{ClientMode, OptimizedClient};
pub use perf_utils::{MessageBatcher, PerformanceMetrics, RetryConfig};
pub use token_tracker::{BudgetLimit, BudgetManager, BudgetStatus, TokenUsageTracker};
/// Default interactive client - the recommended client for interactive use
pub type ClaudeSDKClientDefault = InteractiveClient;
pub use types::{
    AgentDefinition,
    AssistantContent,
    AssistantMessage,
    AsyncHookJSONOutput,
    // Hook Input types (strongly-typed)
    BaseHookInput,
    CanUseTool,
    ClaudeCodeOptions,
    ContentBlock,
    ContentValue,
    ControlProtocolFormat,
    ControlRequest,
    ControlResponse,
    // Hook types (v0.3.0 - strongly-typed hooks)
    HookCallback,
    HookContext,
    HookInput,
    // Hook Output types (strongly-typed)
    HookJSONOutput,
    HookMatcher,
    HookSpecificOutput,
    McpServerConfig,
    Message,
    // Permission types
    PermissionBehavior,
    PermissionMode,
    PermissionResult,
    PermissionResultAllow,
    PermissionResultDeny,
    PermissionRuleValue,
    PermissionUpdate,
    PermissionUpdateDestination,
    PermissionUpdateType,
    PostToolUseHookInput,
    PostToolUseHookSpecificOutput,
    PreCompactHookInput,
    PreToolUseHookInput,
    PreToolUseHookSpecificOutput,
    ResultMessage,
    // SDK Control Protocol types
    SDKControlInitializeRequest,
    SDKControlInterruptRequest,
    SDKControlMcpMessageRequest,
    SDKControlPermissionRequest,
    SDKControlRequest,
    SDKControlRewindFilesRequest,
    SDKControlSetPermissionModeRequest,
    SDKHookCallbackRequest,
    SandboxIgnoreViolations,
    SandboxNetworkConfig,
    SandboxSettings,
    SdkBeta,
    SdkPluginConfig,
    SessionStartHookSpecificOutput,
    // Phase 2 enhancements
    SettingSource,
    StopHookInput,
    SubagentStopHookInput,
    SyncHookJSONOutput,
    SystemMessage,
    SystemPrompt,
    TextContent,
    ThinkingContent,
    ToolPermissionContext,
    ToolResultContent,
    ToolUseContent,
    // Phase 3 enhancements (Python SDK v0.1.12+ sync)
    ToolsConfig,
    ToolsPreset,
    UserContent,
    UserMessage,
    UserPromptSubmitHookInput,
    UserPromptSubmitHookSpecificOutput,
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

// Re-export SDK MCP types
pub use sdk_mcp::{
    SdkMcpServer, SdkMcpServerBuilder, ToolDefinition, ToolHandler, ToolInputSchema, ToolResult,
    ToolResultContent as SdkToolResultContent, create_simple_tool,
};

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::{
        ClaudeCodeOptions, ClaudeSDKClient, ClaudeSDKClientWorking, Message, PermissionMode,
        Result, SdkError, query,
    };
}
