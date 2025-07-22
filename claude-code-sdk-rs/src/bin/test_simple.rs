//! Test the simple interactive client

use claude_code_sdk::{SimpleInteractiveClient, ClaudeCodeOptions, Result, ContentBlock};

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Testing SimpleInteractiveClient ===\n");
    
    let mut client = SimpleInteractiveClient::new(ClaudeCodeOptions::default())?;
    
    // Test 1: Connect
    println!("Test 1: Connect");
    client.connect().await?;
    println!("✓ Connected\n");
    
    // Test 2: Send and receive
    println!("Test 2: Send message and receive response");
    let messages = client.send_and_receive("What is 2 + 2?".to_string()).await?;
    println!("✓ Got {} messages", messages.len());
    
    for msg in &messages {
        match msg {
            claude_code_sdk::Message::Assistant { message } => {
                for content in &message.content {
                    if let ContentBlock::Text(text) = content {
                        println!("  Claude says: {}", text.text);
                    }
                }
            }
            claude_code_sdk::Message::Result { .. } => {
                println!("  Got result message");
            }
            _ => {}
        }
    }
    
    // Test 3: Follow-up
    println!("\nTest 3: Follow-up question");
    let messages = client.send_and_receive("Now what is 10 + 10?".to_string()).await?;
    println!("✓ Got {} messages", messages.len());
    
    for msg in &messages {
        match msg {
            claude_code_sdk::Message::Assistant { message } => {
                for content in &message.content {
                    if let ContentBlock::Text(text) = content {
                        println!("  Claude says: {}", text.text);
                    }
                }
            }
            _ => {}
        }
    }
    
    // Test 4: Another follow-up
    println!("\nTest 4: Another question");
    let messages = client.send_and_receive("What's the capital of France?".to_string()).await?;
    println!("✓ Got {} messages", messages.len());
    
    for msg in &messages {
        match msg {
            claude_code_sdk::Message::Assistant { message } => {
                for content in &message.content {
                    if let ContentBlock::Text(text) = content {
                        println!("  Claude says: {}", text.text);
                    }
                }
            }
            _ => {}
        }
    }
    
    // Test 5: Disconnect
    println!("\nTest 5: Disconnect");
    client.disconnect().await?;
    println!("✓ Disconnected");
    
    println!("\n=== All tests passed! Interactive mode works! ===");
    Ok(())
}