//! Token-efficient usage example
//!
//! Demonstrates best practices for minimizing token consumption and costs.

use futures::StreamExt;
use nexus_claude::model_recommendation::ModelRecommendation;
use nexus_claude::token_tracker::{BudgetLimit, BudgetWarningCallback};
use nexus_claude::{ClaudeCodeOptions, ClaudeSDKClient, PermissionMode, Result};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Token-Efficient Claude Code Usage ===\n");

    // Choose cost-effective model based on task
    let recommender = ModelRecommendation::default();
    let model = recommender.suggest("simple").unwrap();
    println!("📌 Using model: {model} (cheapest option)");

    // Configure for minimal token usage
    let options = ClaudeCodeOptions::builder()
        .model(model)
        .max_turns(2)                          // Limit conversation length
        .max_output_tokens(1500)                // Cap response size
        .allowed_tools(vec!["Read".to_string()]) // Only essential tools
        .permission_mode(PermissionMode::BypassPermissions) // Skip prompts
        .build();

    let mut client = ClaudeSDKClient::new(options);

    // Set budget with warning callback
    println!("💰 Setting budget: $1.00 max\n");
    let callback: BudgetWarningCallback =
        Arc::new(|msg: &str| eprintln!("⚠️  Budget Alert: {msg}"));
    client
        .set_budget_limit(
            BudgetLimit::with_cost(1.0).with_warning_threshold(0.8),
            Some(callback),
        )
        .await;

    // Simple query
    println!("🔍 Query: What is 2+2?\n");
    client
        .connect(Some("What is 2+2? Give a brief answer.".to_string()))
        .await?;

    let mut messages = client.receive_messages().await;
    while let Some(msg) = messages.next().await {
        if let Ok(message) = msg {
            match message {
                nexus_claude::Message::Assistant { message } => {
                    for block in &message.content {
                        if let nexus_claude::ContentBlock::Text(text) = block {
                            println!("💬 Response: {}", text.text);
                        }
                    }
                },
                nexus_claude::Message::Result { .. } => break,
                _ => {},
            }
        }
    }

    // Display usage stats
    let usage = client.get_usage_stats().await;
    println!("\n📊 Usage Statistics:");
    println!("   Total tokens: {}", usage.total_tokens());
    println!("   - Input:  {} tokens", usage.total_input_tokens);
    println!("   - Output: {} tokens", usage.total_output_tokens);
    println!("   Cost: ${:.4}", usage.total_cost_usd);
    println!("   Sessions: {}", usage.session_count);

    if usage.session_count > 0 {
        println!(
            "   Avg per session: {:.0} tokens",
            usage.avg_tokens_per_session()
        );
    }

    client.disconnect().await?;

    println!("\n✅ Demo complete!");
    println!("💡 Compare this cost to using Opus without limits (~10-15x more expensive)");

    Ok(())
}
