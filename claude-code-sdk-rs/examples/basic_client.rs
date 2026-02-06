//! Most basic client test
//!
//! Tests the absolute minimum functionality.

use futures::StreamExt;
use nexus_claude::{ClaudeCodeOptions, ClaudeSDKClient, Message, Result};

#[tokio::main]
async fn main() -> Result<()> {
    println!("Basic Client Test\n");

    // Minimal setup
    let options = ClaudeCodeOptions::default();
    let mut client = ClaudeSDKClient::new(options);

    // Connect
    println!("Connecting...");
    client.connect(None).await?;
    println!("Connected!");

    // Send message
    println!("\nSending: What is 1+1?");
    client
        .send_request("What is 1+1?".to_string(), None)
        .await?;

    // Receive with timeout
    println!("Waiting for response (max 15 seconds)...");

    let result = tokio::time::timeout(std::time::Duration::from_secs(15), async {
        let mut messages = client.receive_messages().await;
        let mut count = 0;

        while let Some(msg) = messages.next().await {
            count += 1;
            println!("\nMessage {count}: ");

            match msg {
                Ok(Message::Assistant { message }) => {
                    for block in &message.content {
                        if let nexus_claude::ContentBlock::Text(text) = block {
                            println!("Assistant: {}", text.text);
                        }
                    }
                },
                Ok(Message::System { subtype, .. }) => {
                    println!("System: {subtype}");
                },
                Ok(Message::Result { duration_ms, .. }) => {
                    println!("Completed in {duration_ms}ms");
                    return Ok(());
                },
                Ok(Message::User { .. }) => {
                    println!("User message (unexpected)");
                },
                Err(e) => {
                    println!("Error: {e}");
                    return Err(e);
                },
            }
        }

        println!("Stream ended without result message");
        Ok(())
    })
    .await;

    match result {
        Ok(Ok(())) => println!("\nSuccess!"),
        Ok(Err(e)) => println!("\nError: {e}"),
        Err(_) => println!("\nTimeout - no response within 15 seconds"),
    }

    // Disconnect
    println!("\nDisconnecting...");
    client.disconnect().await?;
    println!("Done!");

    Ok(())
}
