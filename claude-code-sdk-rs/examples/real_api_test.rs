//! Real API test using actual Claude Code SDK

use nexus_claude::{
    ClaudeCodeOptions, ClientMode, ContentBlock, InteractiveClient, Message, OptimizedClient,
    PermissionMode, Result,
};
use std::time::Instant;
use tracing::{Level, info};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    info!("=== Real Claude Code SDK API Test ===\n");

    // Check if Claude CLI is available
    if !is_claude_cli_available() {
        eprintln!("Error: Claude CLI not found!");
        eprintln!("Please install it with: npm install -g @anthropic-ai/claude-code");
        std::process::exit(1);
    }

    // Configure options
    let options = ClaudeCodeOptions::builder()
        .permission_mode(PermissionMode::AcceptEdits)
        .model("claude-3.5-sonnet")
        .build();

    // Test 1: OneShot Query with OptimizedClient
    info!("Test 1: OneShot Query");
    test_oneshot_query(options.clone()).await?;

    // Test 2: Interactive Mode
    info!("\nTest 2: Interactive Mode");
    test_interactive_mode(options.clone()).await?;

    // Test 3: Batch Processing
    info!("\nTest 3: Batch Processing");
    test_batch_processing(options.clone()).await?;

    // Test 4: Using traditional InteractiveClient
    info!("\nTest 4: Traditional InteractiveClient");
    test_traditional_client(options.clone()).await?;

    Ok(())
}

/// Check if Claude CLI is available
fn is_claude_cli_available() -> bool {
    std::process::Command::new("claude-code")
        .arg("--version")
        .output()
        .is_ok()
}

/// Test OneShot query
async fn test_oneshot_query(options: ClaudeCodeOptions) -> Result<()> {
    let client = OptimizedClient::new(options, ClientMode::OneShot)?;

    let queries = vec![
        "What is 2 + 2?",
        "Write a haiku about Rust programming",
        "Explain async/await in one sentence",
    ];

    for query in queries {
        info!("Query: {}", query);
        let start = Instant::now();

        match client.query(query.to_string()).await {
            Ok(messages) => {
                let elapsed = start.elapsed();
                for msg in messages {
                    if let Message::Assistant { message } = msg {
                        for content in message.content {
                            if let ContentBlock::Text(text) = content {
                                info!("Response: {}", text.text);
                            }
                        }
                    }
                }
                info!("Time: {:?}", elapsed);
            },
            Err(e) => {
                info!("Error: {}", e);
            },
        }
        info!("---");
    }

    Ok(())
}

/// Test Interactive mode
async fn test_interactive_mode(options: ClaudeCodeOptions) -> Result<()> {
    let client = OptimizedClient::new(options, ClientMode::Interactive)?;

    // Start session
    client.start_interactive_session().await?;
    info!("Interactive session started");

    // Have a conversation
    let prompts = vec![
        "Hello! Can you help me with Rust?",
        "What's the difference between String and &str?",
        "Can you show me a simple example?",
    ];

    for prompt in prompts {
        info!("Sending: {}", prompt);

        client.send_interactive(prompt.to_string()).await?;
        let messages = client.receive_interactive().await?;

        for msg in messages {
            if let Message::Assistant { message } = msg {
                for content in message.content {
                    if let ContentBlock::Text(text) = content {
                        info!("Assistant: {}", text.text);
                    }
                }
            }
        }
        info!("---");
    }

    // End session
    client.end_interactive_session().await?;
    info!("Interactive session ended");

    Ok(())
}

/// Test batch processing
async fn test_batch_processing(options: ClaudeCodeOptions) -> Result<()> {
    let client = OptimizedClient::new(options, ClientMode::Batch { max_concurrent: 3 })?;

    let queries = vec![
        "What is the capital of Japan?".to_string(),
        "What is 10 * 10?".to_string(),
        "Name three programming languages".to_string(),
        "What year is it?".to_string(),
        "Explain recursion briefly".to_string(),
    ];

    info!(
        "Processing {} queries with max concurrency 3",
        queries.len()
    );
    let start = Instant::now();

    let results = client.process_batch(queries.clone()).await?;
    let elapsed = start.elapsed();

    let successful = results.iter().filter(|r| r.is_ok()).count();
    info!("Results: {}/{} successful", successful, results.len());
    info!("Total time: {:?}", elapsed);
    info!(
        "Average time per query: {:?}",
        elapsed / results.len() as u32
    );

    // Show first few results
    for (i, result) in results.iter().take(3).enumerate() {
        match result {
            Ok(messages) => {
                info!("Query {}: {}", i + 1, queries[i]);
                for msg in messages {
                    if let Message::Assistant { message } = msg {
                        for content in &message.content {
                            if let ContentBlock::Text(text) = content {
                                info!("Response: {}", text.text);
                            }
                        }
                    }
                }
            },
            Err(e) => {
                info!("Query {} failed: {}", i + 1, e);
            },
        }
        info!("---");
    }

    Ok(())
}

/// Test traditional InteractiveClient
async fn test_traditional_client(options: ClaudeCodeOptions) -> Result<()> {
    let mut client = InteractiveClient::new(options)?;

    // Connect
    client.connect().await?;
    info!("Connected with traditional client");

    // Send and receive
    let prompt = "What is Rust's main advantage?";
    info!("Query: {}", prompt);

    let start = Instant::now();
    let messages = client.send_and_receive(prompt.to_string()).await?;
    let elapsed = start.elapsed();

    for msg in messages {
        if let Message::Assistant { message } = msg {
            for content in message.content {
                if let ContentBlock::Text(text) = content {
                    info!("Response: {}", text.text);
                }
            }
        }
    }
    info!("Time: {:?}", elapsed);

    // Disconnect
    client.disconnect().await?;
    info!("Disconnected");

    Ok(())
}
