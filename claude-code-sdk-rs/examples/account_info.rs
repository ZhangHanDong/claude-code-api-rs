//! Example demonstrating how to retrieve account information
//!
//! This example shows how to use the `get_account_info()` method to retrieve
//! the current Claude account information including email, subscription type, etc.
//!
//! # Usage
//!
//! Set the ANTHROPIC_USER_EMAIL environment variable before running:
//!
//! ```bash
//! export ANTHROPIC_USER_EMAIL="your-email@example.com"
//! cargo run --example account_info
//! ```
//!
//! Or run directly with the environment variable:
//!
//! ```bash
//! ANTHROPIC_USER_EMAIL="your-email@example.com" cargo run --example account_info
//! ```

use nexus_claude::{ClaudeCodeOptions, ClaudeSDKClient, Result};

#[tokio::main]
async fn main() -> Result<()> {
    println!("╔═══════════════════════════════════════════╗");
    println!("║   Claude Code Account Information        ║");
    println!("╚═══════════════════════════════════════════╝\n");

    // Check if environment variable is set
    if let Ok(email) = std::env::var("ANTHROPIC_USER_EMAIL") {
        println!("ℹ️  Using ANTHROPIC_USER_EMAIL: {}\n", email);
    } else {
        println!("⚠️  ANTHROPIC_USER_EMAIL not set");
        println!(
            "   Run with: ANTHROPIC_USER_EMAIL=\"your@email.com\" cargo run --example account_info\n"
        );
    }

    // Create client with default options
    let options = ClaudeCodeOptions::builder()
        .max_turns(1) // Limit to 1 turn since we only need account info
        .build();

    let mut client = ClaudeSDKClient::new(options);

    println!("1. Connecting to Claude CLI...");
    client.connect(None).await?;
    println!("   ✅ Connected\n");

    println!("2. Retrieving account information...");
    match client.get_account_info().await {
        Ok(account_info) => {
            println!("   ✅ Account information retrieved:\n");
            println!("╔═══════════════════════════════════════════╗");
            println!("║ Account Details                           ║");
            println!("╠═══════════════════════════════════════════╣");

            // Print account info with proper formatting
            for line in account_info.lines() {
                println!("║ {:<42}║", line);
            }

            println!("╚═══════════════════════════════════════════╝\n");
        },
        Err(e) => {
            eprintln!("   ❌ Failed to retrieve account information: {}", e);
            eprintln!("   Note: Make sure you're logged in to Claude CLI");
        },
    }

    println!("3. Disconnecting...");
    client.disconnect().await?;
    println!("   ✅ Disconnected\n");

    Ok(())
}
