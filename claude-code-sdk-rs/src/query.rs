//! Simple query interface for one-shot interactions
//!
//! This module provides the `query` function for simple, stateless interactions
//! with Claude Code CLI.

use crate::{
    errors::Result,
    transport::InputMessage,
    types::{ClaudeCodeOptions, Message, PermissionMode},
};
use futures::stream::Stream;
use std::pin::Pin;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tracing::{debug, info, warn};

/// Query input type
pub enum QueryInput {
    /// Simple string prompt
    Text(String),
    /// Stream of input messages for continuous interaction
    Stream(Pin<Box<dyn Stream<Item = InputMessage> + Send>>),
}

impl From<String> for QueryInput {
    fn from(s: String) -> Self {
        QueryInput::Text(s)
    }
}

impl From<&str> for QueryInput {
    fn from(s: &str) -> Self {
        QueryInput::Text(s.to_string())
    }
}

/// Query Claude Code for one-shot or unidirectional streaming interactions.
///
/// This function is ideal for simple, stateless queries where you don't need
/// bidirectional communication or conversation management. For interactive,
/// stateful conversations, use [`ClaudeSDKClient`](crate::ClaudeSDKClient) instead.
///
/// # Key differences from ClaudeSDKClient:
/// - **Unidirectional**: Send all messages upfront, receive all responses
/// - **Stateless**: Each query is independent, no conversation state
/// - **Simple**: Fire-and-forget style, no connection management
/// - **No interrupts**: Cannot interrupt or send follow-up messages
///
/// # When to use query():
/// - Simple one-off questions ("What is 2+2?")
/// - Batch processing of independent prompts
/// - Code generation or analysis tasks
/// - Automated scripts and CI/CD pipelines
/// - When you know all inputs upfront
///
/// # When to use ClaudeSDKClient:
/// - Interactive conversations with follow-ups
/// - Chat applications or REPL-like interfaces
/// - When you need to send messages based on responses
/// - When you need interrupt capabilities
/// - Long-running sessions with state
///
/// # Arguments
///
/// * `prompt` - The prompt to send to Claude. Can be a string for single-shot queries
///   or a Stream of InputMessage for streaming mode.
/// * `options` - Optional configuration. If None, defaults to `ClaudeCodeOptions::default()`.
///
/// # Returns
///
/// A stream of messages from the conversation.
///
/// # Examples
///
/// ## Simple query:
/// ```rust,no_run
/// use nexus_claude::{query, Result};
/// use futures::StreamExt;
///
/// #[tokio::main]
/// async fn main() -> Result<()> {
///     // One-off question
///     let mut messages = query("What is the capital of France?", None).await?;
///
///     while let Some(msg) = messages.next().await {
///         println!("{:?}", msg?);
///     }
///
///     Ok(())
/// }
/// ```
///
/// ## With options:
/// ```rust,no_run
/// use nexus_claude::{query, ClaudeCodeOptions, Result};
/// use futures::StreamExt;
///
/// #[tokio::main]
/// async fn main() -> Result<()> {
///     // Code generation with specific settings
///     let options = ClaudeCodeOptions::builder()
///         .system_prompt("You are an expert Python developer")
///         .model("claude-3-opus-20240229")
///         .build();
///
///     let mut messages = query("Create a Python web server", Some(options)).await?;
///
///     while let Some(msg) = messages.next().await {
///         println!("{:?}", msg?);
///     }
///
///     Ok(())
/// }
/// ```
pub async fn query(
    prompt: impl Into<QueryInput>,
    options: Option<ClaudeCodeOptions>,
) -> Result<impl Stream<Item = Result<Message>>> {
    let options = options.unwrap_or_default();
    let prompt = prompt.into();

    // Set environment variable to indicate SDK usage
    unsafe {
        std::env::set_var("CLAUDE_CODE_ENTRYPOINT", "sdk-rust");
    }

    match prompt {
        QueryInput::Text(text) => {
            // For simple text queries, use --print mode like Python SDK
            query_print_mode(text, options).await
        },
        QueryInput::Stream(_stream) => {
            // For streaming, use the interactive mode
            // TODO: Implement streaming mode
            Err(crate::SdkError::NotSupported {
                feature: "Streaming input mode not yet implemented".into(),
            })
        },
    }
}

