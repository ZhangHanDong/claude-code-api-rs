//! File operations example using ClaudeSDKClient
//!
//! This example demonstrates how to use the interactive client
//! for operations that require file system access.

use cc_sdk::{ClaudeSDKClient, ClaudeCodeOptions, Message, PermissionMode, Result};
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("cc_sdk=info")
        .init();

    println!("Claude Code SDK - File Operations Example\n");
    println!("This example shows how to use ClaudeSDKClient for file operations.\n");

    // Configure options with permission mode for file operations
    let options = ClaudeCodeOptions::builder()
        .system_prompt("You are a helpful coding assistant.")
        .model("claude-3-5-sonnet-20241022")
        .permission_mode(PermissionMode::BypassPermissions)  // Allow all file operations
        .allowed_tools(vec!["write".to_string(), "edit".to_string(), "read".to_string()])
        .cwd(std::env::current_dir().unwrap_or_default())  // Set working directory
        .build();

    // Create client
    let mut client = ClaudeSDKClient::new(options);

    // Connect to Claude CLI with initial prompt
    println!("Connecting to Claude CLI...");
    client.connect(Some("Hello! I'm ready to help with file operations.".to_string())).await?;
    println!("Connected!");

    // Process initial response
    let mut messages = client.receive_messages().await;
    while let Some(msg) = messages.next().await {
        match msg? {
            Message::Assistant { message } => {
                for block in &message.content {
                    if let cc_sdk::ContentBlock::Text(text) = block {
                        println!("Claude: {}", text.text);
                    }
                }
            }
            Message::Result { .. } => break,
            _ => {}
        }
    }

    println!("\n");

    // Example 1: Create a file
    println!("Example 1: Creating a new file");
    println!("------------------------------");

    client.send_request(
        "Create a file called hello_world.rs with a Rust hello world program".to_string(),
        None
    ).await?;

    // Process response
    let mut messages = client.receive_messages().await;
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
                        }
                        _ => {}
                    }
                }
            }
            Message::Result { duration_ms, .. } => {
                println!("\nOperation completed in {}ms", duration_ms);
                break;
            }
            _ => {}
        }
    }

    println!("\n");

    // Example 2: Read and modify a file
    println!("Example 2: Reading and modifying a file");
    println!("---------------------------------------");

    client.send_request(
        "Read the hello_world.rs file and add a comment explaining what it does".to_string(),
        None
    ).await?;

    // Process response
    let mut messages = client.receive_messages().await;
    while let Some(msg) = messages.next().await {
        match msg? {
            Message::Assistant { message } => {
                for block in &message.content {
                    if let cc_sdk::ContentBlock::Text(text) = block {
                        println!("Claude: {}", text.text);
                    }
                }
            }
            Message::Result { duration_ms, total_cost_usd, .. } => {
                println!("\nOperation completed in {}ms", duration_ms);
                if let Some(cost) = total_cost_usd {
                    println!("Total cost: ${:.4}", cost);
                }
                break;
            }
            _ => {}
        }
    }

    // Disconnect
    println!("\nDisconnecting...");
    client.disconnect().await?;
    println!("Done!");

    Ok(())
}
