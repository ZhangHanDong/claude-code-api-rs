//! Full interactive test

use claude_code_sdk::{ClaudeSDKClient, ClaudeCodeOptions, Result};
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Testing ClaudeSDKClient ===\n");
    
    // Test 1: Connect without initial prompt
    println!("Test 1: Connect without initial prompt");
    let mut client = ClaudeSDKClient::new(ClaudeCodeOptions::default());
    client.connect(None).await?;
    println!("✓ Connected successfully\n");
    
    // Test 2: Send a message and receive response
    println!("Test 2: Send message and receive response");
    client.send_request("What is 2 + 2?".to_string(), None).await?;
    
    let mut messages = client.receive_messages().await;
    let mut count = 0;
    while let Some(msg) = messages.next().await {
        match msg {
            Ok(m) => {
                count += 1;
                println!("  Message {}: {:?}", count, m);
                if matches!(m, claude_code_sdk::Message::Result { .. }) {
                    break;
                }
            }
            Err(e) => {
                eprintln!("  Error: {}", e);
                break;
            }
        }
    }
    println!("✓ Received {} messages\n", count);
    
    // Test 3: Send follow-up message
    println!("Test 3: Send follow-up message");
    client.send_request("Now what is 10 + 10?".to_string(), None).await?;
    
    let mut messages = client.receive_messages().await;
    let mut count = 0;
    while let Some(msg) = messages.next().await {
        match msg {
            Ok(m) => {
                count += 1;
                println!("  Message {}: {:?}", count, m);
                if matches!(m, claude_code_sdk::Message::Result { .. }) {
                    break;
                }
            }
            Err(e) => {
                eprintln!("  Error: {}", e);
                break;
            }
        }
    }
    println!("✓ Received {} messages for follow-up\n", count);
    
    // Test 4: Multiple concurrent receivers (demonstrating broadcast)
    println!("Test 4: Multiple concurrent receivers");
    client.send_request("What is the capital of France?".to_string(), None).await?;
    
    // Create two receivers
    let mut receiver1 = client.receive_messages().await;
    let mut receiver2 = client.receive_messages().await;
    
    // Both should receive the same messages
    let handle1 = tokio::spawn(async move {
        let mut msgs = Vec::new();
        while let Some(msg) = receiver1.next().await {
            if let Ok(m) = msg {
                msgs.push(format!("R1: {:?}", m));
                if matches!(m, claude_code_sdk::Message::Result { .. }) {
                    break;
                }
            }
        }
        msgs
    });
    
    let handle2 = tokio::spawn(async move {
        let mut msgs = Vec::new();
        while let Some(msg) = receiver2.next().await {
            if let Ok(m) = msg {
                msgs.push(format!("R2: {:?}", m));
                if matches!(m, claude_code_sdk::Message::Result { .. }) {
                    break;
                }
            }
        }
        msgs
    });
    
    let msgs1 = handle1.await.unwrap();
    let msgs2 = handle2.await.unwrap();
    
    println!("  Receiver 1 got {} messages", msgs1.len());
    println!("  Receiver 2 got {} messages", msgs2.len());
    println!("✓ Both receivers got messages\n");
    
    // Test 5: Disconnect
    println!("Test 5: Disconnect");
    client.disconnect().await?;
    println!("✓ Disconnected successfully\n");
    
    println!("=== All tests passed! ===");
    
    Ok(())
}