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

mod client;
// mod client_v2;  // Has compilation errors
// mod client_final;  // Has compilation errors
mod client_working;
mod errors;
mod interactive;
mod internal_query;
mod message_parser;
mod optimized_client;
mod perf_utils;
mod query;
mod transport;
mod types;

// Re-export main types and functions
pub use client::ClaudeSDKClient;
// pub use client_v2::ClaudeSDKClientV2;  // Has compilation errors
// pub use client_final::ClaudeSDKClientFinal;  // Has compilation errors
pub use client_working::ClaudeSDKClientWorking;
pub use errors::{Result, SdkError};
pub use interactive::InteractiveClient;
pub use query::query;
// Keep the old name as an alias for backward compatibility
pub use interactive::InteractiveClient as SimpleInteractiveClient;
pub use optimized_client::{ClientMode, OptimizedClient};
pub use perf_utils::{MessageBatcher, PerformanceMetrics, RetryConfig};

/// Default interactive client - the recommended client for interactive use
pub type ClaudeSDKClientDefault = InteractiveClient;
pub use types::{
    AssistantContent, AssistantMessage, ClaudeCodeOptions, ContentBlock, ContentValue,
    ControlProtocolFormat, ControlRequest, ControlResponse, McpServerConfig, Message, 
    PermissionMode, ResultMessage, SystemMessage, TextContent, ThinkingContent, 
    ToolResultContent, ToolUseContent, UserContent, UserMessage,
    // New permission and hook types
    PermissionBehavior, PermissionResult, PermissionResultAllow, PermissionResultDeny,
    PermissionRuleValue, PermissionUpdate, PermissionUpdateDestination, PermissionUpdateType,
    ToolPermissionContext, CanUseTool, HookCallback, HookContext, HookMatcher,
    // SDK Control Protocol types
    SDKControlInitializeRequest, SDKControlInterruptRequest, SDKControlMcpMessageRequest,
    SDKControlPermissionRequest, SDKControlRequest, SDKControlSetPermissionModeRequest,
    SDKHookCallbackRequest,
};

// Re-export builder
pub use types::ClaudeCodeOptionsBuilder;

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::{
        ClaudeCodeOptions, ClaudeSDKClient, ClaudeSDKClientWorking, Message, PermissionMode,
        Result, SdkError, query,
    };
}
