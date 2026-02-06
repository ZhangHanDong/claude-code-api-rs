//! Example demonstrating interactive session with automatic account info display
//!
//! This example shows how to start a session and automatically display
//! account information at the beginning for verification purposes.

use futures::StreamExt;
use nexus_claude::{ClaudeCodeOptions, ClaudeSDKClient, Message, Result};

#[tokio::main]
async fn main() -> Result<()> {
    println!("╔═══════════════════════════════════════════════╗");
    println!("║   Interactive Session with Account Info      ║");
    println!("╚═══════════════════════════════════════════════╝\n");

    // Configure options
    let options = ClaudeCodeOptions::builder()
        .model("claude-sonnet-4-5-20250929") // Use latest Sonnet
        .system_prompt("You are a helpful coding assistant")
        .build();

    let mut client = ClaudeSDKClient::new(options);

    // Step 1: Connect
    println!("🔌 Connecting to Claude CLI...");
    client.connect(None).await?;
    println!("   ✅ Connected\n");

    // Step 2: Display account information
    println!("👤 Fetching account information...");
    match client.get_account_info().await {
        Ok(account_info) => {
            println!("╭─────────────────────────────────────────────╮");
            println!("│ 📋 Current Session Account                  │");
            println!("├─────────────────────────────────────────────┤");
            for line in account_info.lines() {
                println!("│ {:<44}│", line);
            }
            println!("╰─────────────────────────────────────────────╯\n");
        },
        Err(e) => {
            eprintln!("⚠️  Warning: Could not retrieve account info: {}\n", e);
        },
    }

    // Step 3: Continue with normal session
    println!("💬 Starting interactive session...\n");
    println!("─────────────────────────────────────────────────\n");

    // Send a query
    client
        .send_user_message("What is the capital of France?".to_string())
        .await?;

    // Receive response
    let mut messages = client.receive_messages().await;
    while let Some(msg_result) = messages.next().await {
        match msg_result? {
            Message::Assistant { message } => {
                for block in message.content {
                    if let nexus_claude::ContentBlock::Text(text) = block {
                        println!("🤖 Claude: {}\n", text.text);
                    }
                }
            },
            Message::Result {
                duration_ms,
                usage,
                total_cost_usd,
                ..
            } => {
                println!("─────────────────────────────────────────────────");
                println!("📊 Response Stats:");
                println!("   ⏱️  Duration: {}ms", duration_ms);

                if let Some(usage_json) = usage {
                    if let Some(input_tokens) = usage_json.get("input_tokens") {
                        println!("   📥 Input tokens: {}", input_tokens);
                    }
                    if let Some(output_tokens) = usage_json.get("output_tokens") {
                        println!("   📤 Output tokens: {}", output_tokens);
                    }
                }

                if let Some(cost) = total_cost_usd {
                    println!("   💰 Cost: ${:.6}", cost);
                }
                println!("─────────────────────────────────────────────────\n");
                break;
            },
            _ => {},
        }
    }

    // Disconnect
    println!("🔌 Disconnecting...");
    client.disconnect().await?;
    println!("   ✅ Session ended\n");

    Ok(())
}
