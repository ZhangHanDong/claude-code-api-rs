//! Simple query example
//!
//! This example demonstrates how to use the simple `query` function
//! for one-shot interactions with Claude.

use futures::StreamExt;
use nexus_claude::{ClaudeCodeOptions, Message, Result, query};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("nexus_claude=debug,simple_query=info")
        .init();

    println!("Claude Code SDK - Simple Query Example\n");

    // Example 1: Basic query
    println!("Example 1: Basic query");
    println!("----------------------");

    let mut messages = query("What is 2 + 2?", None).await?;

    while let Some(msg) = messages.next().await {
        match msg? {
            Message::Assistant { message } => {
                for block in &message.content {
                    println!("Assistant: {block:?}");
                }
            },
            Message::Result { duration_ms, .. } => {
                println!("Query completed in {duration_ms}ms");
                break;
            },
            _ => {},
        }
    }

    println!("\n");

    // Example 2: Query with options
    println!("Example 2: Query with custom options");
    println!("------------------------------------");

    let options = ClaudeCodeOptions::builder()
        .system_prompt("You are a helpful coding assistant. Keep responses concise.")
        .model("sonnet")
        .max_thinking_tokens(1000)
        .build();

    let mut messages = query("Show me a hello world program in Rust", Some(options)).await?;

    while let Some(msg) = messages.next().await {
        match msg? {
            Message::Assistant { message } => {
                for block in &message.content {
                    println!("Assistant: {block:?}");
                }
            },
            Message::System { subtype, .. } => {
                println!("System: {subtype}");
            },
            Message::Result {
                duration_ms,
                total_cost_usd,
                ..
            } => {
                println!("\nQuery completed in {duration_ms}ms");
                if let Some(cost) = total_cost_usd {
                    println!("Cost: ${cost:.4}");
                }
                break;
            },
            _ => {},
        }
    }

    Ok(())
}
