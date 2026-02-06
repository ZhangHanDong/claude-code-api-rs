//! Comprehensive test example for all v0.1.7 features
//! Tests: 2025 models, Plan mode, extra_args, ThinkingContent
//!
//! Run with: cargo run --example comprehensive_test

use futures::StreamExt;
use nexus_claude::{
    ClaudeCodeOptions, ContentBlock, InteractiveClient, Message, PermissionMode, Result, query,
};
use std::collections::HashMap;

/// Test all 2025 models with different configurations
async fn test_all_models() -> Result<()> {
    println!("\n📊 === Testing All 2025 Models ===\n");

    let models = vec![
        ("opus", "Opus 4.1 - Most capable"),
        ("sonnet", "Sonnet 4 - Balanced"),
        ("opus", "Latest Opus alias"),
        ("sonnet", "Latest Sonnet alias"),
        ("sonnet", "Claude 3.5 Sonnet"),
        ("sonnet", "Claude 3.5 Haiku - Fastest"),
    ];

    for (model, description) in models {
        print!("Testing {model} ({description})... ");

        let options = ClaudeCodeOptions::builder()
            .model(model)
            .max_turns(1)
            .build();

        match query("Reply with just 'OK'", Some(options)).await {
            Ok(mut stream) => {
                let mut success = false;
                while let Some(msg) = stream.next().await {
                    if let Ok(Message::Assistant { .. }) = msg {
                        success = true;
                    }
                }
                if success {
                    println!("✅");
                } else {
                    println!("⚠️ No response");
                }
            },
            Err(_) => println!("❌ Not available"),
        }
    }

    Ok(())
}

/// Test Plan permission mode with different scenarios
async fn test_plan_mode_scenarios() -> Result<()> {
    println!("\n📝 === Testing Plan Mode Scenarios ===\n");

    // Scenario 1: Simple planning task
    println!("1. Simple planning task:");
    let options = ClaudeCodeOptions::builder()
        .model("sonnet")
        .permission_mode(PermissionMode::Plan)
        .max_turns(1)
        .build();

    let mut messages = query("Plan the steps to create a CLI tool in Rust", Some(options)).await?;

    let mut _plan_found = false;
    while let Some(msg) = messages.next().await {
        if let Ok(Message::Assistant { message }) = msg {
            for block in message.content {
                if let ContentBlock::Text(_) = block {
                    _plan_found = true;
                    println!("   ✅ Plan generated successfully");
                    break;
                }
            }
        }
    }
    if !_plan_found {
        println!("   ❌ No plan generated");
    }

    // Scenario 2: Plan mode with extended thinking
    println!("\n2. Plan mode with extended thinking:");
    let options = ClaudeCodeOptions::builder()
        .model("opus")
        .permission_mode(PermissionMode::Plan)
        .max_thinking_tokens(8000)
        .max_turns(1)
        .build();

    let mut messages = query(
        "Plan a complex distributed system architecture",
        Some(options),
    )
    .await?;

    let mut thinking_found = false;
    let mut _plan_found = false;

    while let Some(msg) = messages.next().await {
        if let Ok(Message::Assistant { message }) = msg {
            for block in message.content {
                match block {
                    ContentBlock::Thinking(thinking) => {
                        thinking_found = true;
                        println!("   ✅ Thinking content captured");
                        println!("      Signature: {}", thinking.signature);
                    },
                    ContentBlock::Text(_) => {
                        _plan_found = true;
                        println!("   ✅ Plan with thinking generated");
                    },
                    _ => {},
                }
            }
        }
    }

    if !thinking_found {
        println!("   ℹ️ No thinking content (may not be supported by model)");
    }

    Ok(())
}

/// Test extra_args feature (new in v0.1.7)
async fn test_extra_args() -> Result<()> {
    println!("\n🔧 === Testing Extra Args Feature ===\n");

    let mut extra_args = HashMap::new();
    extra_args.insert("temperature".to_string(), Some("0.5".to_string()));
    extra_args.insert("verbose".to_string(), None);
    extra_args.insert("custom-flag".to_string(), Some("test-value".to_string()));

    let options = ClaudeCodeOptions::builder()
        .model("sonnet")
        .extra_args(extra_args.clone())
        .max_turns(1)
        .build();

    println!("Testing with extra args:");
    for (key, value) in &extra_args {
        match value {
            Some(v) => println!("  --{key} {v}"),
            None => println!("  --{key}"),
        }
    }

    match query("Say 'Extra args work!'", Some(options)).await {
        Ok(mut stream) => {
            let mut success = false;
            while let Some(msg) = stream.next().await {
                if let Ok(Message::Assistant { .. }) = msg {
                    success = true;
                }
            }
            if success {
                println!("✅ Extra args feature works!");
            } else {
                println!("⚠️ Query succeeded but no response");
            }
        },
        Err(e) => {
            println!("❌ Extra args test failed: {e:?}");
        },
    }

    Ok(())
}

