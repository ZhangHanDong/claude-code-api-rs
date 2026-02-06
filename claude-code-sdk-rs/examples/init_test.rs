use futures::StreamExt;
use nexus_claude::{ClaudeCodeOptions, ClaudeSDKClient, Result};

#[tokio::main]
async fn main() -> Result<()> {
    // env_logger::init(); // Skip if not available

    println!("=== Initialization Test ===\n");

    let options = ClaudeCodeOptions::default();
    let mut client = ClaudeSDKClient::new(options);

    println!("1. Connecting to Claude CLI...");
    client
        .connect(Some("Test initialization".to_string()))
        .await?;
    println!("   ✅ Connected successfully");

    println!("\n2. Checking server info...");
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    if let Some(server_info) = client.get_server_info().await {
        println!("   ✅ Server info received:");
        println!("   {}", serde_json::to_string_pretty(&server_info)?);
    } else {
        println!("   ⚠️ No server info available");
    }

    println!("\n3. Sending test message...");
    client.send_user_message("What is 2+2?".to_string()).await?;

    let mut messages = client.receive_messages().await;
    let mut msg_count = 0;

    while let Some(msg_result) = messages.next().await {
        msg_count += 1;
        match msg_result {
            Ok(msg) => {
                println!("   Message {msg_count}: {msg:?}");
                if matches!(msg, nexus_claude::Message::Result { .. }) {
                    println!("   ✅ Received result message");
                    break;
                }
            },
            Err(e) => {
                println!("   ❌ Error: {e}");
                break;
            },
        }

        if msg_count > 10 {
            println!("   ⚠️ Stopping after 10 messages");
            break;
        }
    }

    println!("\n4. Disconnecting...");
    client.disconnect().await?;
    println!("   ✅ Disconnected successfully");

    println!("\n=== Test Complete ===");
    Ok(())
}
