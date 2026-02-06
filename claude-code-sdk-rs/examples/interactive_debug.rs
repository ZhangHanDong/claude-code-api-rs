//! Debug version of interactive client
//!
//! This example includes more debugging output to diagnose issues.

use futures::StreamExt;
use nexus_claude::{ClaudeCodeOptions, ClaudeSDKClient, Message, PermissionMode, Result};
use std::io::{self, Write};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging with trace level
    tracing_subscriber::fmt()
        .with_env_filter("nexus_claude=trace")
        .with_max_level(tracing::Level::TRACE)
        .init();

    println!("Claude Code SDK - Interactive Client (Debug Mode)\n");

    // Create client with options
    let options = ClaudeCodeOptions::builder()
        .system_prompt("You are a helpful assistant.")
        .permission_mode(PermissionMode::AcceptEdits)
        .model("sonnet")
        .build();

    let mut client = ClaudeSDKClient::new(options);

    // Connect to Claude
    println!("Connecting to Claude CLI...");
    client.connect(None).await?;
    println!("Connected successfully!\n");

    // Send first message immediately to test
    println!("Sending test message: 'Hello'");
    client.send_request("Hello".to_string(), None).await?;
    println!("Message sent, waiting for response...\n");

    // Try to receive with timeout
    let mut messages = client.receive_messages().await;
    let mut got_response = false;
    let mut message_count = 0;

    println!("Starting to receive messages...");

    // Set a timeout for the first response
    let timeout = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        while let Some(msg) = messages.next().await {
            message_count += 1;
            println!("Received message #{message_count}");

            match msg {
                Ok(message) => {
                    println!("Message type: {:?}", std::mem::discriminant(&message));

                    match message {
                        Message::Assistant { message } => {
                            got_response = true;
                            print!("Claude: ");
                            for block in &message.content {
                                if let nexus_claude::ContentBlock::Text(text) = block {
                                    print!("{}", text.text);
                                }
                            }
                            println!();
                        },
                        Message::System { subtype, .. } => {
                            println!("[System: {subtype}]");
                        },
                        Message::Result { duration_ms, .. } => {
                            println!("[Response time: {duration_ms}ms]");
                            return Ok(());
                        },
                        _ => {
                            println!("[Other message type]");
                        },
                    }
                },
                Err(e) => {
                    println!("Error receiving message: {e}");
                    return Err(e);
                },
            }
        }
        Ok(())
    })
    .await;

    match timeout {
        Ok(Ok(())) => {
            if got_response {
                println!("\nInitial test successful! Starting interactive mode.\n");
            } else {
                println!("\nWarning: No response received for test message.\n");
            }
        },
        Ok(Err(e)) => {
            println!("\nError during initial test: {e}");
            println!("Continuing anyway...\n");
        },
        Err(_) => {
            println!("\nTimeout: No response received within 10 seconds.");
            println!("This might indicate a connection issue.\n");
            println!("Continuing anyway...\n");
        },
    }

    // Interactive loop
    println!("Type 'quit' to exit\n");
    let stdin = io::stdin();
    let mut input = String::new();

    loop {
        print!("You: ");
        io::stdout().flush()?;

        input.clear();
        stdin.read_line(&mut input)?;

        let input = input.trim();

        if input.is_empty() {
            continue;
        }

        if input == "quit" {
            break;
        }

        // Send message
        println!("[Debug] Sending: {input}");
        client.send_request(input.to_string(), None).await?;
        println!("[Debug] Message sent");

        // Receive response
        let mut messages = client.receive_messages().await;
        let mut got_response = false;

        println!("[Debug] Waiting for response...");

        while let Some(msg) = messages.next().await {
            match msg? {
                Message::Assistant { message } => {
                    if !got_response {
                        print!("Claude: ");
                        got_response = true;
                    }

                    for block in &message.content {
                        if let nexus_claude::ContentBlock::Text(text) = block {
                            print!("{}", text.text);
                        }
                    }
                },
                Message::Result { duration_ms, .. } => {
                    println!("\n[Response time: {duration_ms}ms]\n");
                    break;
                },
                _ => {},
            }
        }

        if !got_response {
            println!("[Warning] No response received\n");
        }
    }

    // Disconnect
    println!("\nDisconnecting...");
    client.disconnect().await?;
    println!("Goodbye!");

    Ok(())
}
