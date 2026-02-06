//! Example demonstrating combined use of new features
//!
//! This example shows how to use both settings and add_dirs together

use nexus_claude::{ClaudeCodeOptions, InteractiveClient, Result};
use std::env;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("nexus_claude=info")
        .init();

    println!("Testing combined features: settings + add_dirs\n");
    println!("===================================================\n");

    // Get the current directory to build absolute paths
    let current_dir = env::current_dir().expect("Failed to get current directory");

    // Build absolute path for settings file
    let settings_path = current_dir.join("examples/custom-claude-settings.json");
    let settings_str = settings_path.to_str().expect("Invalid path");

    // Check if settings file exists
    if !settings_path.exists() {
        println!("Warning: Settings file not found at: {settings_str}");
        println!("Creating example will continue without settings file.\n");
    }

    // Create directories list
    let project_dirs = vec![
        PathBuf::from("/Users/zhangalex/Work/Projects/FW/rust-claude-code-api"),
        PathBuf::from("/Users/zhangalex/Work/Projects/FW/claude-code-sdk-python"),
    ];

    // Build options with all new features
    let mut builder = ClaudeCodeOptions::builder()
        // Set primary working directory
        .cwd("/Users/zhangalex/Work/Projects/FW/rust-claude-code-api")
        // Add additional directories
        .add_dirs(project_dirs.clone())
        // Add one more directory individually
        .add_dir("/Users/zhangalex/Work/Projects/FW/url-preview");

    // Only add settings if file exists
    if settings_path.exists() {
        builder = builder.settings(settings_str);
    }

    let options = builder
        // Set other options
        .system_prompt(
            "You are an expert Rust and Python developer with access to multiple projects",
        )
        .model("claude-3-opus-20240229")
        .permission_mode(nexus_claude::PermissionMode::AcceptEdits)
        .max_turns(10)
        .build();

    println!("Configuration:");
    println!("--------------");
    println!("Settings file: {:?}", options.settings);
    println!("Working directory: {:?}", options.cwd);
    println!("Additional directories: {:?}", options.add_dirs);
    println!("Model: {:?}", options.model);
    println!("Permission mode: {:?}", options.permission_mode);
    println!();

    // Use interactive client for a more realistic test
    let mut client = InteractiveClient::new(options)?;

    // Connect to Claude
    println!("Connecting to Claude Code...");
    client.connect().await?;

    // Send a message that might utilize multiple directories
    println!("Sending query...\n");
    client
        .send_message(
            "Can you analyze the structure of these projects and tell me:
        1. What are the main differences between the Rust and Python SDK implementations?
        2. Are there any features in one that are missing in the other?
        Please be concise."
                .to_string(),
        )
        .await?;

    // Receive the response
    println!("Claude's response:\n");
    println!("==================\n");

    let messages = client.receive_response().await?;

    for msg in messages {
        match msg {
            nexus_claude::Message::Assistant { message } => {
                for block in message.content {
                    match block {
                        nexus_claude::ContentBlock::Text(text) => {
                            println!("{}", text.text);
                        },
                        nexus_claude::ContentBlock::ToolUse(tool) => {
                            println!("[Using tool: {}]", tool.name);
                        },
                        _ => {},
                    }
                }
            },
            nexus_claude::Message::Result {
                duration_ms,
                total_cost_usd,
                ..
            } => {
                println!("\n---");
                println!("Session completed");
                println!("Duration: {duration_ms}ms");
                if let Some(cost) = total_cost_usd {
                    println!("Cost: ${cost:.6}");
                }
            },
            _ => {},
        }
    }

    // Disconnect
    client.disconnect().await?;
    println!("\nDisconnected successfully!");

    Ok(())
}
