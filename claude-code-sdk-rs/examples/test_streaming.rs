//! Test streaming functionality
//!
//! This example tests the streaming capabilities of the SDK

use futures::StreamExt;
use nexus_claude::{ClaudeCodeOptions, ClaudeSDKClient, Message, Result};

#[tokio::main]
async fn main() -> Result<()> {
    // Set up logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("nexus_claude=debug".parse().unwrap()),
        )
        .init();

    println!("=== Testing Streaming Functionality ===\n");

    // Test 1: Basic receive_messages
    println!("Test 1: Basic receive_messages");
    {
        let mut client = ClaudeSDKClient::new(ClaudeCodeOptions::default());

        match client.connect(Some("What is 2+2?".to_string())).await {
            Ok(_) => {
                println!("✅ Connected successfully");

                // Test receive_messages
                let mut messages = client.receive_messages().await;
                let mut message_count = 0;

                while let Some(msg_result) = messages.next().await {
                    match msg_result {
                        Ok(msg) => {
                            message_count += 1;
                            match &msg {
                                Message::User { message } => {
                                    println!("  📥 User: {}", message.content);
                                },
                                Message::Assistant { message } => {
                                    println!(
                                        "  🤖 Assistant: {} content blocks",
                                        message.content.len()
                                    );
                                },
                                Message::System { subtype, .. } => {
                                    println!("  ⚙️ System: {subtype}");
                                },
                                Message::Result { is_error, .. } => {
                                    println!("  ✓ Result (error: {is_error})");
                                    break;
                                },
                            }
                        },
                        Err(e) => {
                            println!("  ❌ Error receiving message: {e}");
                            break;
                        },
                    }
                }

                println!("  Total messages received: {message_count}");
                client.disconnect().await?;
            },
            Err(e) => {
                println!("❌ Failed to connect: {e}");
            },
        }
    }

    println!();

    // Test 2: receive_response helper
    println!("Test 2: receive_response helper");
    {
        let mut client = ClaudeSDKClient::new(ClaudeCodeOptions::default());

        match client.connect(None).await {
            Ok(_) => {
                println!("✅ Connected successfully");

                // Send a query
                if let Err(e) = client
                    .query("What is the capital of France?".to_string(), None)
                    .await
                {
                    println!("❌ Failed to send query: {e}");
                } else {
                    println!("✅ Query sent");

                    // Use receive_response which stops after ResultMessage
                    let mut response = client.receive_response().await;
                    let mut message_count = 0;

                    while let Some(msg_result) = response.next().await {
                        message_count += 1;
                        match msg_result {
                            Ok(Message::Result { .. }) => {
                                println!("  ✓ Received ResultMessage (stream should stop)");
                                break;
                            },
                            Ok(_) => {
                                println!("  📦 Received message #{message_count}");
                            },
                            Err(e) => {
                                println!("  ❌ Error: {e}");
                                break;
                            },
                        }
                    }

                    println!("  Total messages in response: {message_count}");
                }

                client.disconnect().await?;
            },
            Err(e) => {
                println!("❌ Failed to connect: {e}");
            },
        }
    }

    println!();

    // Test 3: Multiple queries in sequence
    println!("Test 3: Multiple sequential queries");
    {
        let mut client = ClaudeSDKClient::new(ClaudeCodeOptions::default());

        match client.connect(None).await {
            Ok(_) => {
                println!("✅ Connected successfully");

                let queries = ["What is 1+1?", "What is 2+2?", "What is 3+3?"];

                for (i, query) in queries.iter().enumerate() {
                    println!("  Query {}: {}", i + 1, query);

                    if let Err(e) = client.query(query.to_string(), None).await {
                        println!("    ❌ Failed to send: {e}");
                        continue;
                    }

                    // Receive response
                    let mut response = client.receive_response().await;
                    let mut got_result = false;

                    while let Some(msg_result) = response.next().await {
                        if let Ok(Message::Result { .. }) = msg_result {
                            got_result = true;
                            break;
                        }
                    }

                    if got_result {
                        println!("    ✅ Response received");
                    } else {
                        println!("    ⚠️ No ResultMessage received");
                    }
                }

                client.disconnect().await?;
            },
            Err(e) => {
                println!("❌ Failed to connect: {e}");
            },
        }
    }

    println!("\n=== Streaming Tests Complete ===");

    Ok(())
}
