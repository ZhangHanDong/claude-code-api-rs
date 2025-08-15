//! Safe example demonstrating the use of the settings parameter with proper file handling
//!
//! This example shows how to safely use a custom settings file with Claude Code

use cc_sdk::{ClaudeCodeOptions, Result, query};
use futures::StreamExt;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("cc_sdk=info")
        .init();

    println!("Testing settings parameter with safe file handling...\n");

    // Check if settings file exists and get the correct path
    let settings_path = if Path::new("examples/claude-settings.json").exists() {
        // Running from project root
        "examples/claude-settings.json"
    } else if Path::new("claude-settings.json").exists() {
        // Running from examples directory
        "claude-settings.json"
    } else {
        println!("Warning: Settings file not found, proceeding without it.");
        println!(
            "To use a settings file, ensure claude-settings.json exists in the current or examples directory.\n"
        );
        // Use None for settings
        ""
    };

    // Create options with a custom settings file (if it exists)
    let mut builder = ClaudeCodeOptions::builder()
        .system_prompt("You are a helpful assistant")
        .model("claude-3-opus-20240229")
        .permission_mode(cc_sdk::PermissionMode::AcceptEdits);

    if !settings_path.is_empty() {
        builder = builder.settings(settings_path);
        println!("Using settings file: {}", settings_path);
    } else {
        println!("Running without settings file");
    }

    let options = builder.build();

    println!("Querying Claude Code...\n");

    // Make a simple query
    let mut messages = query(
        "What programming language is best for systems programming and why?",
        Some(options),
    )
    .await?;

    // Process the response
    while let Some(msg) = messages.next().await {
        match msg? {
            cc_sdk::Message::Assistant { message } => {
                for block in message.content {
                    if let cc_sdk::ContentBlock::Text(text) = block {
                        println!("Claude: {}", text.text);
                    }
                }
            }
            cc_sdk::Message::Result {
                duration_ms,
                total_cost_usd,
                ..
            } => {
                println!("\n---");
                println!("Completed in {}ms", duration_ms);
                if let Some(cost) = total_cost_usd {
                    println!("Cost: ${:.6}", cost);
                }
            }
            _ => {}
        }
    }

    Ok(())
}
