//! Minimal test for ClaudeSDKClient
//!
//! This example tests basic client connectivity.

use futures::StreamExt;
use nexus_claude::{ClaudeCodeOptions, ClaudeSDKClient, Message, Result};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging with debug level
    tracing_subscriber::fmt()
        .with_env_filter("nexus_claude=debug")
        .with_max_level(tracing::Level::DEBUG)
        .init();

    println!("Testing ClaudeSDKClient...\n");

    // Minimal options
    let options = ClaudeCodeOptions::default();

    let mut client = ClaudeSDKClient::new(options);

    // Connect without initial prompt
    println!("Connecting (no initial prompt)...");
    client.connect(None).await?;
    println!("Connected!\n");

    // Send a simple message
    println!("Sending: What is 2 + 2?");
    client
        .send_request("What is 2 + 2?".to_string(), None)
        .await?;

    // Receive response
    println!("Waiting for response...");
    let mut messages = client.receive_messages().await;
    let mut message_count = 0;

    while let Some(msg) = messages.next().await {
        message_count += 1;

        match msg {
            Ok(ref message) => {
                println!("Received message #{message_count}: {message:?}");

                // Break on result message
                if let Message::Result { .. } = message {
                    break;
                }
            },
            Err(e) => {
                println!("Error receiving message: {e}");
                break;
            },
        }

        // Safety: break after 10 messages to avoid infinite loop
        if message_count > 10 {
            println!("Breaking after 10 messages");
            break;
        }
    }

    // Disconnect
    println!("\nDisconnecting...");
    client.disconnect().await?;
    println!("Done!");

    Ok(())
}
