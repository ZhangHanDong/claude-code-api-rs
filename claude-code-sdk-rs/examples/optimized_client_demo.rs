//! Example demonstrating the optimized client with various performance features

use nexus_claude::{
    ClaudeCodeOptions, ClientMode, OptimizedClient, PermissionMode, Result, RetryConfig,
};
use std::time::{Duration, Instant};
use tracing::{Level, info};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    // Configure options
    let options = ClaudeCodeOptions::builder()
        .permission_mode(PermissionMode::AcceptEdits)
        .model("claude-3.5-sonnet")
        .build();

    info!("=== Demonstrating One-Shot Query Mode ===");
    demo_oneshot_mode(options.clone()).await?;

    info!("\n=== Demonstrating Interactive Mode ===");
    demo_interactive_mode(options.clone()).await?;

    info!("\n=== Demonstrating Batch Processing Mode ===");
    demo_batch_mode(options.clone()).await?;

    info!("\n=== Demonstrating Retry Logic ===");
    demo_retry_logic(options.clone()).await?;

    Ok(())
}

/// Demonstrate one-shot query mode with connection pooling
async fn demo_oneshot_mode(options: ClaudeCodeOptions) -> Result<()> {
    let client = OptimizedClient::new(options, ClientMode::OneShot)?;

    // Multiple queries reuse connections from the pool
    let queries = vec![
        "What is 2 + 2?",
        "What is the capital of France?",
        "Explain async/await in Rust in one sentence.",
    ];

    for query in queries {
        let start = Instant::now();
        let messages = client.query(query.to_string()).await?;
        let elapsed = start.elapsed();

        info!("Query: {}", query);
        info!("Response time: {:?}", elapsed);

        for msg in messages {
            if let nexus_claude::Message::Assistant {
                message: assistant_msg,
            } = msg
            {
                info!("Answer: {:?}", assistant_msg.content);
            }
        }
        info!("---");
    }

    Ok(())
}

/// Demonstrate interactive mode with optimized message handling
async fn demo_interactive_mode(options: ClaudeCodeOptions) -> Result<()> {
    let client = OptimizedClient::new(options, ClientMode::Interactive)?;

    // Start interactive session
    client.start_interactive_session().await?;

    // Send multiple messages in a conversation
    let prompts = vec![
        "Let's solve a math problem step by step. What's 15 * 23?",
        "Now add 47 to that result.",
        "Finally, divide by 13 and round to 2 decimal places.",
    ];

    for prompt in prompts {
        info!("Sending: {}", prompt);

        let start = Instant::now();
        client.send_interactive(prompt.to_string()).await?;
        let messages = client.receive_interactive().await?;
        let elapsed = start.elapsed();

        info!("Response time: {:?}", elapsed);

        for msg in messages {
            if let nexus_claude::Message::Assistant {
                message: assistant_msg,
            } = msg
            {
                info!("Response: {:?}", assistant_msg.content);
            }
        }
        info!("---");
    }

    // End session
    client.end_interactive_session().await?;

    Ok(())
}

/// Demonstrate batch processing mode with concurrent execution
async fn demo_batch_mode(options: ClaudeCodeOptions) -> Result<()> {
    let client = OptimizedClient::new(options, ClientMode::Batch { max_concurrent: 3 })?;

    // Prepare batch of queries
    let prompts = vec![
        "Write a haiku about Rust programming".to_string(),
        "What are the benefits of async programming?".to_string(),
        "Explain memory safety in one paragraph".to_string(),
        "List 3 popular Rust web frameworks".to_string(),
        "What is a Result type in Rust?".to_string(),
    ];

    info!(
        "Processing {} queries with max concurrency of 3",
        prompts.len()
    );
    let start = Instant::now();

    let results = client.process_batch(prompts).await?;
    let elapsed = start.elapsed();

    info!("Total batch processing time: {:?}", elapsed);
    info!(
        "Average time per query: {:?}",
        elapsed / results.len() as u32
    );

    // Process results
    for (i, result) in results.iter().enumerate() {
        match result {
            Ok(messages) => {
                info!("Query {} completed successfully", i + 1);
                for msg in messages {
                    if let nexus_claude::Message::Assistant {
                        message: assistant_msg,
                    } = msg
                    {
                        info!(
                            "  Response preview: {:?}",
                            format!("{:?}", assistant_msg.content)
                                .chars()
                                .take(50)
                                .collect::<String>()
                        );
                    }
                }
            },
            Err(e) => {
                info!("Query {} failed: {}", i + 1, e);
            },
        }
    }

    Ok(())
}

/// Demonstrate retry logic with exponential backoff
async fn demo_retry_logic(options: ClaudeCodeOptions) -> Result<()> {
    let client = OptimizedClient::new(options, ClientMode::OneShot)?;

    // Custom retry configuration
    let retry_config = RetryConfig {
        max_retries: 5,
        initial_delay: Duration::from_millis(500),
        max_delay: Duration::from_secs(10),
        backoff_multiplier: 2.0,
        jitter_factor: 0.1,
    };

    info!("Testing retry logic with custom configuration");

    // This query should succeed (demonstrating retry is transparent when not needed)
    let start = Instant::now();
    let result = client
        .query_with_retry(
            "What is the meaning of life?".to_string(),
            retry_config.max_retries,
            retry_config.initial_delay,
        )
        .await?;
    let elapsed = start.elapsed();

    info!("Query completed in {:?}", elapsed);

    for msg in result {
        if let nexus_claude::Message::Assistant {
            message: assistant_msg,
        } = msg
        {
            info!("Response: {:?}", assistant_msg.content);
        }
    }

    Ok(())
}
