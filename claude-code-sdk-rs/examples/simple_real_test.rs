//! Simple real API test - the easiest way to test

use futures::StreamExt;
use nexus_claude::{ClaudeCodeOptions, PermissionMode, Result, query};

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Simple Real API Test ===\n");

    // Method 1: Using the simplest query function
    println!("Method 1: Simple query function");
    println!("Query: What is 2 + 2?");

    match query("What is 2 + 2?", None).await {
        Ok(mut stream) => {
            while let Some(result) = stream.next().await {
                match result {
                    Ok(msg) => println!("Message: {msg:?}"),
                    Err(e) => println!("Stream error: {e}"),
                }
            }
        },
        Err(e) => println!("Query error: {e}"),
    }

    println!("\n---\n");

    // Method 2: Using query with options
    println!("Method 2: Query with custom options");
    let options = ClaudeCodeOptions::builder()
        .permission_mode(PermissionMode::AcceptEdits)
        .model("claude-3.5-sonnet")
        .build();

    println!("Query: Write a haiku about programming");

    match query("Write a haiku about programming", Some(options)).await {
        Ok(mut stream) => {
            while let Some(result) = stream.next().await {
                match result {
                    Ok(msg) => {
                        // Extract text content from assistant messages
                        if let nexus_claude::Message::Assistant { message } = msg {
                            for content in message.content {
                                if let nexus_claude::ContentBlock::Text(text) = content {
                                    println!("Response: {}", text.text);
                                }
                            }
                        }
                    },
                    Err(e) => println!("Stream error: {e}"),
                }
            }
        },
        Err(e) => println!("Query error: {e}"),
    }

    Ok(())
}
