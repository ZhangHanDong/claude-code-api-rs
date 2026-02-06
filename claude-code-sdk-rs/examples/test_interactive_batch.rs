//! Test interactive mode and batch requests

use nexus_claude::{
    ClaudeCodeOptions, ClientMode, ContentBlock, InteractiveClient, Message, OptimizedClient,
    PermissionMode, Result,
};
use std::time::Instant;
use tracing::{Level, info};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    info!("=== Testing Interactive Mode and Batch Requests ===\n");

    let options = ClaudeCodeOptions::builder()
        .permission_mode(PermissionMode::AcceptEdits)
        .build();

    // Test 1: Interactive Mode
    test_interactive_mode(options.clone()).await?;

    // Test 2: Batch Processing
    test_batch_mode(options.clone()).await?;

    // Test 3: Mixed Workload
    test_mixed_workload(options.clone()).await?;

    Ok(())
}

/// Test 1: Interactive Mode (maintaining conversation context)
async fn test_interactive_mode(options: ClaudeCodeOptions) -> Result<()> {
    info!("=== Test 1: Interactive Mode ===");
    info!("This mode maintains conversation context across multiple messages\n");

    // Method 1: Using InteractiveClient
    info!("Method 1: Using InteractiveClient");
    let mut client = InteractiveClient::new(options.clone())?;
    client.connect().await?;

    // Have a multi-turn conversation
    let conversation = vec![
        ("User", "My name is Alice and I'm learning Rust"),
        ("User", "What's my name?"),
        ("User", "What am I learning?"),
    ];

    for (role, message) in conversation {
        info!("{}: {}", role, message);

        match client.send_and_receive(message.to_string()).await {
            Ok(messages) => {
                for msg in messages {
                    if let Message::Assistant {
                        message: assistant_msg,
                    } = msg
                    {
                        for content in assistant_msg.content {
                            if let ContentBlock::Text(text) = content {
                                info!("Assistant: {}", text.text);
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

    // Method 2: Using OptimizedClient in Interactive mode
    info!("\nMethod 2: Using OptimizedClient in Interactive Mode");
    let client = OptimizedClient::new(options, ClientMode::Interactive)?;

    client.start_interactive_session().await?;
    info!("Interactive session started\n");

    // Another conversation
    let prompts = vec![
        "I have a list: [1, 2, 3, 4, 5]",
        "What's the sum of all numbers in my list?",
        "What's the average of the numbers?",
    ];

    for prompt in prompts {
        info!("User: {}", prompt);
        client.send_interactive(prompt.to_string()).await?;

        let messages = client.receive_interactive().await?;
        for msg in messages {
            if let Message::Assistant {
                message: assistant_msg,
            } = msg
            {
                for content in assistant_msg.content {
                    if let ContentBlock::Text(text) = content {
                        info!("Assistant: {}", text.text);
                    }
                }
            }
        }
        info!("---");
    }

    client.end_interactive_session().await?;
    info!("Interactive session ended\n");

    Ok(())
}

/// Test 2: Batch Processing
async fn test_batch_mode(options: ClaudeCodeOptions) -> Result<()> {
    info!("=== Test 2: Batch Processing ===");
    info!("Process multiple independent queries concurrently\n");

    // Create batch client with specific concurrency
    let client = OptimizedClient::new(options, ClientMode::Batch { max_concurrent: 3 })?;

    // Prepare diverse queries
    let queries = vec![
        "What is the capital of Japan?".to_string(),
        "Write a haiku about programming".to_string(),
        "Convert 100 Fahrenheit to Celsius".to_string(),
        "List 3 benefits of exercise".to_string(),
        "What is 15% of 200?".to_string(),
        "Explain REST API in one sentence".to_string(),
    ];

    info!("Processing {} queries with max_concurrent=3", queries.len());
    let start = Instant::now();

    match client.process_batch(queries.clone()).await {
        Ok(results) => {
            let elapsed = start.elapsed();
            let successful = results.iter().filter(|r| r.is_ok()).count();

            info!("\nBatch Results:");
            info!("- Total queries: {}", queries.len());
            info!("- Successful: {}", successful);
            info!("- Failed: {}", queries.len() - successful);
            info!("- Total time: {:?}", elapsed);
            info!(
                "- Average time per query: {:?}",
                elapsed / queries.len() as u32
            );
            info!(
                "- Queries per second: {:.2}\n",
                queries.len() as f64 / elapsed.as_secs_f64()
            );

            // Show first 3 results
            for (i, (query, result)) in queries.iter().zip(results.iter()).take(3).enumerate() {
                info!("Query {}: {}", i + 1, query);
                match result {
                    Ok(messages) => {
                        for msg in messages {
                            if let Message::Assistant {
                                message: assistant_msg,
                            } = msg
                            {
                                for content in &assistant_msg.content {
                                    if let ContentBlock::Text(text) = content {
                                        let preview = if text.text.len() > 100 {
                                            format!("{}...", &text.text[..100])
                                        } else {
                                            text.text.clone()
                                        };
                                        info!("Response: {}", preview);
                                    }
                                }
                            }
                        }
                    },
                    Err(e) => info!("Error: {}", e),
                }
                info!("---");
            }

            if queries.len() > 3 {
                info!("... and {} more queries processed", queries.len() - 3);
            }
        },
        Err(e) => info!("Batch processing error: {}", e),
    }

    Ok(())
}

/// Test 3: Mixed Workload (combine interactive and batch)
async fn test_mixed_workload(options: ClaudeCodeOptions) -> Result<()> {
    info!("\n=== Test 3: Mixed Workload ===");
    info!("Demonstrate using both modes for different tasks\n");

    // Use batch mode for independent queries
    info!("Step 1: Gather information using batch mode");
    let batch_client =
        OptimizedClient::new(options.clone(), ClientMode::Batch { max_concurrent: 2 })?;

    let info_queries = vec![
        "What is Rust programming language?".to_string(),
        "What are the main features of Rust?".to_string(),
        "What is memory safety?".to_string(),
    ];

    let results = batch_client.process_batch(info_queries).await?;
    info!("Gathered {} pieces of information", results.len());

    // Use interactive mode to discuss the gathered information
    info!("\nStep 2: Discuss findings in interactive mode");
    let interactive_client = OptimizedClient::new(options, ClientMode::Interactive)?;

    interactive_client.start_interactive_session().await?;

    let discussion = vec![
        "I just learned about Rust. Can you help me understand it better?",
        "What makes Rust different from C++?",
        "Should I learn Rust if I already know Python?",
    ];

    for prompt in discussion {
        info!("User: {}", prompt);
        interactive_client
            .send_interactive(prompt.to_string())
            .await?;

        let messages = interactive_client.receive_interactive().await?;
        for msg in messages {
            if let Message::Assistant {
                message: assistant_msg,
            } = msg
            {
                for content in assistant_msg.content {
                    if let ContentBlock::Text(text) = content {
                        let preview = if text.text.len() > 150 {
                            format!("{}...", &text.text[..150])
                        } else {
                            text.text.clone()
                        };
                        info!("Assistant: {}", preview);
                    }
                }
            }
        }
        info!("---");
    }

    interactive_client.end_interactive_session().await?;

    info!("\nMixed workload complete!");
    info!("- Used batch mode for parallel information gathering");
    info!("- Used interactive mode for contextual discussion");

    Ok(())
}
