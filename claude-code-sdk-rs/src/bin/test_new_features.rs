//! Test new features added for Python SDK parity

use claude_code_sdk::{SimpleInteractiveClient, ClaudeCodeOptions, Result, ContentBlock};

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Testing New Features ===\n");
    
    let mut client = SimpleInteractiveClient::new(ClaudeCodeOptions::default())?;
    
    // Test 1: Connect
    println!("Test 1: Connect");
    client.connect().await?;
    println!("✓ Connected\n");
    
    // Test 2: send_message and receive_response (like Python SDK)
    println!("Test 2: Separate send_message and receive_response");
    client.send_message("What is 5 + 5?".to_string()).await?;
    println!("✓ Message sent");
    
    let messages = client.receive_response().await?;
    println!("✓ Got {} messages", messages.len());
    for msg in &messages {
        if let claude_code_sdk::Message::Assistant { message } = msg {
            for content in &message.content {
                if let ContentBlock::Text(text) = content {
                    println!("  Claude says: {}", text.text);
                }
            }
        }
    }
    
    // Test 3: Test interrupt (send a complex query then interrupt)
    println!("\nTest 3: Interrupt functionality");
    println!("  Sending complex query...");
    client.send_message("Write a very detailed explanation of quantum mechanics, covering all fundamental concepts, mathematical formulations, and practical applications.".to_string()).await?;
    
    // Wait a bit then interrupt
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    println!("  Sending interrupt...");
    match client.interrupt().await {
        Ok(()) => println!("✓ Interrupt sent successfully"),
        Err(e) => println!("  Interrupt error: {}", e),
    }
    
    // Try to receive whatever was generated before interrupt
    println!("  Checking for partial response...");
    match client.receive_response().await {
        Ok(messages) => {
            println!("  Got {} messages before interrupt", messages.len());
            for msg in &messages {
                if let claude_code_sdk::Message::Result { result, .. } = msg {
                    if let Some(result) = result {
                        if result.contains("interrupt") || result.contains("cancelled") {
                            println!("✓ Interrupt confirmed in result");
                        }
                    }
                }
            }
        }
        Err(e) => println!("  Response error after interrupt: {}", e),
    }
    
    // Test 4: Normal operation after interrupt
    println!("\nTest 4: Continue after interrupt");
    let messages = client.send_and_receive("What is 2 + 2?".to_string()).await?;
    println!("✓ Got {} messages", messages.len());
    for msg in &messages {
        if let claude_code_sdk::Message::Assistant { message } = msg {
            for content in &message.content {
                if let ContentBlock::Text(text) = content {
                    println!("  Claude says: {}", text.text);
                }
            }
        }
    }
    
    // Disconnect
    println!("\nTest 5: Disconnect");
    client.disconnect().await?;
    println!("✓ Disconnected");
    
    println!("\n=== All new features tested! ===");
    Ok(())
}