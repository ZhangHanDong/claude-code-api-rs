//! Example demonstrating streaming output support in Rust SDK
//!
//! This example shows how to use the new streaming output methods
//! similar to Python SDK's streaming capabilities.

use futures::StreamExt;
use nexus_claude::{ClaudeCodeOptions, InteractiveClient, Message, Result};
use tokio::pin;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing for debug output
    tracing_subscriber::fmt()
        .with_env_filter("nexus_claude=debug")
        .init();

    println!("=== Rust SDK Streaming Output Example ===\n");

    // Create client with custom options
    let options = ClaudeCodeOptions::builder()
        .system_prompt("You are a helpful assistant")
        .model("claude-sonnet-4-20250514")
        .build();

    let mut client = InteractiveClient::new(options)?;

    // Connect to Claude
    println!("Connecting to Claude...");
    client.connect().await?;
    println!("Connected!\n");

    // Example 1: Basic streaming
    println!("--- Example 1: Basic Streaming ---");
    println!("User: What is 2 + 2?");
    client.send_message("What is 2 + 2?".to_string()).await?;

    // Receive messages as a stream
    {
        let stream = client.receive_messages_stream().await;
        pin!(stream);
        while let Some(result) = stream.next().await {
            match result {
                Ok(message) => {
                    display_message(&message);
                    if matches!(message, Message::Result { .. }) {
                        break;
                    }
                },
                Err(e) => eprintln!("Error: {e}"),
            }
        }
    }

    println!("\n--- Example 2: Using receive_response_stream ---");
    println!("User: Tell me a short joke");
    client
        .send_message("Tell me a short joke".to_string())
        .await?;

    // Use the convenience method that stops at Result message
    {
        let stream = client.receive_response_stream().await;
        pin!(stream);
        while let Some(result) = stream.next().await {
            match result {
                Ok(message) => display_message(&message),
                Err(e) => eprintln!("Error: {e}"),
            }
        }
    }

    println!("\n--- Example 3: Multi-turn Conversation with Streaming ---");

    // First question
    println!("User: What's the capital of France?");
    client
        .send_message("What's the capital of France?".to_string())
        .await?;

    {
        let stream = client.receive_response_stream().await;
        pin!(stream);
        while let Some(result) = stream.next().await {
            match result {
                Ok(message) => display_message(&message),
                Err(e) => eprintln!("Error: {e}"),
            }
        }
    }

    // Follow-up question
    println!("\nUser: What's the population of that city?");
    client
        .send_message("What's the population of that city?".to_string())
        .await?;

    {
        let stream = client.receive_response_stream().await;
        pin!(stream);
        while let Some(result) = stream.next().await {
            match result {
                Ok(message) => display_message(&message),
                Err(e) => eprintln!("Error: {e}"),
            }
        }
    }

    println!("\n--- Example 4: Concurrent Message Processing ---");
    println!("User: List 3 programming languages");
    client
        .send_message("List 3 programming languages briefly".to_string())
        .await?;

    // Process messages as they arrive
    let message_count = {
        let stream = client.receive_messages_stream().await;
        pin!(stream);
        let mut count = 0;

        while let Some(result) = stream.next().await {
            match result {
                Ok(message) => {
                    count += 1;
                    println!("[Message {}] Type: {}", count, message_type(&message));
                    display_message(&message);

                    if matches!(message, Message::Result { .. }) {
                        break;
                    }
                },
                Err(e) => {
                    eprintln!("Error: {e}");
                    break;
                },
            }
        }
        count
    };

    println!("\nTotal messages received: {message_count}");

    // Disconnect
    println!("\nDisconnecting...");
    client.disconnect().await?;
    println!("Disconnected!");

    Ok(())
}

/// Helper function to display messages
fn display_message(msg: &Message) {
    match msg {
        Message::User { message } => {
            println!("User: {}", message.content);
        },
        Message::Assistant { message } => {
            for block in &message.content {
                if let nexus_claude::ContentBlock::Text(text_content) = block {
                    println!("Claude: {}", text_content.text);
                }
            }
        },
        Message::System { .. } => {
            // Skip system messages in output
        },
        Message::Result { total_cost_usd, .. } => {
            println!("=== Result ===");
            if let Some(cost) = total_cost_usd {
                println!("Total cost: ${cost:.4} USD");
            }
        },
    }
}

/// Helper function to get message type as string
fn message_type(msg: &Message) -> &str {
    match msg {
        Message::User { .. } => "User",
        Message::Assistant { .. } => "Assistant",
        Message::System { .. } => "System",
        Message::Result { .. } => "Result",
    }
}
