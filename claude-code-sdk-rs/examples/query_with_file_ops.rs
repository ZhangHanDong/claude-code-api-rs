//! Query with file operations example
//!
//! This example demonstrates how to use query() with BypassPermissions
//! to allow file operations in --print mode.

use cc_sdk::{ClaudeCodeOptions, Message, PermissionMode, Result, query};
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("cc_sdk=info")
        .init();

    println!("Claude Code SDK - Query with File Operations Example\n");

    // Example: Query with file operations using BypassPermissions
    println!("Creating a file using query() with BypassPermissions");
    println!("---------------------------------------------------");
    println!("WARNING: BypassPermissions allows all operations without prompts!");
    println!("Use this mode only in trusted environments.\n");

    let options = ClaudeCodeOptions::builder()
        .system_prompt("You are a helpful coding assistant.")
        .model("claude-3-5-sonnet-20241022")
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
                        cc_sdk::ContentBlock::Text(text) => {
                            println!("Claude: {}", text.text);
                        }
                        cc_sdk::ContentBlock::ToolUse(tool_use) => {
                            println!("Claude is using tool: {} ({})", tool_use.name, tool_use.id);
                            if let Some(file_path) = tool_use.input.get("file_path") {
                                println!("  File path: {}", file_path);
                            }
                        }
                        _ => {}
                    }
                }
            }
            Message::Result {
                duration_ms,
                is_error,
                ..
            } => {
                if is_error {
                    println!("\nQuery completed with error in {}ms", duration_ms);
                } else {
                    println!("\nQuery completed successfully in {}ms", duration_ms);
                }
                break;
            }
            _ => {}
        }
    }

    println!("\nNote: For interactive permission prompts, use ClaudeSDKClient instead.");

    Ok(())
}
