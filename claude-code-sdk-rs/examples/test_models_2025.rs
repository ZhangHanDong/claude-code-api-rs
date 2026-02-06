//! Test example for 2025 models (Opus 4.1 and Sonnet 4)
//!
//! Run with: cargo run --example test_models_2025

use futures::StreamExt;
use nexus_claude::{ClaudeCodeOptions, Message, Result, query};

async fn test_opus_4_1() -> Result<()> {
    println!("\n=== Testing Opus 4.1 ===");

    // Test with short alias
    let options = ClaudeCodeOptions::builder()
        .model("opus")
        .max_turns(1)
        .build();

    println!("Testing with model: opus-4.1");
    let mut messages = query("What model are you? Reply in one sentence.", Some(options)).await?;

    let mut found_response = false;
    while let Some(msg) = messages.next().await {
        match msg? {
            Message::Assistant { message } => {
                for block in message.content {
                    if let nexus_claude::ContentBlock::Text(text) = block {
                        println!("Response: {}", text.text);
                        found_response = true;
                    }
                }
            },
            Message::Result { duration_ms, .. } => {
                println!("Response time: {duration_ms}ms");
            },
            _ => {},
        }
    }

    if found_response {
        println!("✅ Opus 4.1 model works!");
    } else {
        println!("❌ Opus 4.1 model failed to respond");
    }

    Ok(())
}

async fn test_sonnet_4() -> Result<()> {
    println!("\n=== Testing Sonnet 4 ===");

    let options = ClaudeCodeOptions::builder()
        .model("sonnet")
        .max_turns(1)
        .build();

    println!("Testing with model: sonnet-4");
    let mut messages = query("What model are you? Reply in one sentence.", Some(options)).await?;

    let mut found_response = false;
    while let Some(msg) = messages.next().await {
        match msg? {
            Message::Assistant { message } => {
                for block in message.content {
                    if let nexus_claude::ContentBlock::Text(text) = block {
                        println!("Response: {}", text.text);
                        found_response = true;
                    }
                }
            },
            Message::Result { duration_ms, .. } => {
                println!("Response time: {duration_ms}ms");
            },
            _ => {},
        }
    }

    if found_response {
        println!("✅ Sonnet 4 model works!");
    } else {
        println!("❌ Sonnet 4 model failed to respond");
    }

    Ok(())
}

async fn test_model_aliases() -> Result<()> {
    println!("\n=== Testing Model Aliases ===");

    // Test "opus" alias (should map to latest Opus)
    let options_opus = ClaudeCodeOptions::builder()
        .model("opus")
        .max_turns(1)
        .build();

    println!("Testing with alias: opus");
    match query("Say OK", Some(options_opus)).await {
        Ok(mut stream) => {
            let mut success = false;
            while let Some(msg) = stream.next().await {
                if let Ok(Message::Assistant { .. }) = msg {
                    success = true;
                }
            }
            if success {
                println!("✅ 'opus' alias works!");
            }
        },
        Err(e) => println!("❌ 'opus' alias failed: {e:?}"),
    }

    // Test "sonnet" alias
    let options_sonnet = ClaudeCodeOptions::builder()
        .model("sonnet")
        .max_turns(1)
        .build();

    println!("Testing with alias: sonnet");
    match query("Say OK", Some(options_sonnet)).await {
        Ok(mut stream) => {
            let mut success = false;
            while let Some(msg) = stream.next().await {
                if let Ok(Message::Assistant { .. }) = msg {
                    success = true;
                }
            }
            if success {
                println!("✅ 'sonnet' alias works!");
            }
        },
        Err(e) => println!("❌ 'sonnet' alias failed: {e:?}"),
    }

    Ok(())
}

async fn test_full_model_names() -> Result<()> {
    println!("\n=== Testing Full Model Names ===");

    // Test full Opus 4.1 name
    let options = ClaudeCodeOptions::builder()
        .model("claude-opus-4-1-20250805")
        .max_turns(1)
        .build();

    println!("Testing: claude-opus-4-1-20250805");
    match query("Say OK", Some(options)).await {
        Ok(mut stream) => {
            let mut success = false;
            while let Some(msg) = stream.next().await {
                if let Ok(Message::Assistant { .. }) = msg {
                    success = true;
                }
            }
            if success {
                println!("✅ Full Opus 4.1 name works!");
            }
        },
        Err(e) => println!("❌ Full Opus 4.1 name failed: {e:?}"),
    }

    // Test full Sonnet 4 name
    let options = ClaudeCodeOptions::builder()
        .model("claude-sonnet-4-20250514")
        .max_turns(1)
        .build();

    println!("Testing: claude-sonnet-4-20250514");
    match query("Say OK", Some(options)).await {
        Ok(mut stream) => {
            let mut success = false;
            while let Some(msg) = stream.next().await {
                if let Ok(Message::Assistant { .. }) = msg {
                    success = true;
                }
            }
            if success {
                println!("✅ Full Sonnet 4 name works!");
            }
        },
        Err(e) => println!("❌ Full Sonnet 4 name failed: {e:?}"),
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    println!("=== Claude Code SDK - Testing 2025 Models ===");
    println!("This will test various model names and aliases\n");

    // Test different model formats
    if let Err(e) = test_opus_4_1().await {
        println!("Opus 4.1 test error: {e:?}");
    }

    if let Err(e) = test_sonnet_4().await {
        println!("Sonnet 4 test error: {e:?}");
    }

    if let Err(e) = test_model_aliases().await {
        println!("Model aliases test error: {e:?}");
    }

    if let Err(e) = test_full_model_names().await {
        println!("Full model names test error: {e:?}");
    }

    println!("\n=== All model tests completed ===");
}
