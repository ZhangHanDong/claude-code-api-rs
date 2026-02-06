//! Tests for token optimization features

use nexus_claude::ClaudeCodeOptions;
use nexus_claude::model_recommendation::ModelRecommendation;
use nexus_claude::token_tracker::{BudgetLimit, BudgetManager, BudgetStatus, TokenUsageTracker};

#[test]
fn test_token_tracker_basic() {
    let mut tracker = TokenUsageTracker::new();
    assert_eq!(tracker.total_tokens(), 0);

    tracker.update(100, 200, 0.05);
    assert_eq!(tracker.total_input_tokens, 100);
    assert_eq!(tracker.total_output_tokens, 200);
    assert_eq!(tracker.total_tokens(), 300);
    assert_eq!(tracker.total_cost_usd, 0.05);
    assert_eq!(tracker.session_count, 1);

    tracker.update(50, 100, 0.02);
    assert_eq!(tracker.total_tokens(), 450);
    assert_eq!(tracker.session_count, 2);
}

#[test]
fn test_token_tracker_averages() {
    let mut tracker = TokenUsageTracker::new();
    tracker.update(100, 100, 0.02);
    tracker.update(200, 200, 0.04);

    assert_eq!(tracker.avg_tokens_per_session(), 300.0);
    assert_eq!(tracker.avg_cost_per_session(), 0.03);
}

#[test]
fn test_budget_limit_cost() {
    let limit = BudgetLimit::with_cost(1.0);

    let mut tracker = TokenUsageTracker::new();
    tracker.update(100, 200, 0.5);
    assert!(matches!(limit.check_limits(&tracker), BudgetStatus::Ok));

    tracker.update(100, 200, 0.35);
    assert!(matches!(
        limit.check_limits(&tracker),
        BudgetStatus::Warning { .. }
    ));

    tracker.update(100, 200, 0.2);
    assert!(matches!(
        limit.check_limits(&tracker),
        BudgetStatus::Exceeded
    ));
}

#[test]
fn test_budget_limit_tokens() {
    let limit = BudgetLimit::with_tokens(1000);

    let mut tracker = TokenUsageTracker::new();
    tracker.update(300, 200, 0.05);
    assert!(matches!(limit.check_limits(&tracker), BudgetStatus::Ok));

    tracker.update(300, 300, 0.05);
    assert!(matches!(
        limit.check_limits(&tracker),
        BudgetStatus::Exceeded
    ));
}

#[tokio::test]
async fn test_budget_manager() {
    let manager = BudgetManager::new();

    manager.set_limit(BudgetLimit::with_tokens(1000)).await;
    manager.update_usage(300, 200, 0.05).await;

    let usage = manager.get_usage().await;
    assert_eq!(usage.total_tokens(), 500);

    assert!(!manager.is_exceeded().await);

    manager.update_usage(300, 300, 0.05).await;
    assert!(manager.is_exceeded().await);
}

#[test]
fn test_model_recommendations() {
    let recommender = ModelRecommendation::default();

    assert_eq!(
        recommender.suggest("simple"),
        Some("claude-3-5-haiku-20241022")
    );
    assert_eq!(
        recommender.suggest("fast"),
        Some("claude-3-5-haiku-20241022")
    );
    // balanced now returns full Sonnet 4.5 model ID
    assert_eq!(
        recommender.suggest("balanced"),
        Some("claude-sonnet-4-5-20250929")
    );
    assert_eq!(recommender.suggest("complex"), Some("opus"));
    assert_eq!(recommender.suggest("unknown"), None);
}

#[test]
fn test_custom_model_recommendations() {
    let mut recommender = ModelRecommendation::default();

    recommender.add("my_task", "sonnet");
    assert_eq!(recommender.suggest("my_task"), Some("sonnet"));

    recommender.remove("my_task");
    assert_eq!(recommender.suggest("my_task"), None);
}

#[test]
fn test_max_output_tokens_option() {
    let options = ClaudeCodeOptions::builder().max_output_tokens(2000).build();

    assert_eq!(options.max_output_tokens, Some(2000));
}

#[test]
fn test_max_output_tokens_clamping() {
    // Should clamp to 32000
    let options = ClaudeCodeOptions::builder()
        .max_output_tokens(50000)
        .build();

    assert_eq!(options.max_output_tokens, Some(32000));

    // Should clamp to 1
    let options2 = ClaudeCodeOptions::builder().max_output_tokens(0).build();

    assert_eq!(options2.max_output_tokens, Some(1));
}
