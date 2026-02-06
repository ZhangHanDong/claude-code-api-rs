//! Safe example demonstrating the use of the settings parameter with proper file handling
//!
//! This example shows how to safely use a custom settings file with Claude Code

use futures::StreamExt;
use nexus_claude::{ClaudeCodeOptions, Result, query};
use std::path::Path;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("nexus_claude=info")
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
        .permission_mode(nexus_claude::PermissionMode::AcceptEdits);

    if !settings_path.is_empty() {
        builder = builder.settings(settings_path);
        println!("Using settings file: {settings_path}");
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
            nexus_claude::Message::Assistant { message } => {
                for block in message.content {
                    if let nexus_claude::ContentBlock::Text(text) = block {
                        println!("Claude: {}", text.text);
                    }
                }
            },
            nexus_claude::Message::Result {
                duration_ms,
                total_cost_usd,
                ..
            } => {
                println!("\n---");
                println!("Completed in {duration_ms}ms");
                if let Some(cost) = total_cost_usd {
                    println!("Cost: ${cost:.6}");
                }
            },
            _ => {},
        }
    }

    Ok(())
}
