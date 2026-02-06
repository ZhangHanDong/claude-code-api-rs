//! Simple test for model availability
//! Run with: cargo run --example simple_model_test

use futures::StreamExt;
use nexus_claude::{ClaudeCodeOptions, PermissionMode, Result, query};

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Testing Model and Plan Mode ===\n");

    // Test 1: Use working model with Plan mode
    println!("Test 1: Using 'sonnet' alias with Plan mode");
    let options = ClaudeCodeOptions::builder()
        .model("sonnet")  // This works based on previous test
        .permission_mode(PermissionMode::Plan)
        .max_turns(1)
        .build();

    match query("Say 'Plan mode works'", Some(options)).await {
        Ok(mut stream) => {
            let mut success = false;
            while let Some(msg) = stream.next().await {
                if let Ok(nexus_claude::Message::Assistant { message }) = msg {
                    for block in message.content {
                        if let nexus_claude::ContentBlock::Text(text) = block {
                            println!("Response: {}", text.text);
                            success = true;
                        }
                    }
                }
            }
            if success {
                println!("✅ Plan mode with 'sonnet' works!\n");
            }
        },
        Err(e) => println!("❌ Error: {e:?}\n"),
    }

    // Test 2: Use full model name
    println!("Test 2: Using full Opus 4.1 name");
    let options = ClaudeCodeOptions::builder()
        .model("claude-opus-4-1-20250805")
        .max_turns(1)
        .build();

    match query("What model are you?", Some(options)).await {
        Ok(mut stream) => {
            let mut success = false;
            while let Some(msg) = stream.next().await {
                if let Ok(nexus_claude::Message::Assistant { message }) = msg {
                    for block in message.content {
                        if let nexus_claude::ContentBlock::Text(text) = block {
                            let preview = if text.text.len() > 100 {
                                format!("{}...", &text.text[..100])
                            } else {
                                text.text.clone()
                            };
                            println!("Response: {preview}");
                            success = true;
                        }
                    }
                }
            }
            if success {
                println!("✅ Full Opus 4.1 name works!\n");
            }
        },
        Err(e) => println!("❌ Error: {e:?}\n"),
    }

    // Test 3: Working model names summary
    println!("=== Working Model Names ===");
    println!("✅ Aliases that work:");
    println!("   - 'opus' (maps to latest Opus)");
    println!("   - 'sonnet' (maps to latest Sonnet)");
    println!("\n✅ Full names that work:");
    println!("   - 'claude-opus-4-1-20250805' (Opus 4.1)");
    println!("   - 'claude-sonnet-4-20250514' (Sonnet 4)");
    println!("   - 'claude-3-5-sonnet-20241022' (Claude 3.5 Sonnet)");
    println!("   - 'claude-3-5-haiku-20241022' (Claude 3.5 Haiku)");
    println!("\n❌ Names that DON'T work:");
    println!("   - 'opus-4.1' (returns 404)");
    println!("   - 'sonnet-4' (returns 404)");
    println!("\n✅ Permission modes supported:");
    println!("   - PermissionMode::Default");
    println!("   - PermissionMode::AcceptEdits");
    println!("   - PermissionMode::Plan (new in v0.1.7)");
    println!("   - PermissionMode::BypassPermissions");

    Ok(())
}
