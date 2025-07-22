//! Interactive demo showing stateful conversation

use cc_sdk::{SimpleInteractiveClient, ClaudeCodeOptions, Result, ContentBlock};

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Claude Interactive Demo ===");
    println!("This demonstrates stateful conversations where Claude remembers context\n");

    // Create and connect client
    let mut client = SimpleInteractiveClient::new(ClaudeCodeOptions::default())?;
    client.connect().await?;

    // Conversation 1: Establish context
    println!("User: My name is Alice and I like programming in Rust.");
    let messages = client.send_and_receive(
        "My name is Alice and I like programming in Rust.".to_string()
    ).await?;

    print!("Claude: ");
    for msg in &messages {
        if let cc_sdk::Message::Assistant { message } = msg {
            for content in &message.content {
                if let ContentBlock::Text(text) = content {
                    print!("{}", text.text);
                }
            }
        }
    }
    println!("\n");

    // Conversation 2: Test memory
    println!("User: What's my name and what language do I like?");
    let messages = client.send_and_receive(
        "What's my name and what language do I like?".to_string()
    ).await?;

    print!("Claude: ");
    for msg in &messages {
        if let cc_sdk::Message::Assistant { message } = msg {
            for content in &message.content {
                if let ContentBlock::Text(text) = content {
                    print!("{}", text.text);
                }
            }
        }
    }
    println!("\n");

    // Conversation 3: Continue context
    println!("User: Can you give me a simple Rust tip?");
    let messages = client.send_and_receive(
        "Can you give me a simple Rust tip?".to_string()
    ).await?;

    print!("Claude: ");
    for msg in &messages {
        if let cc_sdk::Message::Assistant { message } = msg {
            for content in &message.content {
                if let ContentBlock::Text(text) = content {
                    print!("{}", text.text);
                }
            }
        }
    }
    println!("\n");

    // Disconnect
    client.disconnect().await?;
    println!("=== Demo Complete - Claude remembered the conversation context! ===");

    Ok(())
}
