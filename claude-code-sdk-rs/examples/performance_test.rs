//! Performance testing for the optimized Claude Code SDK

use nexus_claude::{
    ClaudeCodeOptions, ClientMode, OptimizedClient, PerformanceMetrics, PermissionMode, Result,
};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{Level, info};

#[derive(Debug, Clone)]
struct TestResult {
    name: String,
    duration: Duration,
    success: bool,
    error: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    info!("=== Claude Code SDK Performance Test ===\n");

    let options = ClaudeCodeOptions::builder()
        .permission_mode(PermissionMode::AcceptEdits)
        .model("claude-3.5-sonnet")
        .build();

    let mut results = Vec::new();

    // Test 1: Single query latency
    info!("Test 1: Single Query Latency");
    results.push(test_single_query_latency(options.clone()).await);

    // Test 2: Connection pooling effectiveness
    info!("\nTest 2: Connection Pooling");
    results.push(test_connection_pooling(options.clone()).await);

    // Test 3: Concurrent request throughput
    info!("\nTest 3: Concurrent Request Throughput");
    results.push(test_concurrent_throughput(options.clone()).await);

    // Test 4: Interactive session latency
    info!("\nTest 4: Interactive Session");
    results.push(test_interactive_latency(options.clone()).await);

    // Test 5: Large batch processing
    info!("\nTest 5: Large Batch Processing");
    results.push(test_large_batch(options.clone()).await);

    // Print summary
    print_test_summary(&results);

    Ok(())
}

/// Test single query latency
async fn test_single_query_latency(options: ClaudeCodeOptions) -> TestResult {
    let client = OptimizedClient::new(options, ClientMode::OneShot).unwrap();

    let start = Instant::now();
    let result = client.query("What is 2 + 2?".to_string()).await;
    let duration = start.elapsed();

    match result {
        Ok(_) => {
            info!("  Single query completed in {:?}", duration);
            TestResult {
                name: "Single Query Latency".to_string(),
                duration,
                success: true,
                error: None,
            }
        },
        Err(e) => {
            info!("  Single query failed: {}", e);
            TestResult {
                name: "Single Query Latency".to_string(),
                duration,
                success: false,
                error: Some(e.to_string()),
            }
        },
    }
}

/// Test connection pooling effectiveness
async fn test_connection_pooling(options: ClaudeCodeOptions) -> TestResult {
    let client = OptimizedClient::new(options, ClientMode::OneShot).unwrap();

    let mut durations = Vec::new();
    let start = Instant::now();

    for i in 0..5 {
        let query_start = Instant::now();
        match client.query(format!("What is {i} + {i}?")).await {
            Ok(_) => {
                let query_duration = query_start.elapsed();
                durations.push(query_duration);
                info!("  Query {} completed in {:?}", i + 1, query_duration);
            },
            Err(e) => {
                info!("  Query {} failed: {}", i + 1, e);
                return TestResult {
                    name: "Connection Pooling".to_string(),
                    duration: start.elapsed(),
                    success: false,
                    error: Some(e.to_string()),
                };
            },
        }
    }

    let total_duration = start.elapsed();
    let avg_duration = durations.iter().sum::<Duration>() / durations.len() as u32;

    info!("  Average query time: {:?}", avg_duration);
    info!(
        "  First query: {:?}, Last query: {:?}",
        durations[0], durations[4]
    );

    // Check if pooling is effective (later queries should be faster)
    let improvement = if durations[4] < durations[0] {
        let percent = ((durations[0].as_millis() - durations[4].as_millis()) as f64
            / durations[0].as_millis() as f64)
            * 100.0;
        info!("  Performance improvement: {:.1}%", percent);
        true
    } else {
        false
    };

    TestResult {
        name: "Connection Pooling".to_string(),
        duration: total_duration,
        success: true,
        error: if improvement {
            None
        } else {
            Some("No improvement detected".to_string())
        },
    }
}

/// Test concurrent request throughput
async fn test_concurrent_throughput(options: ClaudeCodeOptions) -> TestResult {
    let client =
        Arc::new(OptimizedClient::new(options, ClientMode::Batch { max_concurrent: 5 }).unwrap());

    let queries = [
        "What is 1 + 1?",
        "What is 2 + 2?",
        "What is 3 + 3?",
        "What is 4 + 4?",
        "What is 5 + 5?",
        "What is 6 + 6?",
        "What is 7 + 7?",
        "What is 8 + 8?",
        "What is 9 + 9?",
        "What is 10 + 10?",
    ];

    let start = Instant::now();
    let results = client
        .process_batch(queries.iter().map(|q| q.to_string()).collect())
        .await
        .unwrap();
    let duration = start.elapsed();

    let successful = results.iter().filter(|r| r.is_ok()).count();
    let qps = queries.len() as f64 / duration.as_secs_f64();

    info!("  Processed {} queries in {:?}", queries.len(), duration);
    info!("  Successful: {}/{}", successful, queries.len());
    info!("  Throughput: {:.2} queries/second", qps);

    TestResult {
        name: "Concurrent Throughput".to_string(),
        duration,
        success: successful == queries.len(),
        error: if successful < queries.len() {
            Some(format!("{} queries failed", queries.len() - successful))
        } else {
            None
        },
    }
}

