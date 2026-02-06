//! Query with file operations example
//!
//! This example demonstrates how to use query() with BypassPermissions
//! to allow file operations in --print mode.

use futures::StreamExt;
use nexus_claude::{ClaudeCodeOptions, Message, PermissionMode, Result, query};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("nexus_claude=info")
        .init();

    println!("Claude Code SDK - Query with File Operations Example\n");

    // Example: Query with file operations using BypassPermissions
    println!("Creating a file using query() with BypassPermissions");
    println!("---------------------------------------------------");
    println!("WARNING: BypassPermissions allows all operations without prompts!");
    println!("Use this mode only in trusted environments.\n");

    let options = ClaudeCodeOptions::builder()
        .system_prompt("You are a helpful coding assistant.")
        .model("sonnet")
        .permission_mode(PermissionMode::BypassPermissions) // Allow all operations
        .allowed_tools(vec!["write".to_string()]) // Still good practice to limit tools
        .build();

    let mut messages = query(
        "Create a file called hello.txt with the content 'Hello from Rust SDK!'",
        Some(options),
    )
    .await?;

    while let Some(msg) = messages.next().await {
        match msg? {
            Message::Assistant { message } => {
                for block in &message.content {
                    match block {
                        nexus_claude::ContentBlock::Text(text) => {
                            println!("Claude: {}", text.text);
                        },
                        nexus_claude::ContentBlock::ToolUse(tool_use) => {
                            println!("Claude is using tool: {} ({})", tool_use.name, tool_use.id);
                            if let Some(file_path) = tool_use.input.get("file_path") {
                                println!("  File path: {file_path}");
                            }
                        },
                        _ => {},
                    }
                }
            },
            Message::Result {
                duration_ms,
                is_error,
                ..
            } => {
                if is_error {
                    println!("\nQuery completed with error in {duration_ms}ms");
                } else {
                    println!("\nQuery completed successfully in {duration_ms}ms");
                }
                break;
            },
            _ => {},
        }
    }

    println!("\nNote: For interactive permission prompts, use ClaudeSDKClient instead.");

    Ok(())
}
