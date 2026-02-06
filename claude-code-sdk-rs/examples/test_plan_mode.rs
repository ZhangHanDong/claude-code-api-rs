//! Test example for Plan permission mode (new in v0.1.7)
//!
//! Run with: cargo run --example test_plan_mode

use futures::StreamExt;
use nexus_claude::{ClaudeCodeOptions, InteractiveClient, Message, PermissionMode, Result, query};

async fn test_plan_mode_query() -> Result<()> {
    println!("\n=== Testing Plan Mode with Query ===");

    let options = ClaudeCodeOptions::builder()
        .model("sonnet")  // Use Sonnet 4 for faster response
        .permission_mode(PermissionMode::Plan)
        .max_turns(1)
        .build();

    println!("Using Plan permission mode for query...");
    let mut messages = query(
        "Create a plan for building a simple web server in Rust. Just outline the steps, don't implement.",
        Some(options)
    ).await?;

    let mut found_response = false;
    while let Some(msg) = messages.next().await {
        match msg? {
            Message::Assistant { message } => {
                for block in message.content {
                    if let nexus_claude::ContentBlock::Text(text) = block {
                        println!("Plan Response:\n{}", text.text);
                        found_response = true;
                    }
                }
            },
            Message::Result { duration_ms, .. } => {
                println!("\nPlan generated in: {duration_ms}ms");
            },
            _ => {},
        }
    }

    if found_response {
        println!("✅ Plan mode query works!");
    } else {
        println!("❌ Plan mode query failed");
    }

    Ok(())
}

async fn test_plan_mode_interactive() -> Result<()> {
    println!("\n=== Testing Plan Mode with Interactive Client ===");

    let options = ClaudeCodeOptions::builder()
        .model("sonnet")
        .permission_mode(PermissionMode::Plan)
        .system_prompt("You are a planning assistant. Create structured plans for tasks.")
        .max_turns(5)
        .build();

    let mut client = InteractiveClient::new(options)?;
    client.connect().await?;

    println!("Interactive client connected with Plan mode");

    // First planning request
    let messages = client
        .send_and_receive("Plan a REST API project structure for a todo application".to_string())
        .await?;

    let mut plan_received = false;
    for msg in &messages {
        if let Message::Assistant { message } = msg {
            for block in &message.content {
                if let nexus_claude::ContentBlock::Text(text) = block {
                    println!(
                        "Plan output (truncated): {}",
                        &text.text[..text.text.len().min(200)]
                    );
                    plan_received = true;
                }
            }
        }
    }

    if plan_received {
        // Follow-up planning request
        let messages = client
            .send_and_receive("Now create a plan for implementing authentication".to_string())
            .await?;

        for msg in &messages {
            if let Message::Assistant { message } = msg {
                for block in &message.content {
                    if let nexus_claude::ContentBlock::Text(text) = block {
                        println!(
                            "Follow-up plan (truncated): {}",
                            &text.text[..text.text.len().min(200)]
                        );
                    }
                }
            }
        }

        println!("✅ Interactive Plan mode works!");
    } else {
        println!("❌ Interactive Plan mode failed");
    }

    client.disconnect().await?;
    Ok(())
}

async fn compare_permission_modes() -> Result<()> {
    println!("\n=== Comparing Permission Modes ===");

    let modes = vec![
        (PermissionMode::Default, "Default"),
        (PermissionMode::AcceptEdits, "AcceptEdits"),
        (PermissionMode::Plan, "Plan"),
    ];

    for (mode, name) in modes {
        println!("\nTesting {name} mode:");

        let options = ClaudeCodeOptions::builder()
            .model("sonnet")  // Use latest Sonnet
            .permission_mode(mode)
            .max_turns(1)
            .build();

        match query("What permission mode are you using?", Some(options)).await {
            Ok(mut stream) => {
                let mut success = false;
                while let Some(msg) = stream.next().await {
                    if let Ok(Message::Assistant { .. }) = msg {
                        success = true;
                    }
                }
                if success {
                    println!("✅ {name} mode is supported");
                } else {
                    println!("⚠️ {name} mode - no response");
                }
            },
            Err(e) => {
                println!("❌ {name} mode error: {e:?}");
            },
        }
    }

    Ok(())
}

async fn test_plan_with_thinking_tokens() -> Result<()> {
    println!("\n=== Testing Plan Mode with Extended Thinking ===");

    let options = ClaudeCodeOptions::builder()
        .model("opus")  // Opus 4.1 has best thinking capabilities
        .permission_mode(PermissionMode::Plan)
        .max_thinking_tokens(10000)  // Allow extended thinking for planning
        .max_turns(1)
        .build();

    println!("Using Plan mode with extended thinking tokens...");
    let mut messages = query(
        "Create a comprehensive plan for migrating a large monolithic application to microservices. Consider all technical and organizational aspects.",
        Some(options)
    ).await?;

    let mut found_response = false;
    let mut found_thinking = false;

    while let Some(msg) = messages.next().await {
        match msg? {
            Message::Assistant { message } => {
                for block in message.content {
                    match block {
                        nexus_claude::ContentBlock::Text(text) => {
                            println!(
                                "Plan with thinking (first 300 chars): {}",
                                &text.text[..text.text.len().min(300)]
                            );
                            found_response = true;
                        },
                        nexus_claude::ContentBlock::Thinking(thinking) => {
                            println!(
                                "Thinking process detected (first 200 chars): {}",
                                &thinking.thinking[..thinking.thinking.len().min(200)]
                            );
                            found_thinking = true;
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
                println!("\nPlan generated in: {duration_ms}ms");
                if let Some(cost) = total_cost_usd {
                    println!("Cost: ${cost:.6}");
                }
            },
            _ => {},
        }
    }

    if found_response {
        println!("✅ Plan mode with thinking works!");
        if found_thinking {
            println!("✅ Thinking content was captured!");
        }
    } else {
        println!("❌ Plan mode with thinking failed");
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    println!("=== Claude Code SDK - Testing Plan Permission Mode ===");
    println!("Plan mode is new in v0.1.7 and is used for planning tasks\n");

    // Test Plan mode in different contexts
    if let Err(e) = test_plan_mode_query().await {
        println!("Plan mode query test error: {e:?}");
    }

    if let Err(e) = test_plan_mode_interactive().await {
        println!("Plan mode interactive test error: {e:?}");
    }

    if let Err(e) = compare_permission_modes().await {
        println!("Permission modes comparison error: {e:?}");
    }

    if let Err(e) = test_plan_with_thinking_tokens().await {
        println!("Plan with thinking test error: {e:?}");
    }

    println!("\n=== All Plan mode tests completed ===");
}
