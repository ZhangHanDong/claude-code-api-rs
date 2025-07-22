//! Real interactive demo - you can type questions and get responses

use claude_code_sdk::{SimpleInteractiveClient, ClaudeCodeOptions, Result, ContentBlock};
use std::io::{self, Write};

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Claude Interactive Demo ===");
    println!("Type 'quit' to exit\n");
    
    // Create and connect client
    let mut client = SimpleInteractiveClient::new(ClaudeCodeOptions::default())?;
    println!("Connecting to Claude...");
    client.connect().await?;
    println!("Connected! Start chatting:\n");
    
    // Interactive loop
    let stdin = io::stdin();
    let mut input = String::new();
    
    loop {
        print!("You: ");
        io::stdout().flush().unwrap();
        
        input.clear();
        stdin.read_line(&mut input).unwrap();
        let input_text = input.trim();
        
        if input_text.is_empty() {
            continue;
        }
        
        if input_text == "quit" {
            break;
        }
        
        // Send and receive
        println!("Claude is thinking...");
        match client.send_and_receive(input_text.to_string()).await {
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