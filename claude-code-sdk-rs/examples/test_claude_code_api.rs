//! Test Claude Code as API service

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

    info!("=== Testing Claude Code API Service ===\n");
    info!("This uses claude-code CLI, no API key needed\n");

    // Configure options
    let options = ClaudeCodeOptions::builder()
        .permission_mode(PermissionMode::AcceptEdits)
        .build();

    // Test 1: Simple query
    info!("Test 1: Simple Query");
    test_simple_query(options.clone()).await?;

    // Test 2: Interactive conversation
    info!("\nTest 2: Interactive Conversation");
    test_interactive(options.clone()).await?;

    // Test 3: Batch processing
    info!("\nTest 3: Batch Processing");
    test_batch(options.clone()).await?;

    Ok(())
}

async fn test_simple_query(options: ClaudeCodeOptions) -> Result<()> {
    let client = OptimizedClient::new(options, ClientMode::OneShot)?;

    let queries = vec![
        "What is 2 + 2?",
        "Write a Python hello world",
        "Explain what Git is in one sentence",
    ];

    for query in queries {
        info!("Query: {}", query);
        let start = Instant::now();

        match client.query(query.to_string()).await {
            Ok(messages) => {
                for msg in messages {
                    if let Message::Assistant { message } = msg {
                        for content in message.content {
                            if let ContentBlock::Text(text) = content {
                                info!("Response: {}", text.text);
                            }
                        }
                    }
                }
                info!("Time: {:?}", start.elapsed());
            },
            Err(e) => info!("Error: {}", e),
        }
        info!("---");
    }

    Ok(())
}

async fn test_interactive(options: ClaudeCodeOptions) -> Result<()> {
    let mut client = InteractiveClient::new(options)?;

    client.connect().await?;
    info!("Connected to claude-code");

    let conversation = vec![
        "Hello!",
        "Can you help me write a function?",
        "I need a function to reverse a string in Rust",
    ];

    for prompt in conversation {
        info!("You: {}", prompt);

        match client.send_and_receive(prompt.to_string()).await {
            Ok(messages) => {
                for msg in messages {
                    if let Message::Assistant { message } = msg {
                        for content in message.content {
                            if let ContentBlock::Text(text) = content {
                                info!("Claude: {}", text.text);
                            }
                        }
                    }
                }
            },
            Err(e) => info!("Error: {}", e),
        }
        info!("---");
    }

    client.disconnect().await?;
    info!("Disconnected");

    Ok(())
}

async fn test_batch(options: ClaudeCodeOptions) -> Result<()> {
    let client = OptimizedClient::new(options, ClientMode::Batch { max_concurrent: 3 })?;

    let tasks = vec![
        "What is the capital of Japan?".to_string(),
        "Convert 100 Fahrenheit to Celsius".to_string(),
        "List 3 popular programming languages".to_string(),
        "What is 15 * 15?".to_string(),
        "Explain REST API in one sentence".to_string(),
    ];

    info!("Processing {} tasks with max concurrency 3", tasks.len());
    let start = Instant::now();

    match client.process_batch(tasks.clone()).await {
        Ok(results) => {
            let successful = results.iter().filter(|r| r.is_ok()).count();
            info!("Completed: {}/{} successful", successful, results.len());
            info!("Total time: {:?}", start.elapsed());

            // Show first 2 results
            for (i, result) in results.iter().take(2).enumerate() {
                if let Ok(messages) = result {
                    info!("\nTask {}: {}", i + 1, tasks[i]);
                    for msg in messages {
                        if let Message::Assistant { message } = msg {
                            for content in &message.content {
                                if let ContentBlock::Text(text) = content {
                                    info!("Result: {}", text.text);
                                }
                            }
                        }
                    }
                }
            }
        },
        Err(e) => info!("Batch processing error: {}", e),
    }

    Ok(())
}
