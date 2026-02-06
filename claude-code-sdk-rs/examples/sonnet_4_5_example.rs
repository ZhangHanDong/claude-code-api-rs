//! Example demonstrating Claude Sonnet 4.5 (latest model)
//!
//! This example shows how to use the newest Sonnet 4.5 model released in September 2025.
//! Sonnet 4.5 offers the best balance of performance, speed, and cost for most applications.

use futures::StreamExt;
use nexus_claude::model_recommendation::{balanced_model, latest_sonnet};
use nexus_claude::{ClaudeCodeOptions, InteractiveClient, Message, Result, query};

/// Simple query using Sonnet 4.5
async fn simple_query_example() -> Result<()> {
    println!("=== Simple Query with Sonnet 4.5 ===\n");

    let options = ClaudeCodeOptions::builder()
        .model("claude-sonnet-4-5-20250929")  // Latest Sonnet 4.5
        .build();

    let mut messages = query(
        "What are the key improvements in Claude Sonnet 4.5 compared to previous versions?",
        Some(options),
    )
    .await?;

    while let Some(msg) = messages.next().await {
        match msg? {
            Message::Assistant { message } => {
                for block in message.content {
                    if let nexus_claude::ContentBlock::Text(text) = block {
                        println!("{}", text.text);
                    }
                }
            },
            Message::Result {
                total_cost_usd,
                duration_ms,
                ..
            } => {
                println!("\n---");
                println!("Response time: {}ms", duration_ms);
                if let Some(cost) = total_cost_usd {
                    println!("Cost: ${:.6}", cost);
                }
            },
            _ => {},
        }
    }

    Ok(())
}

/// Interactive session with Sonnet 4.5
async fn interactive_session_example() -> Result<()> {
    println!("\n=== Interactive Session with Sonnet 4.5 ===\n");

    let options = ClaudeCodeOptions::builder()
        .model("claude-sonnet-4-5-20250929")
        .system_prompt("You are an expert Rust developer")
        .build();

    let mut client = InteractiveClient::new(options)?;
    client.connect().await?;

    // First query
    println!("Query 1: Creating a simple async function");
    let messages = client
        .send_and_receive(
            "Write a simple async function in Rust that fetches data from an API".to_string(),
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

    // Follow-up query
    println!("\n---\nQuery 2: Adding error handling");
    let messages = client
        .send_and_receive("Now add proper error handling to that function".to_string())
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

/// Using model recommendation helpers
async fn model_recommendation_example() -> Result<()> {
    println!("\n=== Using Model Recommendation Helpers ===\n");

    // Get latest Sonnet model programmatically
    let latest = latest_sonnet();
    println!("Latest Sonnet model: {}", latest);

    let balanced = balanced_model();
    println!("Balanced model (also Sonnet 4.5): {}", balanced);

    // Use recommended model
    let options = ClaudeCodeOptions::builder().model(latest).build();

    let mut messages = query(
        "What makes you a good choice for most applications?",
        Some(options),
    )
    .await?;

    while let Some(msg) = messages.next().await {
        if let Ok(Message::Assistant { message }) = msg {
            for block in message.content {
                if let nexus_claude::ContentBlock::Text(text) = block {
                    println!("{}", text.text);
                }
            }
        }
    }

    Ok(())
}

/// Comparing Sonnet 4.5 with other models
async fn model_comparison_example() -> Result<()> {
    println!("\n=== Model Comparison ===\n");

    let models = vec![
        ("claude-sonnet-4-5-20250929", "Sonnet 4.5 (Latest)"),
        ("claude-sonnet-4-20250514", "Sonnet 4"),
        ("claude-3-5-haiku-20241022", "Haiku 3.5"),
    ];

    let prompt = "What is 2 + 2? Reply in one sentence.";

    for (model, name) in models {
        println!("Testing {}", name);

        let start = std::time::Instant::now();
        let options = ClaudeCodeOptions::builder()
            .model(model)
            .max_turns(1)
            .build();

        match query(prompt, Some(options)).await {
            Ok(mut stream) => {
                while let Some(msg) = stream.next().await {
                    if let Ok(Message::Assistant { message }) = msg {
                        for block in message.content {
                            if let nexus_claude::ContentBlock::Text(text) = block {
                                println!("  Response: {}", text.text);
                            }
                        }
                    }
                }
                let elapsed = start.elapsed();
                println!("  Time: {:?}\n", elapsed);
            },
            Err(e) => {
                println!("  Error: {:?}\n", e);
            },
        }
    }

    Ok(())
}

/// Advanced features with Sonnet 4.5
async fn advanced_features_example() -> Result<()> {
    println!("\n=== Advanced Features with Sonnet 4.5 ===\n");

    let options = ClaudeCodeOptions::builder()
        .model("claude-sonnet-4-5-20250929")
        .max_thinking_tokens(8000)  // Sonnet 4.5 supports extended thinking
        .max_output_tokens(4000)    // Limit output tokens
        .max_turns(3)               // Limit conversation turns
        .permission_mode(nexus_claude::PermissionMode::AcceptEdits)
        .build();

    let mut messages = query(
        "Design a microservices architecture for a social media platform. Include key components and their interactions.",
        Some(options)
    ).await?;

    while let Some(msg) = messages.next().await {
        match msg? {
            Message::Assistant { message } => {
                for block in message.content {
                    match block {
                        nexus_claude::ContentBlock::Text(text) => {
                            println!("{}", text.text);
                        },
                        nexus_claude::ContentBlock::Thinking(thinking) => {
                            println!("\n[Thinking]: {}", thinking.thinking);
                        },
                        _ => {},
                    }
                }
            },
            Message::Result {
                usage,
                total_cost_usd,
                ..
            } => {
                println!("\n---");
                if let Some(usage_json) = usage {
                    println!("Token usage: {:?}", usage_json);
                }
                if let Some(cost) = total_cost_usd {
                    println!("Total cost: ${:.6}", cost);
                }
            },
            _ => {},
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("╔═══════════════════════════════════════════════╗");
    println!("║   Claude Sonnet 4.5 - Latest Model Example   ║");
    println!("╚═══════════════════════════════════════════════╝\n");

    // Get command line argument
    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 {
        match args[1].as_str() {
            "simple" => simple_query_example().await?,
            "interactive" => interactive_session_example().await?,
            "recommendation" => model_recommendation_example().await?,
            "compare" => model_comparison_example().await?,
            "advanced" => advanced_features_example().await?,
            _ => {
                println!("Unknown command: {}", args[1]);
                println!(
                    "Usage: {} [simple|interactive|recommendation|compare|advanced]",
                    args[0]
                );
                println!("\nRunning all examples...\n");
                run_all_examples().await?;
            },
        }
    } else {
        run_all_examples().await?;
    }

    Ok(())
}

async fn run_all_examples() -> Result<()> {
    simple_query_example().await?;
    interactive_session_example().await?;
    model_recommendation_example().await?;
    model_comparison_example().await?;
    advanced_features_example().await?;
    Ok(())
}