/// Test interactive session latency
async fn test_interactive_latency(options: ClaudeCodeOptions) -> TestResult {
    let client = OptimizedClient::new(options, ClientMode::Interactive).unwrap();

    let start = Instant::now();

    match client.start_interactive_session().await {
        Ok(_) => {
            let mut round_trip_times = Vec::new();

            for i in 0..3 {
                let msg_start = Instant::now();

                if let Err(e) = client.send_interactive(format!("Message {i}")).await {
                    return TestResult {
                        name: "Interactive Session".to_string(),
                        duration: start.elapsed(),
                        success: false,
                        error: Some(format!("Send failed: {e}")),
                    };
                }

                match client.receive_interactive().await {
                    Ok(_) => {
                        let round_trip = msg_start.elapsed();
                        round_trip_times.push(round_trip);
                        info!("  Round trip {}: {:?}", i + 1, round_trip);
                    },
                    Err(e) => {
                        return TestResult {
                            name: "Interactive Session".to_string(),
                            duration: start.elapsed(),
                            success: false,
                            error: Some(format!("Receive failed: {e}")),
                        };
                    },
                }
            }

            let _ = client.end_interactive_session().await;
            let total_duration = start.elapsed();

            let avg_round_trip =
                round_trip_times.iter().sum::<Duration>() / round_trip_times.len() as u32;
            info!("  Average round trip: {:?}", avg_round_trip);

            TestResult {
                name: "Interactive Session".to_string(),
                duration: total_duration,
                success: true,
                error: None,
            }
        },
        Err(e) => TestResult {
            name: "Interactive Session".to_string(),
            duration: start.elapsed(),
            success: false,
            error: Some(format!("Session start failed: {e}")),
        },
    }
}

/// Test large batch processing
async fn test_large_batch(options: ClaudeCodeOptions) -> TestResult {
    let client =
        Arc::new(OptimizedClient::new(options, ClientMode::Batch { max_concurrent: 10 }).unwrap());

    let metrics = Arc::new(RwLock::new(PerformanceMetrics::default()));

    // Generate 20 queries
    let queries: Vec<String> = (1..=20).map(|i| format!("What is {i} squared?")).collect();

    info!(
        "  Processing {} queries with max concurrency 10",
        queries.len()
    );
    let start = Instant::now();

    match client.process_batch(queries.clone()).await {
        Ok(results) => {
            let duration = start.elapsed();

            // Update metrics
            for result in results.iter() {
                match result {
                    Ok(_) => {
                        let latency = (duration.as_millis() / results.len() as u128) as u64;
                        metrics.write().await.record_success(latency);
                    },
                    Err(_) => {
                        metrics.write().await.record_failure();
                    },
                }
            }

            let final_metrics = metrics.read().await;
            info!("  Total time: {:?}", duration);
            info!(
                "  Success rate: {:.1}%",
                final_metrics.success_rate() * 100.0
            );
            info!(
                "  Average latency: {:.0}ms",
                final_metrics.average_latency_ms()
            );
            info!(
                "  Throughput: {:.2} queries/second",
                queries.len() as f64 / duration.as_secs_f64()
            );

            TestResult {
                name: "Large Batch Processing".to_string(),
                duration,
                success: final_metrics.success_rate() > 0.8,
                error: if final_metrics.success_rate() < 0.8 {
                    Some(format!(
                        "Low success rate: {:.1}%",
                        final_metrics.success_rate() * 100.0
                    ))
                } else {
                    None
                },
            }
        },
        Err(e) => TestResult {
            name: "Large Batch Processing".to_string(),
            duration: start.elapsed(),
            success: false,
            error: Some(e.to_string()),
        },
    }
}

/// Print test summary
fn print_test_summary(results: &[TestResult]) {
    info!("\n=== Test Summary ===");
    info!(
        "{:<30} {:<15} {:<15} {}",
        "Test", "Duration", "Status", "Notes"
    );
    info!("{}", "-".repeat(80));

    for result in results {
        let status = if result.success {
            "✓ PASS"
        } else {
            "✗ FAIL"
        };
        let notes = result.error.as_deref().unwrap_or("-");
        info!(
            "{:<30} {:>12.2?} {:<15} {}",
            result.name, result.duration, status, notes
        );
    }

    let total_passed = results.iter().filter(|r| r.success).count();
    let total_duration: Duration = results.iter().map(|r| r.duration).sum();

    info!("{}", "-".repeat(80));
    info!(
        "Total: {}/{} passed, Total time: {:?}",
        total_passed,
        results.len(),
        total_duration
    );
}
