//! Simple test for interactive client

use claude_code_sdk::{ClaudeSDKClient, ClaudeCodeOptions, Result};
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<()> {
    // Set up simple println-based debugging
    std::env::set_var("RUST_LOG", "claude_code_sdk=debug");
    
    println!("Creating client with default options...");
    let options = ClaudeCodeOptions::default();
    let mut client = ClaudeSDKClient::new(options);
    
    println!("Connecting...");
    client.connect(None).await?;
    println!("Connected!");
    
    println!("Sending message: What is 1+1?");
    client.send_request("What is 1+1?".to_string(), None).await?;
    println!("Message sent!");
    
    println!("Receiving messages...");
    let mut message_count = 0;
    let mut stream = client.receive_messages().await;
    
    while let Some(result) = stream.next().await {
        match result {
            Ok(message) => {
                message_count += 1;
                println!("Message {}: {:?}", message_count, message);
                
                // Check if it's a result message
                if matches!(message, claude_code_sdk::Message::Result { .. }) {
                    println!("Got result message, stopping...");
                    break;
                }
            }
            Err(e) => {
                eprintln!("Error receiving message: {}", e);
                break;
            }
        }
    }
    
    println!("Disconnecting...");
    client.disconnect().await?;
    println!("Done!");
    
    Ok(())
}