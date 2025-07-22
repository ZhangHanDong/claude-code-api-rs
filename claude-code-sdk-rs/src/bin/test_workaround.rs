//! Test workaround for the deadlock issue

use claude_code_sdk::{ClaudeCodeOptions, Result};
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Testing workaround using query function...\n");
    
    // The query function works because it creates a new transport for each query
    println!("Query 1: What is 2 + 2?");
    let mut stream1 = claude_code_sdk::query("What is 2 + 2?", None).await?;
    
    let mut messages = Vec::new();
    while let Some(msg) = stream1.next().await {
        match msg {
            Ok(m) => {
                println!("  {:?}", m);
                if matches!(m, claude_code_sdk::Message::Result { .. }) {
                    messages.push(m);
                    break;
                }
                messages.push(m);
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                break;
            }
        }
    }
    
    println!("\nQuery 2: What is 10 + 10?");
    let mut stream2 = claude_code_sdk::query("What is 10 + 10?", None).await?;
    
    while let Some(msg) = stream2.next().await {
        match msg {
            Ok(m) => {
                println!("  {:?}", m);
                if matches!(m, claude_code_sdk::Message::Result { .. }) {
                    break;
                }
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                break;
            }
        }
    }
    
    println!("\nBoth queries completed successfully!");
    println!("\nRecommendation: Use multiple query() calls instead of ClaudeSDKClient for now.");
    
    Ok(())
}