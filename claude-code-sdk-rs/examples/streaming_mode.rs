//! Comprehensive examples of using ClaudeSDKClient for streaming mode.
//!
//! This file demonstrates various patterns for building applications with
//! the ClaudeSDKClient streaming interface, mirroring the Python SDK examples.
//!
//! Usage:
//! cargo run --example streaming_mode          # List examples
//! cargo run --example streaming_mode all      # Run all examples  
//! cargo run --example streaming_mode basic    # Run specific example

use futures::StreamExt;
use nexus_claude::{
    ClaudeCodeOptions, ClaudeSDKClient, ContentBlock, Message, Result, TextContent,
};
use std::env;
use std::time::Duration;
use tokio::time::sleep;

/// Display a message in a standardized format
fn display_message(msg: &Message) {
    match msg {
        Message::User { message } => {
            println!("User: {}", message.content);
        },
        Message::Assistant { message } => {
            for block in &message.content {
                if let ContentBlock::Text(TextContent { text }) = block {
                    println!("Claude: {text}");
                }
            }
        },
        Message::System { .. } => {
            // Ignore system messages
        },
        Message::Result { total_cost_usd, .. } => {
            println!("Result ended");
            if let Some(cost) = total_cost_usd {
                println!("Cost: ${cost:.4}");
            }
        },
    }
}

/// Basic streaming example with context manager pattern
async fn example_basic_streaming() -> Result<()> {
    println!("=== Basic Streaming Example ===");

    let mut client = ClaudeSDKClient::new(ClaudeCodeOptions::default());

    // Connect with empty stream (similar to Python's context manager)
    client.connect(None).await?;

    println!("User: What is 2+2?");
    client.query("What is 2+2?".to_string(), None).await?;

    // Receive complete response using the helper method
    {
        let mut response = client.receive_response().await;
        while let Some(msg_result) = response.next().await {
            match msg_result {
                Ok(msg) => display_message(&msg),
                Err(e) => eprintln!("Error: {e}"),
            }
        }
    } // response is dropped here, releasing the borrow

    client.disconnect().await?;
    println!();

    Ok(())
}

/// Multi-turn conversation using receive_response helper
async fn example_multi_turn_conversation() -> Result<()> {
    println!("=== Multi-Turn Conversation Example ===");

    let mut client = ClaudeSDKClient::new(ClaudeCodeOptions::default());
    client.connect(None).await?;

    // First turn
    println!("User: What's the capital of France?");
    client
        .query("What's the capital of France?".to_string(), None)
        .await?;

    // Extract and print response
    {
        let mut response = client.receive_response().await;
        while let Some(msg_result) = response.next().await {
            if let Ok(msg) = msg_result {
                display_message(&msg);
            }
        }
    }

    // Second turn - follow-up
    println!("\nUser: What's the population of that city?");
    client
        .query("What's the population of that city?".to_string(), None)
        .await?;

    {
        let mut response = client.receive_response().await;
        while let Some(msg_result) = response.next().await {
            if let Ok(msg) = msg_result {
                display_message(&msg);
            }
        }
    }

    client.disconnect().await?;
    println!();

    Ok(())
}

/// Handle responses while sending new messages
async fn example_concurrent_responses() -> Result<()> {
    println!("=== Concurrent Send/Receive Example ===");
    println!("Note: This example requires refactoring to support concurrent access");

    // For now, we'll simulate the pattern with sequential operations
    let mut client = ClaudeSDKClient::new(ClaudeCodeOptions::default());
    client.connect(None).await?;

    // Send multiple messages with delays
    let questions = vec![
        "What is 2 + 2?",
        "What is the square root of 144?",
        "What is 10% of 80?",
    ];

    for question in questions {
        println!("\nUser: {question}");
        client.query(question.to_string(), None).await?;

        // Receive response for this question
        {
            let mut response = client.receive_response().await;
            while let Some(msg_result) = response.next().await {
                if let Ok(msg) = msg_result {
                    display_message(&msg);
                }
            }
        }

        sleep(Duration::from_secs(1)).await;
    }

    client.disconnect().await?;
    println!();

    Ok(())
}

