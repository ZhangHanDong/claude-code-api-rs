//! Simple API demonstration without requiring Claude CLI connection

use nexus_claude::{AssistantMessage, ClientMode, ContentBlock, Message, Result, TextContent};
use std::time::Instant;
use tracing::{Level, info};

/// Mock client for testing without Claude CLI
struct MockClient {
    mode: ClientMode,
}

impl MockClient {
    fn new(mode: ClientMode) -> Self {
        Self { mode }
    }

    /// Simulate a query response
    async fn query(&self, prompt: String) -> Result<Vec<Message>> {
        info!("Mock query: {}", prompt);

        // Simulate processing delay
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Generate mock response based on prompt
        let response = match prompt.as_str() {
            "What is 2 + 2?" => "The answer is 4.".to_string(),
            "What is the capital of France?" => "The capital of France is Paris.".to_string(),
            prompt if prompt.contains("squared") => {
                if let Some(num) = prompt.split_whitespace().nth(2) {
                    if let Ok(n) = num.parse::<i32>() {
                        format!("{} squared is {}", n, n * n)
                    } else {
                        "I couldn't parse the number.".to_string()
                    }
                } else {
                    "Please provide a number to square.".to_string()
                }
            },
            _ => format!("Mock response to: {prompt}"),
        };

        // Create mock assistant message
        let assistant_msg = Message::Assistant {
            message: AssistantMessage {
                content: vec![ContentBlock::Text(TextContent { text: response })],
            },
        };

        // Add result message
        let result_msg = Message::Result {
            subtype: "done".to_string(),
            duration_ms: 100,
            duration_api_ms: 80,
            is_error: false,
            num_turns: 1,
            session_id: "mock-session".to_string(),
            total_cost_usd: Some(0.0001),
            usage: None,
            result: Some("Success".to_string()),
            structured_output: None,
        };

        Ok(vec![assistant_msg, result_msg])
    }

    /// Simulate batch processing
    async fn process_batch(&self, prompts: Vec<String>) -> Vec<Result<Vec<Message>>> {
        let mut results = Vec::new();

        match self.mode {
            ClientMode::Batch { max_concurrent } => {
                info!(
                    "Processing batch of {} with max concurrency {}",
                    prompts.len(),
                    max_concurrent
                );

                // Simulate concurrent processing
                for (i, prompt) in prompts.into_iter().enumerate() {
                    if i > 0 && i.is_multiple_of(max_concurrent) {
                        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                    }
                    results.push(self.query(prompt).await);
                }
            },
            _ => {
                info!("Sequential processing (not in batch mode)");
                for prompt in prompts {
                    results.push(self.query(prompt).await);
                }
            },
        }

        results
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    info!("=== Claude Code SDK API Demo (Mock Mode) ===\n");

    // Demo 1: OneShot Mode
    demo_oneshot_mode().await?;

    // Demo 2: Batch Mode
    demo_batch_mode().await?;

    // Demo 3: Performance Comparison
    demo_performance_comparison().await?;

    Ok(())
}

/// Demonstrate OneShot mode
async fn demo_oneshot_mode() -> Result<()> {
    info!("--- Demo 1: OneShot Mode ---");

    let client = MockClient::new(ClientMode::OneShot);

    let queries = vec![
        "What is 2 + 2?",
        "What is the capital of France?",
        "What is 5 squared?",
    ];

    for query in queries {
        let start = Instant::now();
        let messages = client.query(query.to_string()).await?;
        let elapsed = start.elapsed();

        info!("Query: {}", query);
        for msg in messages {
            if let Message::Assistant { message } = msg {
                for content in message.content {
                    if let ContentBlock::Text(text) = content {
                        info!("Response: {}", text.text);
                    }
                }
            }
        }
        info!("Time: {:?}\n", elapsed);
    }

    Ok(())
}

/// Demonstrate Batch mode
async fn demo_batch_mode() -> Result<()> {
    info!("--- Demo 2: Batch Mode ---");

    let client = MockClient::new(ClientMode::Batch { max_concurrent: 3 });

    let queries: Vec<String> = (1..=10).map(|i| format!("What is {i} squared?")).collect();

    info!(
        "Processing {} queries with max concurrency 3",
        queries.len()
    );
    let start = Instant::now();

    let results = client.process_batch(queries.clone()).await;
    let elapsed = start.elapsed();

    let successful = results.iter().filter(|r| r.is_ok()).count();
    info!("Completed: {}/{} successful", successful, results.len());
    info!("Total time: {:?}", elapsed);
    info!(
        "Average time per query: {:?}",
        elapsed / results.len() as u32
    );

    // Show first few results
    for (i, result) in results.iter().take(3).enumerate() {
        if let Ok(messages) = result {
            for msg in messages {
                if let Message::Assistant { message } = msg {
                    for content in &message.content {
                        if let ContentBlock::Text(text) = content {
                            info!("Result {}: {}", i + 1, text.text);
                        }
                    }
                }
            }
        }
    }
    info!("...\n");

    Ok(())
}

/// Compare performance between modes
async fn demo_performance_comparison() -> Result<()> {
    info!("--- Demo 3: Performance Comparison ---");

    let queries: Vec<String> = (1..=5).map(|i| format!("What is {i} squared?")).collect();

    // Test with OneShot mode (sequential)
    let oneshot_client = MockClient::new(ClientMode::OneShot);
    let start = Instant::now();
    for query in &queries {
        let _ = oneshot_client.query(query.clone()).await?;
    }
    let oneshot_time = start.elapsed();
    info!("OneShot mode (sequential): {:?}", oneshot_time);

    // Test with Batch mode (concurrent)
    let batch_client = MockClient::new(ClientMode::Batch { max_concurrent: 5 });
    let start = Instant::now();
    let _ = batch_client.process_batch(queries.clone()).await;
    let batch_time = start.elapsed();
    info!("Batch mode (concurrent): {:?}", batch_time);

    let speedup = oneshot_time.as_secs_f64() / batch_time.as_secs_f64();
    info!("Speedup: {:.2}x\n", speedup);

    Ok(())
}
