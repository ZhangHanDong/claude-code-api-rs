//! Interactive client example
//!
//! This example demonstrates how to use the SimpleInteractiveClient for
//! interactive, stateful conversations with Claude.

use claude_code_sdk::{ClaudeCodeOptions, SimpleInteractiveClient, Message, PermissionMode, Result, ContentBlock};
use std::io::{self, Write};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("claude_code_sdk=debug,interactive=info")
        .init();

    println!("Claude Code SDK - Interactive Client Example");
    println!("Type 'quit' to exit\n");

    // Create client with options
    let options = ClaudeCodeOptions::builder()
        .system_prompt("You are a helpful assistant.")
        .permission_mode(PermissionMode::AcceptEdits)
        .model("claude-3-5-sonnet-20241022")
        .build();

    let mut client = SimpleInteractiveClient::new(options)?;

    // Connect to Claude
    println!("Connecting to Claude...");
    client.connect().await?;
    println!("Connected!\n");
    
    println!("Ready for conversation. Type your message:");

    // Interactive loop
    let stdin = io::stdin();
    let mut input = String::new();

    loop {
        print!("You: ");
        io::stdout().flush()?;
        
        input.clear();
        stdin.read_line(&mut input)?;
        
        let input = input.trim();
        
        if input.is_empty() {
            continue;
        }
        
        if input == "quit" {
            break;
        }

        // Send message and receive response
        let messages = client.send_and_receive(input.to_string()).await?;
        
        // Process response
        for msg in &messages {
            match msg {
                Message::Assistant { message } => {
                    print!("Claude: ");
                    for block in &message.content {
                        match block {
                            ContentBlock::Text(text) => {
                                print!("{}", text.text);
                            }
                            ContentBlock::ToolUse(tool) => {
                                println!("\n[Using tool: {} ({})]", tool.name, tool.id);
                            }
                            ContentBlock::ToolResult(result) => {
                                println!("[Tool result for {}]", result.tool_use_id);
                            }
                        }
                    }
                    println!();
                }
                Message::System { subtype, data: _ } => {
                    if subtype != "thinking" {
                        println!("[System: {}]", subtype);
                    }
                }
                Message::Result { duration_ms, total_cost_usd, .. } => {
                    print!("[Response time: {}ms", duration_ms);
                    if let Some(cost) = total_cost_usd {
                        print!(", cost: ${:.6}", cost);
                    }
                    println!("]");
                }
                _ => {}
            }
        }
        println!();
    }

    // Disconnect
    println!("\nDisconnecting...");
    client.disconnect().await?;
    println!("Goodbye!");

    Ok(())
}