/// Demonstrate interrupt capability
async fn example_with_interrupt() -> Result<()> {
    println!("=== Interrupt Example ===");
    println!("IMPORTANT: Interrupts require active message consumption.");

    let mut client = ClaudeSDKClient::new(ClaudeCodeOptions::default());
    client.connect(None).await?;

    // Start a long-running task
    println!("\nUser: Count from 1 to 100 slowly");
    client
        .query(
            "Count from 1 to 100 slowly, with a brief pause between each number".to_string(),
            None,
        )
        .await?;

    // Start receiving messages in background
    // NOTE: In real usage, this would need proper concurrent handling
    let consume_handle = tokio::spawn(async move {
        // Simulating message consumption
        println!("[Message consumer started]");
    });

    // Wait a bit to let counting start
    sleep(Duration::from_secs(2)).await;

    // Send interrupt
    println!("\n[Sending interrupt after 2 seconds...]");
    match client.interrupt().await {
        Ok(_) => println!("[Interrupt sent successfully]"),
        Err(e) => eprintln!("[Failed to interrupt: {e}]"),
    }

    // Wait for the interrupted response to complete
    sleep(Duration::from_secs(1)).await;

    // Send new task
    println!("\nUser: What's 2 + 2?");
    client.query("What's 2 + 2?".to_string(), None).await?;

    // Get response
    {
        let mut response = client.receive_response().await;
        while let Some(msg_result) = response.next().await {
            if let Ok(msg) = msg_result {
                display_message(&msg);
            }
        }
    }

    consume_handle.abort();
    client.disconnect().await?;
    println!();

    Ok(())
}

/// Demonstrate session management
async fn example_with_sessions() -> Result<()> {
    println!("=== Session Management Example ===");

    let mut client = ClaudeSDKClient::new(ClaudeCodeOptions::default());
    client.connect(None).await?;

    // Session 1: Math context
    println!("Session 1 - Math:");
    client
        .query(
            "Let's do some math problems".to_string(),
            Some("math".to_string()),
        )
        .await?;

    {
        let mut response = client.receive_response().await;
        while let Some(msg_result) = response.next().await {
            if let Ok(msg) = msg_result {
                display_message(&msg);
            }
        }
    }

    // Session 2: History context
    println!("\nSession 2 - History:");
    client
        .query(
            "Tell me about ancient Rome".to_string(),
            Some("history".to_string()),
        )
        .await?;

    {
        let mut response = client.receive_response().await;
        while let Some(msg_result) = response.next().await {
            if let Ok(msg) = msg_result {
                display_message(&msg);
            }
        }
    }

    // Back to Session 1
    println!("\nBack to Session 1 - Math:");
    client
        .query("What's 15% of 200?".to_string(), Some("math".to_string()))
        .await?;

    {
        let mut response = client.receive_response().await;
        while let Some(msg_result) = response.next().await {
            if let Ok(msg) = msg_result {
                display_message(&msg);
            }
        }
    }

    // List active sessions
    let sessions = client.get_sessions().await;
    println!("\nActive sessions: {sessions:?}");

    client.disconnect().await?;
    println!();

    Ok(())
}

/// Main function to run examples
#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("Usage: {} <example_name>", args[0]);
        println!("\nAvailable examples:");
        println!("  basic        - Basic streaming example");
        println!("  multi_turn   - Multi-turn conversation");
        println!("  concurrent   - Concurrent send/receive");
        println!("  interrupt    - Interrupt demonstration");
        println!("  sessions     - Session management");
        println!("  all          - Run all examples");
        return Ok(());
    }

    let example = &args[1];

    match example.as_str() {
        "basic" => example_basic_streaming().await,
        "multi_turn" => example_multi_turn_conversation().await,
        "concurrent" => example_concurrent_responses().await,
        "interrupt" => example_with_interrupt().await,
        "sessions" => example_with_sessions().await,
        "all" => {
            example_basic_streaming().await?;
            example_multi_turn_conversation().await?;
            example_concurrent_responses().await?;
            example_with_interrupt().await?;
            example_with_sessions().await
        },
        _ => {
            eprintln!("Unknown example: {example}");
            eprintln!("Run without arguments to see available examples");
            return Ok(());
        },
    }
}
