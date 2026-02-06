//! Manual interactive mode test - type your own messages

use nexus_claude::{
    ClaudeCodeOptions, ClientMode, ContentBlock, InteractiveClient, Message, OptimizedClient,
    PermissionMode, Result,
};
use std::io::{self, Write};
use tracing::Level;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    println!("=== Claude Code Interactive Mode Test ===");
    println!("Choose client type:");
    println!("1. Traditional InteractiveClient");
    println!("2. OptimizedClient with Interactive Mode");
    print!("Your choice (1 or 2): ");
    io::stdout().flush().unwrap();

    let mut choice = String::new();
    io::stdin().read_line(&mut choice).unwrap();

    let options = ClaudeCodeOptions::builder()
        .permission_mode(PermissionMode::AcceptEdits)
        .build();

    match choice.trim() {
        "1" => test_traditional_interactive(options).await?,
        "2" => test_optimized_interactive(options).await?,
        _ => {
            println!("Invalid choice. Using traditional client.");
            test_traditional_interactive(options).await?;
        },
    }

    Ok(())
}

async fn test_traditional_interactive(options: ClaudeCodeOptions) -> Result<()> {
    println!("\n=== Using Traditional InteractiveClient ===");
    println!("This maintains conversation context across messages.");
    println!("Type 'exit' to quit.\n");

    let mut client = InteractiveClient::new(options)?;

    println!("Connecting to Claude Code...");
    client.connect().await?;
    println!("Connected! Start chatting:\n");

    loop {
        print!("You: ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let input = input.trim();

        if input == "exit" || input == "quit" {
            break;
        }

        if input.is_empty() {
            continue;
        }

        // Send and receive
        match client.send_and_receive(input.to_string()).await {
            Ok(messages) => {
                print!("Claude: ");
                let mut first = true;
                for msg in messages {
                    if let Message::Assistant { message } = msg {
                        for content in message.content {
                            if let ContentBlock::Text(text) = content {
                                if !first {
                                    print!("\n        ");
                                }
                                println!("{}", text.text);
                                first = false;
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

    println!("\nDisconnecting...");
    client.disconnect().await?;
    println!("Goodbye!");

    Ok(())
}

async fn test_optimized_interactive(options: ClaudeCodeOptions) -> Result<()> {
    println!("\n=== Using OptimizedClient in Interactive Mode ===");
    println!("This uses connection pooling for better performance.");
    println!("Type 'exit' to quit.\n");

    let client = OptimizedClient::new(options, ClientMode::Interactive)?;

    println!("Starting interactive session...");
    client.start_interactive_session().await?;
    println!("Session started! Start chatting:\n");

    loop {
        print!("You: ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let input = input.trim();

        if input == "exit" || input == "quit" {
            break;
        }

        if input.is_empty() {
            continue;
        }

        // Special commands
        if input.starts_with("/") {
            handle_command(&client, input).await;
            continue;
        }

        // Send message
        match client.send_interactive(input.to_string()).await {
            Ok(_) => {
                // Receive response
                match client.receive_interactive().await {
                    Ok(messages) => {
                        print!("Claude: ");
                        let mut first = true;
                        for msg in messages {
                            if let Message::Assistant { message } = msg {
                                for content in message.content {
                                    if let ContentBlock::Text(text) = content {
                                        if !first {
                                            print!("\n        ");
                                        }
                                        println!("{}", text.text);
                                        first = false;
                                    }
                                }
                            }
                        }
                        println!();
                    },
                    Err(e) => {
                        println!("Error receiving: {e}");
                    },
                }
            },
            Err(e) => {
                println!("Error sending: {e}");
            },
        }
    }

    println!("\nEnding session...");
    client.end_interactive_session().await?;
    println!("Session ended. Goodbye!");

    Ok(())
}

async fn handle_command(client: &OptimizedClient, command: &str) {
    match command {
        "/help" => {
            println!("\nAvailable commands:");
            println!("/help    - Show this help");
            println!("/clear   - Clear conversation (restart session)");
            println!("/status  - Show session status");
            println!("/exit    - Exit the program");
            println!();
        },
        "/clear" => {
            println!("Clearing conversation...");
            if let Err(e) = client.end_interactive_session().await {
                println!("Error ending session: {e}");
            }
            if let Err(e) = client.start_interactive_session().await {
                println!("Error starting new session: {e}");
            } else {
                println!("New session started!");
            }
        },
        "/status" => {
            println!("Session is active");
            println!("Using OptimizedClient with connection pooling");
        },
        "/exit" => {
            std::process::exit(0);
        },
        _ => {
            println!("Unknown command. Type /help for available commands.");
        },
    }
}
