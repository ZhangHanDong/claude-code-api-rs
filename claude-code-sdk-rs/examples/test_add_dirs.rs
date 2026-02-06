//! Example demonstrating the use of add_dirs parameter
//!
//! This example shows how to add multiple directories as working directories

use futures::StreamExt;
use nexus_claude::{ClaudeCodeOptions, Result, query};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("nexus_claude=info")
        .init();

    println!("Testing add_dirs parameter...\n");

    // Example 1: Using add_dir() to add directories one by one
    let options1 = ClaudeCodeOptions::builder()
        .cwd("/Users/zhangalex/Work/Projects/FW/rust-claude-code-api")
        .add_dir("/Users/zhangalex/Work/Projects/FW/claude-code-sdk-python")
        .add_dir("/Users/zhangalex/Work/Projects/FW/url-preview")
        .system_prompt("You have access to multiple project directories")
        .build();

    println!("Example 1: Added directories one by one");
    println!("Directories: {:?}\n", options1.add_dirs);

    // Example 2: Using add_dirs() to add multiple directories at once
    let dirs = vec![
        PathBuf::from("/Users/zhangalex/Work/Projects/FW/rust-claude-code-api"),
        PathBuf::from("/Users/zhangalex/Work/Projects/FW/claude-code-sdk-python"),
        PathBuf::from("/Users/zhangalex/Work/Projects/FW/url-preview"),
    ];

    let options2 = ClaudeCodeOptions::builder()
        .add_dirs(dirs.clone())
        .system_prompt("You are working with multiple related projects")
        .permission_mode(nexus_claude::PermissionMode::AcceptEdits)
        .build();

    println!("Example 2: Added directories in batch");
    println!("Directories: {:?}\n", options2.add_dirs);

    // Make a query that could reference multiple directories
    println!("Querying Claude Code with access to multiple directories...\n");

    let mut messages = query(
        "Can you list the main programming languages used across the directories I've given you access to?",
        Some(options2),
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
