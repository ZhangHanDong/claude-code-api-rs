//! Performance Benchmark: Command Line vs SDK
//!
//! This example compares the performance of using Claude via command line
//! versus using the SDK for batch processing.

use nexus_claude::{ClaudeCodeOptions, InteractiveClient, PermissionMode, Result};
use std::process::Command;
use std::time::{Duration, Instant};
use tokio::time::sleep;

/// Benchmark result structure
#[derive(Debug)]
struct BenchmarkResult {
    #[allow(dead_code)]
    method: String,
    total_duration: Duration,
    per_query_avg: Duration,
    queries_completed: usize,
}

/// Run command line benchmark
fn benchmark_command_line(queries: &[&str]) -> BenchmarkResult {
    println!("🔧 Running Command Line Benchmark...");
    let start = Instant::now();
    let mut completed = 0;

    for (idx, query) in queries.iter().enumerate() {
        println!("  CMD Query {}/{}: {}", idx + 1, queries.len(), query);
        let query_start = Instant::now();

        let output = Command::new("claude")
            .args(["-p", "--max-turns", "5", "--model", "sonnet", query])
            .output();

        match output {
            Ok(output) => {
                if output.status.success() {
                    completed += 1;
                    println!(
                        "    ✓ Completed in {:.2}s",
                        query_start.elapsed().as_secs_f64()
                    );
                } else {
                    println!("    ✗ Failed with status: {}", output.status);
                }
            },
            Err(e) => {
                println!("    ✗ Error: {e}");
            },
        }

        // Small delay between queries
        std::thread::sleep(Duration::from_millis(500));
    }

    let total_duration = start.elapsed();
    BenchmarkResult {
        method: "Command Line".to_string(),
        total_duration,
        per_query_avg: total_duration / queries.len() as u32,
        queries_completed: completed,
    }
}

/// Run SDK benchmark
async fn benchmark_sdk(queries: &[&str]) -> Result<BenchmarkResult> {
    println!("\n🚀 Running SDK Benchmark...");
    let start = Instant::now();
    let mut completed = 0;

    // Create client with optimized settings for benchmarking
    let options = ClaudeCodeOptions::builder()
        .system_prompt("You are a helpful Rust expert. Provide concise answers.")
        .model("sonnet")
        .permission_mode(PermissionMode::Default)
        .max_turns(5) // Limit turns for faster responses
        .build();

    let mut client = InteractiveClient::new(options)?;

    // Connect once
    let connect_start = Instant::now();
    client.connect().await?;
    println!(
        "  Connection established in {:.2}s",
        connect_start.elapsed().as_secs_f64()
    );

    // Process all queries with the same client
    for (idx, query) in queries.iter().enumerate() {
        println!("  SDK Query {}/{}: {}", idx + 1, queries.len(), query);
        let query_start = Instant::now();

        match client.send_and_receive(query.to_string()).await {
            Ok(_messages) => {
                completed += 1;
                println!(
                    "    ✓ Completed in {:.2}s",
                    query_start.elapsed().as_secs_f64()
                );
            },
            Err(e) => {
                println!("    ✗ Error: {e:?}");
            },
        }

        // Small delay between queries
        sleep(Duration::from_millis(500)).await;
    }

    client.disconnect().await?;

    let total_duration = start.elapsed();
    Ok(BenchmarkResult {
        method: "SDK".to_string(),
        total_duration,
        per_query_avg: total_duration / queries.len() as u32,
        queries_completed: completed,
    })
}

/// Print benchmark comparison
fn print_comparison(cmd_result: &BenchmarkResult, sdk_result: &BenchmarkResult) {
    println!("\n📊 BENCHMARK RESULTS");
    println!("{}", "=".repeat(60));

    println!("\n📌 Command Line:");
    println!(
        "  Total time: {:.2}s",
        cmd_result.total_duration.as_secs_f64()
    );
    println!(
        "  Average per query: {:.2}s",
        cmd_result.per_query_avg.as_secs_f64()
    );
    println!("  Queries completed: {}", cmd_result.queries_completed);

    println!("\n📌 SDK:");
    println!(
        "  Total time: {:.2}s",
        sdk_result.total_duration.as_secs_f64()
    );
    println!(
        "  Average per query: {:.2}s",
        sdk_result.per_query_avg.as_secs_f64()
    );
    println!("  Queries completed: {}", sdk_result.queries_completed);

    // Calculate performance improvement
    let improvement = (cmd_result.total_duration.as_secs_f64()
        - sdk_result.total_duration.as_secs_f64())
        / cmd_result.total_duration.as_secs_f64()
        * 100.0;

    println!("\n🎯 Performance Improvement:");
    println!("  SDK is {improvement:.1}% faster than command line");
    println!(
        "  Time saved: {:.2}s",
        (cmd_result.total_duration - sdk_result.total_duration).as_secs_f64()
    );

    // Per-query improvement
    let per_query_improvement = (cmd_result.per_query_avg.as_secs_f64()
        - sdk_result.per_query_avg.as_secs_f64())
        / cmd_result.per_query_avg.as_secs_f64()
        * 100.0;
    println!("  Per-query improvement: {per_query_improvement:.1}%");

    println!("\n📈 Extrapolated Performance:");
    println!("  For 100 queries:");
    println!(
        "    Command line: ~{:.1} minutes",
        cmd_result.per_query_avg.as_secs_f64() * 100.0 / 60.0
    );
    println!(
        "    SDK: ~{:.1} minutes",
        sdk_result.per_query_avg.as_secs_f64() * 100.0 / 60.0
    );
    println!(
        "    Time saved: ~{:.1} minutes",
        (cmd_result.per_query_avg.as_secs_f64() - sdk_result.per_query_avg.as_secs_f64()) * 100.0
            / 60.0
    );
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("🏁 Claude SDK Performance Benchmark");
    println!("{}", "=".repeat(60));

    // Test queries - simple to avoid long processing times
    let test_queries = vec![
        "Write a one-line Rust function to check if a number is even",
        "What is the syntax for a match expression in Rust?",
        "How do I create a vector of integers in Rust?",
        "Write a simple hello world function in Rust",
        "What is the difference between String and &str in Rust?",
    ];

    println!(
        "📝 Test queries: {} simple Rust questions",
        test_queries.len()
    );
    println!("⚠️  Note: Using simple queries for faster benchmark completion");

    // Run command line benchmark
    let cmd_result = benchmark_command_line(&test_queries);

    // Run SDK benchmark
    let sdk_result = benchmark_sdk(&test_queries).await?;

    // Print comparison
    print_comparison(&cmd_result, &sdk_result);

    println!("\n✅ Benchmark completed!");
    Ok(())
}
