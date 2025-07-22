//! Test the working client

use claude_code_sdk::{ClaudeSDKClientWorking, ClaudeCodeOptions, Result};

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Testing ClaudeSDKClientWorking ===\n");
    
    // Test 1: Basic interaction
    println!("Test 1: Basic interaction");
    let mut client = ClaudeSDKClientWorking::new(ClaudeCodeOptions::default());
    
    println!("  Connecting...");
    client.connect(None).await?;
    println!("  ✓ Connected");
    
    println!("  Sending: What is 2 + 2?");
    client.send_user_message("What is 2 + 2?".to_string()).await?;
    println!("  ✓ Message sent");
    
    println!("  Receiving response...");
    let messages = client.receive_response().await?;
    println!("  ✓ Got {} messages", messages.len());
    
    for (i, msg) in messages.iter().enumerate() {
        println!("    Message {}: {:?}", i + 1, msg);
    }
    
    // Test 2: Follow-up
    println!("\nTest 2: Follow-up message");
    println!("  Sending: Now what is 10 + 10?");
    client.send_user_message("Now what is 10 + 10?".to_string()).await?;
    println!("  ✓ Message sent");
    
    let messages = client.receive_response().await?;
    println!("  ✓ Got {} messages", messages.len());
    
    // Test 3: Another follow-up
    println!("\nTest 3: Another follow-up");
    println!("  Sending: What's the capital of France?");
    client.send_user_message("What's the capital of France?".to_string()).await?;
    println!("  ✓ Message sent");
    
    let messages = client.receive_response().await?;
    println!("  ✓ Got {} messages", messages.len());
    
    // Extract the answer
    for msg in &messages {
        if let claude_code_sdk::Message::Assistant { message } = msg {
            for content in &message.content {
                match content {
                    claude_code_sdk::ContentBlock::Text(text) => {
                        println!("  Answer: {}", text.text);
                    }
                    _ => {}
                }
            }
        }
    }
    
    // Test 4: Disconnect
    println!("\nTest 4: Disconnect");
    client.disconnect().await?;
    println!("  ✓ Disconnected");
    
    println!("\n=== All tests passed! Interactive mode works! ===");
    Ok(())
}