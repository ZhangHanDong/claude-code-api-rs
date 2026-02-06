//! Permission modes example
//!
//! This example demonstrates different permission modes for file operations

use futures::StreamExt;
use nexus_claude::{ClaudeCodeOptions, Message, PermissionMode, Result, query};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("nexus_claude=info")
        .init();

    println!("Claude Code SDK - Permission Modes Example\n");

    // Example 1: Default mode (prompts for permission)
    println!("Example 1: Default permission mode");
    println!("----------------------------------");
    println!("This mode will prompt for permission before writing files.");
    println!("Since we're using --print mode, the prompt won't be shown.\n");

    let options = ClaudeCodeOptions::builder()
        .system_prompt("You are a helpful coding assistant.")
        .permission_mode(PermissionMode::Default)
        .build();

    run_query("Create a file called test1.txt with 'Hello World'", options).await?;

    println!("\n");

    // Example 2: AcceptEdits mode (auto-accepts edit prompts)
    println!("Example 2: AcceptEdits permission mode");
    println!("--------------------------------------");
    println!("This mode automatically accepts edit prompts but still checks permissions.\n");

    let options = ClaudeCodeOptions::builder()
        .system_prompt("You are a helpful coding assistant.")
        .permission_mode(PermissionMode::AcceptEdits)
        .allowed_tools(vec!["write".to_string(), "edit".to_string()])
        .build();

    run_query("Try to create a file called test2.txt", options).await?;

    println!("\n");

    // Example 3: BypassPermissions mode (allows all operations)
    println!("Example 3: BypassPermissions mode");
    println!("---------------------------------");
    println!("This mode allows all tool operations without prompting.");
    println!("USE WITH CAUTION - only in trusted environments!\n");

    let options = ClaudeCodeOptions::builder()
        .system_prompt("You are a helpful coding assistant.")
        .permission_mode(PermissionMode::BypassPermissions)
        .build();

    run_query("List files in current directory", options).await?;

    println!("\n");

    // Example 4: Restricted tools with AcceptEdits
    println!("Example 4: Restricted tools with AcceptEdits");
    println!("--------------------------------------------");
    println!("Only allows specific tools, auto-accepts those operations.\n");

    let options = ClaudeCodeOptions::builder()
        .system_prompt("You are a helpful coding assistant.")
        .permission_mode(PermissionMode::AcceptEdits)
        .allowed_tools(vec!["read".to_string()])
        .disallowed_tools(vec!["write".to_string(), "bash".to_string()])
        .build();

    run_query("Try to read and write a file", options).await?;

    Ok(())
}

async fn run_query(prompt: &str, options: ClaudeCodeOptions) -> Result<()> {
    println!("Query: {prompt}");
    println!("Permission mode: {:?}", options.permission_mode);

    let mut messages = query(prompt, Some(options)).await?;

    while let Some(msg) = messages.next().await {
        match msg? {
            Message::Assistant { message } => {
                for block in &message.content {
                    match block {
                        nexus_claude::ContentBlock::Text(text) => {
                            println!("Claude: {}", text.text);
                        },
                        nexus_claude::ContentBlock::ToolUse(tool_use) => {
                            println!(
                                "Claude wants to use tool: {} ({})",
                                tool_use.name, tool_use.id
                            );
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
                    println!("Query completed with error in {duration_ms}ms");
                } else {
                    println!("Query completed successfully in {duration_ms}ms");
                }
                break;
            },
            _ => {},
        }
    }

    Ok(())
}
