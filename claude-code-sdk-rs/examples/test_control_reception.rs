use futures::StreamExt;
use nexus_claude::{ClaudeCodeOptions, ClaudeSDKClient, Result};
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() -> Result<()> {
    // Create a simple permission callback
    let permission_callback = Arc::new(TestPermissionCallback {
        log: Arc::new(Mutex::new(Vec::new())),
    });

    let options = ClaudeCodeOptions {
        can_use_tool: Some(permission_callback.clone()),
        ..Default::default()
    };

    let mut client = ClaudeSDKClient::new(options);

    println!("Testing control protocol reception...");

    // Connect to CLI
    client
        .connect(Some("Test control protocol".to_string()))
        .await?;

    // Send a test query that might trigger tool use
    client
        .send_user_message("Please use a tool to test permissions".to_string())
        .await?;

    // Receive messages
    let mut messages = client.receive_messages().await;
    let mut message_count = 0;

    while let Some(msg) = messages.next().await {
        message_count += 1;
        match msg {
            Ok(msg) => {
                println!("Received message: {msg:?}");
                if matches!(msg, nexus_claude::Message::Result { .. }) {
                    break;
                }
            },
            Err(e) => {
                eprintln!("Error: {e}");
                break;
            },
        }

        if message_count > 10 {
            println!("Stopping after 10 messages");
            break;
        }
    }

    // Check if permission callback was triggered
    let log = permission_callback.log.lock().await;
    if !log.is_empty() {
        println!(
            "\n✅ Permission callback was triggered {} times!",
            log.len()
        );
        for entry in log.iter() {
            println!("  - {entry}");
        }
    } else {
        println!("\n⚠️  Permission callback was not triggered");
    }

    client.disconnect().await?;
    Ok(())
}

struct TestPermissionCallback {
    log: Arc<Mutex<Vec<String>>>,
}

#[async_trait::async_trait]
impl nexus_claude::CanUseTool for TestPermissionCallback {
    async fn can_use_tool(
        &self,
        tool_name: &str,
        _input: &serde_json::Value,
        _context: &nexus_claude::ToolPermissionContext,
    ) -> nexus_claude::PermissionResult {
        let mut log = self.log.lock().await;
        log.push(format!("Permission check for tool: {tool_name}"));
        println!("🔐 Permission callback triggered for tool: {}", tool_name);

        // Always allow for testing
        nexus_claude::PermissionResult::Allow(nexus_claude::PermissionResultAllow {
            updated_input: None,
            updated_permissions: None,
        })
    }
}
