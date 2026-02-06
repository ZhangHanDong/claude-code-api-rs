//! Demonstration of control protocol format configuration
//!
//! This example shows how to configure the SDK to use different control protocol
//! formats for compatibility with various CLI versions.

use futures::StreamExt;
use nexus_claude::{ClaudeCodeOptions, ClaudeSDKClient, ControlProtocolFormat, Result};

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Control Protocol Format Demo ===\n");

    // Example 1: Use legacy format (default, maximum compatibility)
    println!("1. Using Legacy format (default):");
    let options = ClaudeCodeOptions::builder()
        .system_prompt("You are a helpful assistant")
        .control_protocol_format(ControlProtocolFormat::Legacy)
        .build();

    println!("   Format: Legacy (sdk_control_request/sdk_control_response)");
    println!("   Compatible with: All CLI versions\n");

    // Example 2: Use new format (for newer CLIs)
    println!("2. Using Control format:");
    let _options_new = ClaudeCodeOptions::builder()
        .system_prompt("You are a helpful assistant")
        .control_protocol_format(ControlProtocolFormat::Control)
        .build();

    println!("   Format: Control (type=control)");
    println!("   Compatible with: Newer CLI versions only\n");

    // Example 3: Auto-detect (future feature)
    println!("3. Using Auto format:");
    let _options_auto = ClaudeCodeOptions::builder()
        .system_prompt("You are a helpful assistant")
        .control_protocol_format(ControlProtocolFormat::Auto)
        .build();

    println!("   Format: Auto (defaults to Legacy for now)");
    println!("   Future: Will detect CLI capabilities\n");

    // Example 4: Environment variable override
    println!("4. Environment variable override:");
    println!("   Set CLAUDE_CODE_CONTROL_FORMAT=legacy or control");
    println!("   This overrides the programmatic setting\n");

    // Demonstrate with actual client
    println!("5. Testing with actual client (Legacy format):");
    let mut client = ClaudeSDKClient::new(options);

    match client.connect(Some("What is 2 + 2?".to_string())).await {
        Ok(_) => {
            println!("   ✓ Connected successfully");

            // Receive response
            let mut messages = client.receive_messages().await;
            let mut response_count = 0;

            while let Some(msg) = messages.next().await {
                match msg {
                    Ok(msg) => {
                        response_count += 1;
                        if response_count <= 3 {
                            println!("   ✓ Received message: {msg:?}");
                        }

                        // Stop after Result message
                        if matches!(msg, nexus_claude::Message::Result { .. }) {
                            break;
                        }
                    },
                    Err(e) => {
                        println!("   ✗ Error receiving message: {e}");
                        break;
                    },
                }
            }

            client.disconnect().await?;
            println!("   ✓ Disconnected successfully");
        },
        Err(e) => {
            println!("   ✗ Failed to connect: {e}");
            println!("   Note: This might be expected if Claude CLI is not installed");
        },
    }

    println!("\n=== Configuration Summary ===");
    println!("• Default: Legacy format for maximum compatibility");
    println!("• Can be changed via ClaudeCodeOptions::control_protocol_format");
    println!("• Can be overridden via CLAUDE_CODE_CONTROL_FORMAT env var");
    println!("• Receiving: Always supports both formats (dual-stack)");
    println!("• Sending: Configurable based on your CLI version");

    Ok(())
}
