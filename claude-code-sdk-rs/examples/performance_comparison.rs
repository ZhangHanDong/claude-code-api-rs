//! Performance comparison between normal and optimized clients

use cc_sdk::{
    ClaudeCodeOptions, ClientMode, OptimizedClient, InteractiveClient,
    PermissionMode, Result,
};
use std::time::Instant;
use tracing::{info, Level};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    info!("=== Performance Comparison: Normal vs Optimized ===\n");

    let options = ClaudeCodeOptions::builder()
        .permission_mode(PermissionMode::AcceptEdits)
        .build();

    // Test queries
    let queries = vec![
        "What is 10 + 10?",
        "What is 20 + 20?",
        "What is 30 + 30?",
    ];

    // Test 1: Traditional InteractiveClient (creates new process each time)
    info!("Test 1: Traditional InteractiveClient");
    info!("(Creates new claude-code process for each connection)\n");
    
    let mut traditional_times = Vec::new();
    for (i, query) in queries.iter().enumerate() {
        let start = Instant::now();
        
        let mut client = InteractiveClient::new(options.clone())?;
        client.connect().await?;
        let _ = client.send_and_receive(query.to_string()).await?;
        client.disconnect().await?;
        
        let elapsed = start.elapsed();
        traditional_times.push(elapsed);
        info!("Query {}: {:?}", i + 1, elapsed);
    }
    
    let traditional_avg = traditional_times.iter().sum::<std::time::Duration>() / traditional_times.len() as u32;
    info!("Average time: {:?}\n", traditional_avg);

    // Test 2: OptimizedClient with connection pooling
    info!("Test 2: OptimizedClient with Connection Pooling");
    info!("(Reuses claude-code processes, pre-warmed)\n");
    
    let optimized_client = OptimizedClient::new(options, ClientMode::OneShot)?;
    
    // Pre-warm the connection pool
    info!("Pre-warming connection pool...");
    let warmup_start = Instant::now();
    let _ = optimized_client.query("Hi".to_string()).await?;
    info!("Pool warmed up in {:?}\n", warmup_start.elapsed());
    
    let mut optimized_times = Vec::new();
    for (i, query) in queries.iter().enumerate() {
        let start = Instant::now();
        let _ = optimized_client.query(query.to_string()).await?;
        let elapsed = start.elapsed();
        optimized_times.push(elapsed);
        info!("Query {}: {:?}", i + 1, elapsed);
    }
    
    let optimized_avg = optimized_times.iter().sum::<std::time::Duration>() / optimized_times.len() as u32;
    info!("Average time: {:?}\n", optimized_avg);

    // Summary
    info!("=== Summary ===");
    info!("Traditional average: {:?}", traditional_avg);
    info!("Optimized average: {:?}", optimized_avg);
    
    let improvement = (traditional_avg.as_millis() as f64 - optimized_avg.as_millis() as f64) 
                     / traditional_avg.as_millis() as f64 * 100.0;
    info!("Performance improvement: {:.1}%", improvement);
    
    info!("\nKey optimizations:");
    info!("✓ Connection pooling - reuses claude-code processes");
    info!("✓ Pre-warming - first connection established early");
    info!("✓ Reduced overhead - no process creation per query");

    Ok(())
}