//! Example demonstrating model selection with 2025 models
//!
//! This example shows how to use Opus 4.1 and Sonnet 4 models
//! with proper fallback handling.

use futures::StreamExt;
use nexus_claude::{ClaudeCodeOptions, InteractiveClient, Message, Result, query};
use std::env;

/// Test if a model is available
async fn test_model_availability(model: &str) -> bool {
    println!("Testing model: {model}");

    let options = ClaudeCodeOptions::builder()
        .model(model)
        .max_turns(1)
        .build();

    match query("Reply with 'OK' if you're working", Some(options)).await {
        Ok(mut stream) => {
            while let Some(result) = stream.next().await {
                if let Ok(Message::Assistant { .. }) = result {
                    println!("  ✓ {model} is available");
                    return true;
                }
            }
            println!("  ✗ {model} - no response");
            false
        },
        Err(e) => {
            println!("  ✗ {model} - error: {e:?}");
            false
        },
    }
}

/// Example using Opus 4.1
async fn use_opus_4_1() -> Result<()> {
    println!("\n=== Using Opus 4.1 ===");

    let options = ClaudeCodeOptions::builder()
        .model("opus")  // or "claude-opus-4-1-20250805"
        .max_thinking_tokens(15000)  // Opus 4.1 supports extended thinking
        .system_prompt("You are an expert Rust developer")
        .build();

    let mut messages = query(
        "What model are you? What are your key capabilities compared to previous versions?",
        Some(options),
    )
    .await?;

    while let Some(msg) = messages.next().await {
        match msg? {
            Message::Assistant { message } => {
                for block in message.content {
                    if let nexus_claude::ContentBlock::Text(text) = block {
                        println!("Opus 4.1: {}", text.text);
                    }
                }
            },
            Message::Result {
                total_cost_usd,
                duration_ms,
                ..
            } => {
                println!("Response time: {duration_ms}ms");
                if let Some(cost) = total_cost_usd {
                    println!("Cost: ${cost:.6}");
                }
            },
            _ => {},
        }
    }

    Ok(())
}

/// Example using Sonnet 4
async fn use_sonnet_4() -> Result<()> {
    println!("\n=== Using Sonnet 4 ===");

    let options = ClaudeCodeOptions::builder()
        .model("sonnet")  // or "claude-sonnet-4-20250514"
        .build();

    let mut messages = query(
        "What model are you? How do you compare to Opus 4.1?",
        Some(options),
    )
    .await?;

    while let Some(msg) = messages.next().await {
        if let Message::Assistant { message } = msg? {
            for block in message.content {
                if let nexus_claude::ContentBlock::Text(text) = block {
                    println!("Sonnet 4: {}", text.text);
                }
            }
        }
    }

    Ok(())
}

/// Interactive session with model selection
async fn interactive_with_model_choice() -> Result<()> {
    println!("\n=== Interactive Session ===");
    println!("Available models:");
    println!("1. Opus 4.1 (Most capable)");
    println!("2. Sonnet 4 (Balanced)");
    println!("3. Latest Opus");
    println!("4. Latest Sonnet");

    print!("Select model (1-4): ");
    use std::io::{self, Write};
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();

    let model = match input.trim() {
        "1" => "opus",
        "2" => "sonnet",
        "3" => "opus",
        "4" => "sonnet",
        _ => {
            println!("Invalid choice, using Sonnet 4");
            "sonnet"
        },
    };

    println!("Using model: {model}");

    let options = ClaudeCodeOptions::builder().model(model).build();

    let mut client = InteractiveClient::new(options)?;
    client.connect().await?;

    // Send a test message
    let messages = client
        .send_and_receive(
            "Create a simple Rust function that calculates fibonacci numbers".to_string(),
        )
        .await?;

    for msg in messages {
        if let Message::Assistant { message } = msg {
            for block in message.content {
                if let nexus_claude::ContentBlock::Text(text) = block {
                    println!("{}", text.text);
                }
            }
        }
    }

    client.disconnect().await?;
    Ok(())
}

/// Example with automatic fallback
async fn with_fallback() -> Result<()> {
    println!("\n=== Query with Automatic Fallback ===");

    let models = vec!["opus", "sonnet", "sonnet", "sonnet"];
    let mut success = false;

    for model in models {
        println!("Trying model: {model}");

        let options = ClaudeCodeOptions::builder().model(model).build();

        match query("Say hello and tell me your model", Some(options)).await {
            Ok(mut stream) => {
                println!("Success with model: {model}");

                while let Some(msg) = stream.next().await {
                    if let Ok(Message::Assistant { message }) = msg {
                        for block in message.content {
                            if let nexus_claude::ContentBlock::Text(text) = block {
                                println!("{}", text.text);
                            }
                        }
                    }
                }

                success = true;
                break;
            },
            Err(e) => {
                println!("Failed with {model}: {e:?}");
                continue;
            },
        }
    }

    if !success {
        println!("All models failed!");
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Claude Code SDK - 2025 Models Example ===\n");

    // Check which models are available
    println!("Checking model availability...");
    let models_to_test = vec![
        "opus",
        "opus-4",
        "opus",
        "sonnet",
        "sonnet",
        "claude-opus-4-1-20250805",
        "claude-sonnet-4-20250514",
    ];

    let mut available_models = Vec::new();
    for model in models_to_test {
        if test_model_availability(model).await {
            available_models.push(model);
        }
    }

    println!("\nAvailable models: {available_models:?}");

    // Get command line argument
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 {
        match args[1].as_str() {
            "opus" => use_opus_4_1().await?,
            "sonnet" => use_sonnet_4().await?,
            "interactive" => interactive_with_model_choice().await?,
            "fallback" => with_fallback().await?,
            _ => {
                println!("Unknown command: {}", args[1]);
                println!("Usage: {} [opus|sonnet|interactive|fallback]", args[0]);
            },
        }
    } else {
        // Default: test all available models
        if available_models.contains(&"opus") {
            use_opus_4_1().await?;
        }

        if available_models.contains(&"sonnet") {
            use_sonnet_4().await?;
        }

        // Show fallback example
        with_fallback().await?;
    }

    Ok(())
}