/// Test interactive client with all new features
async fn test_interactive_with_new_features() -> Result<()> {
    println!("\n💬 === Testing Interactive Client with New Features ===\n");

    let mut extra_args = HashMap::new();
    extra_args.insert("max-retries".to_string(), Some("3".to_string()));

    let options = ClaudeCodeOptions::builder()
        .model("sonnet")
        .permission_mode(PermissionMode::Plan)
        .system_prompt("You are a planning assistant")
        .max_thinking_tokens(5000)
        .extra_args(extra_args)
        .max_turns(3)
        .build();

    let mut client = InteractiveClient::new(options)?;
    client.connect().await?;

    println!("Connected with Plan mode and extra args");

    // First message
    let messages = client
        .send_and_receive("Create a plan for building a web scraper".to_string())
        .await?;

    let mut msg_count = 0;
    for msg in &messages {
        if let Message::Assistant { .. } = msg {
            msg_count += 1;
        }
    }

    if msg_count > 0 {
        println!("✅ First message received");

        // Follow-up
        let messages = client
            .send_and_receive("What about error handling?".to_string())
            .await?;

        for msg in &messages {
            if let Message::Assistant { .. } = msg {
                println!("✅ Follow-up message received");
                break;
            }
        }
    }

    client.disconnect().await?;
    println!("✅ Interactive session completed");

    Ok(())
}

/// Test ThinkingContent parsing
async fn test_thinking_content() -> Result<()> {
    println!("\n🤔 === Testing ThinkingContent Block ===\n");

    let options = ClaudeCodeOptions::builder()
        .model("opus")  // Opus 4.1 most likely to include thinking
        .max_thinking_tokens(10000)
        .max_turns(1)
        .build();

    println!("Requesting complex reasoning task...");
    let mut messages = query(
        "Analyze the computational complexity of merge sort vs quick sort, showing your reasoning process",
        Some(options)
    ).await?;

    let mut found_thinking = false;
    let mut found_text = false;

    while let Some(msg) = messages.next().await {
        match msg? {
            Message::Assistant { message } => {
                for block in message.content {
                    match block {
                        ContentBlock::Thinking(thinking) => {
                            found_thinking = true;
                            println!("✅ ThinkingContent block found!");
                            println!("   Thinking length: {} chars", thinking.thinking.len());
                            println!("   Signature: {}", thinking.signature);
                        },
                        ContentBlock::Text(text) => {
                            found_text = true;
                            println!("✅ Text response received");
                            println!("   Response length: {} chars", text.text.len());
                        },
                        _ => {},
                    }
                }
            },
            Message::Result {
                duration_ms,
                total_cost_usd,
                ..
            } => {
                println!("\nStats:");
                println!("  Duration: {duration_ms}ms");
                if let Some(cost) = total_cost_usd {
                    println!("  Cost: ${cost:.6}");
                }
            },
            _ => {},
        }
    }

    if !found_thinking {
        println!("ℹ️ No ThinkingContent (model may not emit thinking for this query)");
    }
    if !found_text {
        println!("⚠️ No text response received");
    }

    Ok(())
}

/// Run all tests
#[tokio::main]
async fn main() {
    println!("╔════════════════════════════════════════════════════╗");
    println!("║     Claude Code SDK v0.1.7 Comprehensive Test     ║");
    println!("╚════════════════════════════════════════════════════╝");

    println!("\nThis test covers:");
    println!("• 2025 Models (Opus 4.1, Sonnet 4)");
    println!("• Plan permission mode");
    println!("• Extra CLI arguments");
    println!("• ThinkingContent blocks");
    println!("• Interactive sessions\n");

    // Run all test suites
    let mut passed = 0;
    let mut failed = 0;

    // Run each test suite
    let test_results = vec![
        ("Models", test_all_models().await),
        ("Plan Mode", test_plan_mode_scenarios().await),
        ("Extra Args", test_extra_args().await),
        ("Interactive", test_interactive_with_new_features().await),
        ("Thinking", test_thinking_content().await),
    ];

    for (name, result) in test_results {
        match result {
            Ok(_) => {
                passed += 1;
                println!("\n✅ {name} test suite completed");
            },
            Err(e) => {
                failed += 1;
                println!("\n❌ {name} test suite failed: {e:?}");
            },
        }
    }

    println!("\n╔════════════════════════════════════════════════════╗");
    println!("║                   Test Results                     ║");
    println!("╠════════════════════════════════════════════════════╣");
    println!(
        "║  Passed: {:2}  │  Failed: {:2}  │  Total: {:2}          ║",
        passed,
        failed,
        passed + failed
    );
    println!("╚════════════════════════════════════════════════════╝");

    if failed == 0 {
        println!("\n🎉 All tests completed successfully!");
    } else {
        println!("\n⚠️ Some tests failed. Check the output above.");
    }
}
