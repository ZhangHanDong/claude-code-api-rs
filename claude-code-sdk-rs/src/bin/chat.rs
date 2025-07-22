//! Interactive chat with Claude - you can type your own questions!

use claude_code_sdk::{SimpleInteractiveClient, ClaudeCodeOptions, Result, ContentBlock};
use std::io::{self, Write};

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Claude Chat ===");
    println!("Type your messages below. Type 'quit' to exit.\n");
    
    // Create and connect client
    let mut client = SimpleInteractiveClient::new(ClaudeCodeOptions::default())?;
    println!("Connecting to Claude...");
    client.connect().await?;
    println!("Connected! You can start chatting now.\n");
    
    // Read input line by line
    loop {
        print!("You: ");
        io::stdout().flush().unwrap();
        
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let input = input.trim();
        
        if input.is_empty() {
            continue;
        }
        
        if input == "quit" {
            break;
        }
        
        // Send message and get response
        match client.send_and_receive(input.to_string()).await {
            Ok(messages) => {
                print!("Claude: ");
                for msg in &messages {
                    if let claude_code_sdk::Message::Assistant { message } = msg {
                        for content in &message.content {
                            if let ContentBlock::Text(text) = content {
                                print!("{}", text.text);
                            }
                        }
                    }
                }
                println!("\n");
            }
            Err(e) => {
                println!("Error: {}\n", e);
            }
        }
    }
    
    // Disconnect
    println!("\nDisconnecting...");
    client.disconnect().await?;
    println!("Goodbye!");
    
    Ok(())
}