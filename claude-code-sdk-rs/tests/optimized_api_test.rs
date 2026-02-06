//! Integration tests for the optimized API

use nexus_claude::{
    ClaudeCodeOptions, ClientMode, Message, OptimizedClient, PerformanceMetrics, PermissionMode,
    Result, RetryConfig,
};
use std::time::{Duration, Instant};

/// Test basic one-shot query functionality
#[tokio::test]
#[ignore = "Requires Claude CLI to be installed and configured"]
async fn test_oneshot_query() -> Result<()> {
    let options = ClaudeCodeOptions::builder()
        .permission_mode(PermissionMode::AcceptEdits)
        .build();

    let client = OptimizedClient::new(options, ClientMode::OneShot)?;

    // Test simple query
    let messages = client.query("What is 2 + 2?".to_string()).await?;

    // Verify we got a response
    assert!(!messages.is_empty());

    // Check for assistant message
    let has_assistant_msg = messages
        .iter()
        .any(|msg| matches!(msg, Message::Assistant { .. }));
    assert!(
        has_assistant_msg,
        "Should have received an assistant message"
    );

    Ok(())
}

/// Test retry functionality
#[tokio::test]
#[ignore = "Requires Claude CLI to be installed and configured"]
async fn test_retry_logic() -> Result<()> {
    let options = ClaudeCodeOptions::builder()
        .permission_mode(PermissionMode::AcceptEdits)
        .build();

    let client = OptimizedClient::new(options, ClientMode::OneShot)?;

    // Test with custom retry config
    let start = Instant::now();
    let result = client
        .query_with_retry(
            "What is the capital of France?".to_string(),
            2,                         // max retries
            Duration::from_millis(50), // initial delay
        )
        .await;

    // Should succeed
    assert!(result.is_ok());
    let elapsed = start.elapsed();

    // Basic timing check (should be relatively fast for successful query)
    assert!(elapsed < Duration::from_secs(30), "Query took too long");

    Ok(())
}

/// Test interactive mode
#[tokio::test]
#[ignore = "Requires Claude CLI to be installed and configured"]
async fn test_interactive_mode() -> Result<()> {
    let options = ClaudeCodeOptions::builder()
        .permission_mode(PermissionMode::AcceptEdits)
        .build();

    let client = OptimizedClient::new(options, ClientMode::Interactive)?;

    // Start session
    client.start_interactive_session().await?;

    // Send a message
    client
        .send_interactive("Hello, can you hear me?".to_string())
        .await?;

    // Receive response
    let messages = client.receive_interactive().await?;
    assert!(!messages.is_empty(), "Should receive response messages");

    // Send follow-up
    client
        .send_interactive("What's 10 * 10?".to_string())
        .await?;
    let messages = client.receive_interactive().await?;
    assert!(!messages.is_empty(), "Should receive follow-up response");

    // End session
    client.end_interactive_session().await?;

    Ok(())
}

/// Test batch processing
#[tokio::test]
#[ignore = "Requires Claude CLI to be installed and configured"]
async fn test_batch_processing() -> Result<()> {
    let options = ClaudeCodeOptions::builder()
        .permission_mode(PermissionMode::AcceptEdits)
        .build();

    let client = OptimizedClient::new(options, ClientMode::Batch { max_concurrent: 2 })?;

    let queries = vec![
        "What is 1 + 1?".to_string(),
        "What is 2 + 2?".to_string(),
        "What is 3 + 3?".to_string(),
    ];

    let start = Instant::now();
    let results = client.process_batch(queries.clone()).await?;
    let elapsed = start.elapsed();

    // All queries should complete
    assert_eq!(results.len(), queries.len());

    // Count successful queries
    let successful = results.iter().filter(|r| r.is_ok()).count();
    assert!(successful > 0, "At least some queries should succeed");

    println!(
        "Batch processing {} queries took {:?}",
        queries.len(),
        elapsed
    );

    Ok(())
}

/// Test performance metrics
#[test]
fn test_performance_metrics() {
    let mut metrics = PerformanceMetrics::default();

    // Record some operations
    metrics.record_success(100);
    metrics.record_success(200);
    metrics.record_success(150);
    metrics.record_failure();
    metrics.record_failure();

    // Verify metrics
    assert_eq!(metrics.total_requests, 5);
    assert_eq!(metrics.successful_requests, 3);
    assert_eq!(metrics.failed_requests, 2);
    assert_eq!(metrics.average_latency_ms(), 150.0);
    assert_eq!(metrics.success_rate(), 0.6);
    assert_eq!(metrics.min_latency_ms, 100);
    assert_eq!(metrics.max_latency_ms, 200);
}

/// Test retry configuration
#[test]
fn test_retry_config() {
    let config = RetryConfig::default();

    assert_eq!(config.max_retries, 3);
    assert_eq!(config.initial_delay, Duration::from_millis(100));
    assert_eq!(config.max_delay, Duration::from_secs(30));
    assert_eq!(config.backoff_multiplier, 2.0);
    assert!(config.jitter_factor > 0.0);

    // Test custom config
    let custom = RetryConfig {
        max_retries: 5,
        initial_delay: Duration::from_millis(200),
        max_delay: Duration::from_secs(60),
        backoff_multiplier: 1.5,
        jitter_factor: 0.2,
    };

    assert_eq!(custom.max_retries, 5);
    assert_eq!(custom.backoff_multiplier, 1.5);
}

/// Test client mode variants
#[test]
fn test_client_modes() {
    // Test mode creation
    let oneshot = ClientMode::OneShot;
    let interactive = ClientMode::Interactive;
    let batch = ClientMode::Batch { max_concurrent: 10 };

    // Verify pattern matching works
    match oneshot {
        ClientMode::OneShot => (),
        _ => panic!("Expected OneShot mode"),
    }

    match interactive {
        ClientMode::Interactive => (),
        _ => panic!("Expected Interactive mode"),
    }

    match batch {
        ClientMode::Batch { max_concurrent } => {
            assert_eq!(max_concurrent, 10);
        },
        _ => panic!("Expected Batch mode"),
    }
}

/// Test connection pooling behavior
#[tokio::test]
#[ignore = "Requires Claude CLI to be installed and configured"]
async fn test_connection_pooling() -> Result<()> {
    let options = ClaudeCodeOptions::builder()
        .permission_mode(PermissionMode::AcceptEdits)
        .build();

    let client = OptimizedClient::new(options, ClientMode::OneShot)?;

    // Execute multiple queries to test connection reuse
    let mut total_time = Duration::ZERO;
    let mut first_query_time = Duration::ZERO;

    for i in 0..3 {
        let start = Instant::now();
        let _messages = client.query(format!("What is {i} + {i}?")).await?;
        let elapsed = start.elapsed();

        if i == 0 {
            first_query_time = elapsed;
        }
        total_time += elapsed;

        println!("Query {} took {:?}", i + 1, elapsed);
    }

    // Later queries should generally be faster due to connection pooling
    let avg_time = total_time / 3;
    println!("Average query time: {avg_time:?}");
    println!("First query time: {first_query_time:?}");

    Ok(())
}
