//! Token budget monitoring example
//!
//! Demonstrates comprehensive token usage tracking and budget management.

use futures::StreamExt;
use nexus_claude::token_tracker::{BudgetLimit, BudgetWarningCallback};
use nexus_claude::{ClaudeCodeOptions, ClaudeSDKClient, Result};
use std::sync::{Arc, Mutex};

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Token Budget Monitoring Demo ===\n");

    // Track warnings
    let warnings = Arc::new(Mutex::new(Vec::new()));
    let warnings_clone = warnings.clone();

    let options = ClaudeCodeOptions::builder()
        .model("claude-3-5-haiku-20241022")
        .max_turns(3)
        .build();

    let mut client = ClaudeSDKClient::new(options);

    // Set budget with callback
    println!("Setting budget: $0.50 max (warning at 80%)");
    let cb: BudgetWarningCallback = Arc::new(move |msg: &str| {
        println!("⚠️  {msg}");
        warnings_clone.lock().unwrap().push(msg.to_string());
    });
    client
        .set_budget_limit(
            BudgetLimit::with_cost(0.50).with_warning_threshold(0.8),
            Some(cb),
        )
        .await;

    // Run multiple queries to demonstrate tracking
    let queries = [
        "What is 2+2?",
        "What is the capital of France?",
        "Explain quantum computing in one sentence.",
    ];

    for (i, query) in queries.iter().enumerate() {
        println!("\n--- Query {} ---", i + 1);
        println!("Question: {query}");

        client.connect(Some(query.to_string())).await?;

        let mut messages = client.receive_messages().await;
        while let Some(msg) = messages.next().await {
            if let Ok(message) = msg
                && let nexus_claude::Message::Result { .. } = message
            {
                break;
            }
        }

        client.disconnect().await?;

        // Check usage after each query
        let usage = client.get_usage_stats().await;
        println!(
            "Cumulative usage: {} tokens, ${:.4}",
            usage.total_tokens(),
            usage.total_cost_usd
        );

        // Check budget status
        if client.is_budget_exceeded().await {
            println!("❌ Budget exceeded! Stopping.");
            break;
        }
    }

    // Final report
    let usage = client.get_usage_stats().await;
    println!("\n=== Final Report ===");
    println!("Total queries: {}", usage.session_count);
    println!("Total tokens: {}", usage.total_tokens());
    println!("  Input:  {} tokens", usage.total_input_tokens);
    println!("  Output: {} tokens", usage.total_output_tokens);
    println!("Total cost: ${:.4}", usage.total_cost_usd);
    println!(
        "Average per query: {:.0} tokens",
        usage.avg_tokens_per_session()
    );

    let warnings = warnings.lock().unwrap();
    if !warnings.is_empty() {
        println!("\n⚠️  Warnings triggered: {}", warnings.len());
        for warning in warnings.iter() {
            println!("  - {warning}");
        }
    }

    println!("\n✅ Monitoring demo complete!");

    Ok(())
}
