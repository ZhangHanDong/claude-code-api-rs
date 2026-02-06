//! Interactive real API test

use nexus_claude::{
    ClaudeCodeOptions, ContentBlock, InteractiveClient, Message, PermissionMode, Result,
};
use std::io::{self, Write};

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Interactive Real API Test ===");
    println!("Type 'exit' to quit\n");

    // Create client with options
    let options = ClaudeCodeOptions::builder()
        .permission_mode(PermissionMode::AcceptEdits)
        .model("claude-3.5-sonnet")
        .build();

    let mut client = InteractiveClient::new(options)?;

    // Connect to Claude
    println!("Connecting to Claude...");
    client.connect().await?;
    println!("Connected! You can start chatting.\n");

    // Interactive loop
    loop {
        print!("You: ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let input = input.trim();

        if input == "exit" {
            break;
        }

        if input.is_empty() {
            continue;
        }

        // Send message and get response
        match client.send_and_receive(input.to_string()).await {
            Ok(messages) => {
                print!("Claude: ");
                for msg in messages {
                    if let Message::Assistant { message } = msg {
                        for content in message.content {
                            if let ContentBlock::Text(text) = content {
                                println!("{}", text.text);
                            }
                        }
                    }
                }
                println!();
            },
            Err(e) => {
                println!("Error: {e}");
            },
        }
    }

    // Disconnect
    println!("\nDisconnecting...");
    client.disconnect().await?;
    println!("Goodbye!");

    Ok(())
}