/// Execute a simple query using --print mode
#[allow(deprecated)]
async fn query_print_mode(
    prompt: String,
    options: ClaudeCodeOptions,
) -> Result<impl Stream<Item = Result<Message>>> {
    use std::sync::Arc;
    use tokio::io::{AsyncBufReadExt, BufReader};
    use tokio::process::Command;
    use tokio::sync::Mutex;

    let cli_path = crate::transport::subprocess::find_claude_cli()?;
    let mut cmd = Command::new(&cli_path);

    // Build command with --print mode
    cmd.arg("--output-format").arg("stream-json");
    cmd.arg("--verbose");

    // System prompts (match Python SDK behavior)
    //
    // Python always passes `--system-prompt ""` when `system_prompt` is None.
    if let Some(ref prompt_v2) = options.system_prompt_v2 {
        match prompt_v2 {
            crate::types::SystemPrompt::String(s) => {
                cmd.arg("--system-prompt").arg(s);
            },
            crate::types::SystemPrompt::Preset { append, .. } => {
                if let Some(append_text) = append {
                    cmd.arg("--append-system-prompt").arg(append_text);
                }
            },
        }
    } else {
        #[allow(deprecated)]
        match options.system_prompt.as_deref() {
            Some(prompt) => {
                cmd.arg("--system-prompt").arg(prompt);
            },
            None => {
                cmd.arg("--system-prompt").arg("");
            },
        }

        #[allow(deprecated)]
        if let Some(ref append_prompt) = options.append_system_prompt {
            cmd.arg("--append-system-prompt").arg(append_prompt);
        }
    }

    if !options.allowed_tools.is_empty() {
        cmd.arg("--allowedTools")
            .arg(options.allowed_tools.join(","));
    }

    if let Some(max_turns) = options.max_turns {
        cmd.arg("--max-turns").arg(max_turns.to_string());
    }

    // Max thinking tokens (extended thinking budget)
    // Only pass if non-zero to match Python SDK behavior
    if options.max_thinking_tokens > 0 {
        cmd.arg("--max-thinking-tokens")
            .arg(options.max_thinking_tokens.to_string());
    }

    if !options.disallowed_tools.is_empty() {
        cmd.arg("--disallowedTools")
            .arg(options.disallowed_tools.join(","));
    }

    if let Some(ref model) = options.model {
        cmd.arg("--model").arg(model);
    }

    if let Some(ref tool_name) = options.permission_prompt_tool_name {
        cmd.arg("--permission-prompt-tool").arg(tool_name);
    }

    match options.permission_mode {
        PermissionMode::Default => {
            cmd.arg("--permission-mode").arg("default");
        },
        PermissionMode::AcceptEdits => {
            cmd.arg("--permission-mode").arg("acceptEdits");
        },
        PermissionMode::Plan => {
            cmd.arg("--permission-mode").arg("plan");
        },
        PermissionMode::BypassPermissions => {
            cmd.arg("--permission-mode").arg("bypassPermissions");
        },
    }

    if options.continue_conversation {
        cmd.arg("--continue");
    }

    if let Some(ref resume_id) = options.resume {
        cmd.arg("--resume").arg(resume_id);
    }

    if !options.mcp_servers.is_empty() {
        let mcp_config = serde_json::json!({
            "mcpServers": options.mcp_servers
        });
        cmd.arg("--mcp-config").arg(mcp_config.to_string());
    }

    // Extra arguments
    for (key, value) in &options.extra_args {
        let flag = if key.starts_with("--") || key.starts_with("-") {
            key.clone()
        } else {
            format!("--{key}")
        };
        cmd.arg(&flag);
        if let Some(val) = value {
            cmd.arg(val);
        }
    }

    // Add the prompt with --print
    cmd.arg("--print").arg("--").arg(&prompt);

    // Set up process pipes
    cmd.stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    // Handle max_output_tokens (priority: option > env var)
    // Maximum safe value is 32000, values above this may cause issues
    if let Some(max_tokens) = options.max_output_tokens {
        // Option takes priority - validate and cap at 32000
        let capped = max_tokens.clamp(1, 32000);
        cmd.env("CLAUDE_CODE_MAX_OUTPUT_TOKENS", capped.to_string());
        debug!("Setting max_output_tokens from option: {}", capped);
    } else {
        // Fall back to environment variable handling
        if let Ok(current_value) = std::env::var("CLAUDE_CODE_MAX_OUTPUT_TOKENS") {
            if let Ok(tokens) = current_value.parse::<u32>() {
                if tokens > 32000 {
                    warn!(
                        "CLAUDE_CODE_MAX_OUTPUT_TOKENS={} exceeds maximum safe value of 32000, overriding to 32000",
                        tokens
                    );
                    cmd.env("CLAUDE_CODE_MAX_OUTPUT_TOKENS", "32000");
                }
            } else {
                warn!(
                    "Invalid CLAUDE_CODE_MAX_OUTPUT_TOKENS value: {}, setting to 8192",
                    current_value
                );
                cmd.env("CLAUDE_CODE_MAX_OUTPUT_TOKENS", "8192");
            }
        }
    }

    info!("Starting Claude CLI with --print mode");
    debug!("Command: {:?}", cmd);

    if let Some(user) = options.user.as_deref() {
        crate::transport::subprocess::apply_process_user(&mut cmd, user)?;
    }

    let mut child = cmd.spawn().map_err(crate::SdkError::ProcessError)?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| crate::SdkError::ConnectionError("Failed to get stdout".into()))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| crate::SdkError::ConnectionError("Failed to get stderr".into()))?;

    // Wrap child process in Arc<Mutex> for shared ownership
    let child = Arc::new(Mutex::new(child));
    let child_clone = Arc::clone(&child);

    // Create a channel to collect messages
    let (tx, rx) = mpsc::channel(100);

    // Spawn stderr handler
    tokio::spawn(async move {
        let reader = BufReader::new(stderr);
        let mut lines = reader.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            if !line.trim().is_empty() {
                debug!("Claude stderr: {}", line);
            }
        }
    });

    // Clone tx for cleanup task
    let tx_cleanup = tx.clone();

    // Spawn stdout handler
    tokio::spawn(async move {
        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();

        while let Ok(Some(line)) = lines.next_line().await {
            if line.trim().is_empty() {
                continue;
            }

            debug!("Claude output: {}", line);

            // Parse JSON line
            match serde_json::from_str::<serde_json::Value>(&line) {
                Ok(json) => {
                    match crate::message_parser::parse_message(json) {
                        Ok(Some(message)) => {
                            if tx.send(Ok(message)).await.is_err() {
                                break;
                            }
                        },
                        Ok(None) => {
                            // Ignore non-message JSON
                        },
                        Err(e) => {
                            if tx.send(Err(e)).await.is_err() {
                                break;
                            }
                        },
                    }
                },
                Err(e) => {
                    debug!("Failed to parse JSON: {} - Line: {}", e, line);
                },
            }
        }

        // Wait for process to complete and ensure cleanup
        let mut child = child_clone.lock().await;
        match child.wait().await {
            Ok(status) => {
                if !status.success() {
                    let _ = tx
                        .send(Err(crate::SdkError::ProcessExited {
                            code: status.code(),
                        }))
                        .await;
                }
            },
            Err(e) => {
                let _ = tx.send(Err(crate::SdkError::ProcessError(e))).await;
            },
        }
    });

    // Spawn cleanup task that will ensure process is killed when stream is dropped
    tokio::spawn(async move {
        // Wait for the channel to be closed (all receivers dropped)
        tx_cleanup.closed().await;

        // Kill the process if it's still running
        let mut child = child.lock().await;
        match child.try_wait() {
            Ok(Some(_)) => {
                // Process already exited
                debug!("Claude CLI process already exited");
            },
            Ok(None) => {
                // Process still running, kill it
                info!("Killing Claude CLI process on stream drop");
                if let Err(e) = child.kill().await {
                    warn!("Failed to kill Claude CLI process: {}", e);
                } else {
                    // Wait for the process to actually exit
                    let _ = child.wait().await;
                    debug!("Claude CLI process killed and cleaned up");
                }
            },
            Err(e) => {
                warn!("Failed to check process status: {}", e);
            },
        }
    });

    // Return receiver as stream
    Ok(ReceiverStream::new(rx))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_input_from_string() {
        let input: QueryInput = "Hello".into();
        match input {
            QueryInput::Text(s) => assert_eq!(s, "Hello"),
            _ => panic!("Expected Text variant"),
        }
    }

    #[test]
    fn test_query_input_from_str() {
        let input: QueryInput = "World".into();
        match input {
            QueryInput::Text(s) => assert_eq!(s, "World"),
            _ => panic!("Expected Text variant"),
        }
    }

    #[test]
    fn test_extra_args_formatting() {
        use std::collections::HashMap;

        // Test that extra_args are properly formatted as CLI flags
        let mut extra_args = HashMap::new();
        extra_args.insert("custom-flag".to_string(), Some("value".to_string()));
        extra_args.insert("--already-dashed".to_string(), None);
        extra_args.insert("-s".to_string(), Some("short".to_string()));

        let options = ClaudeCodeOptions {
            extra_args,
            ..Default::default()
        };

        // Verify the args are properly stored
        assert_eq!(options.extra_args.len(), 3);
        assert!(options.extra_args.contains_key("custom-flag"));
        assert!(options.extra_args.contains_key("--already-dashed"));
        assert!(options.extra_args.contains_key("-s"));
    }
}
