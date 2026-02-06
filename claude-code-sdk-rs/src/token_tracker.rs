//! Token usage tracking and budget management
//!
//! This module provides utilities for monitoring token consumption and managing budgets
//! to help control costs when using Claude Code.

use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::warn;

/// Token usage statistics tracker
#[derive(Debug, Clone, Default)]
pub struct TokenUsageTracker {
    /// Total input tokens consumed
    pub total_input_tokens: u64,
    /// Total output tokens consumed
    pub total_output_tokens: u64,
    /// Total cost in USD
    pub total_cost_usd: f64,
    /// Number of sessions/queries completed
    pub session_count: usize,
}

impl TokenUsageTracker {
    /// Create a new empty tracker
    pub fn new() -> Self {
        Self::default()
    }

    /// Get total tokens (input + output)
    pub fn total_tokens(&self) -> u64 {
        self.total_input_tokens + self.total_output_tokens
    }

    /// Get average tokens per session
    pub fn avg_tokens_per_session(&self) -> f64 {
        if self.session_count == 0 {
            0.0
        } else {
            self.total_tokens() as f64 / self.session_count as f64
        }
    }

    /// Get average cost per session
    pub fn avg_cost_per_session(&self) -> f64 {
        if self.session_count == 0 {
            0.0
        } else {
            self.total_cost_usd / self.session_count as f64
        }
    }

    /// Update statistics with new usage data
    pub fn update(&mut self, input_tokens: u64, output_tokens: u64, cost_usd: f64) {
        self.total_input_tokens += input_tokens;
        self.total_output_tokens += output_tokens;
        self.total_cost_usd += cost_usd;
        self.session_count += 1;
    }

    /// Reset all statistics to zero
    pub fn reset(&mut self) {
        self.total_input_tokens = 0;
        self.total_output_tokens = 0;
        self.total_cost_usd = 0.0;
        self.session_count = 0;
    }
}

/// Budget limits and alerts
#[derive(Debug, Clone)]
pub struct BudgetLimit {
    /// Maximum cost in USD (None = unlimited)
    pub max_cost_usd: Option<f64>,
    /// Maximum total tokens (None = unlimited)
    pub max_tokens: Option<u64>,
    /// Threshold percentage for warning (0.0-1.0, default 0.8 for 80%)
    pub warning_threshold: f64,
}

impl Default for BudgetLimit {
    fn default() -> Self {
        Self {
            max_cost_usd: None,
            max_tokens: None,
            warning_threshold: 0.8,
        }
    }
}

impl BudgetLimit {
    /// Create a new budget limit with cost cap
    pub fn with_cost(max_cost_usd: f64) -> Self {
        Self {
            max_cost_usd: Some(max_cost_usd),
            ..Default::default()
        }
    }

    /// Create a new budget limit with token cap
    pub fn with_tokens(max_tokens: u64) -> Self {
        Self {
            max_tokens: Some(max_tokens),
            ..Default::default()
        }
    }

    /// Create a new budget limit with both caps
    pub fn with_both(max_cost_usd: f64, max_tokens: u64) -> Self {
        Self {
            max_cost_usd: Some(max_cost_usd),
            max_tokens: Some(max_tokens),
            warning_threshold: 0.8,
        }
    }

    /// Set warning threshold (0.0-1.0)
    pub fn with_warning_threshold(mut self, threshold: f64) -> Self {
        self.warning_threshold = threshold.clamp(0.0, 1.0);
        self
    }

    /// Check if usage exceeds limits
    pub fn check_limits(&self, usage: &TokenUsageTracker) -> BudgetStatus {
        let mut status = BudgetStatus::Ok;

        // Check cost limit
        if let Some(max_cost) = self.max_cost_usd {
            let cost_ratio = usage.total_cost_usd / max_cost;

            if cost_ratio >= 1.0 {
                status = BudgetStatus::Exceeded;
            } else if cost_ratio >= self.warning_threshold {
                status = BudgetStatus::Warning {
                    current_ratio: cost_ratio,
                    message: format!(
                        "Cost usage at {:.1}% (${:.2}/${:.2})",
                        cost_ratio * 100.0,
                        usage.total_cost_usd,
                        max_cost
                    ),
                };
            }
        }

        // Check token limit
        if let Some(max_tokens) = self.max_tokens {
            let token_ratio = usage.total_tokens() as f64 / max_tokens as f64;

            if token_ratio >= 1.0 {
                status = BudgetStatus::Exceeded;
            } else if token_ratio >= self.warning_threshold {
                // If already warning from cost, keep the exceeded state
                if !matches!(status, BudgetStatus::Exceeded) {
                    status = BudgetStatus::Warning {
                        current_ratio: token_ratio,
                        message: format!(
                            "Token usage at {:.1}% ({}/{})",
                            token_ratio * 100.0,
                            usage.total_tokens(),
                            max_tokens
                        ),
                    };
                }
            }
        }

        status
    }
}

/// Budget status result
#[derive(Debug, Clone, PartialEq)]
pub enum BudgetStatus {
    /// Usage is within limits
    Ok,
    /// Usage exceeds warning threshold
    Warning {
        /// Current usage ratio (0.0-1.0)
        current_ratio: f64,
        /// Warning message
        message: String,
    },
    /// Usage exceeds limits
    Exceeded,
}

/// Callback type for budget warnings
pub type BudgetWarningCallback = Arc<dyn Fn(&str) + Send + Sync>;

/// Budget manager that combines tracker and limits
#[derive(Clone)]
pub struct BudgetManager {
    tracker: Arc<RwLock<TokenUsageTracker>>,
    limit: Arc<RwLock<Option<BudgetLimit>>>,
    on_warning: Arc<RwLock<Option<BudgetWarningCallback>>>,
    warning_fired: Arc<RwLock<bool>>,
}

impl BudgetManager {
    /// Create a new budget manager
    pub fn new() -> Self {
        Self {
            tracker: Arc::new(RwLock::new(TokenUsageTracker::new())),
            limit: Arc::new(RwLock::new(None)),
            on_warning: Arc::new(RwLock::new(None)),
            warning_fired: Arc::new(RwLock::new(false)),
        }
    }

    /// Set budget limit
    pub async fn set_limit(&self, limit: BudgetLimit) {
        *self.limit.write().await = Some(limit);
        *self.warning_fired.write().await = false;
    }

    /// Set warning callback
    pub async fn set_warning_callback(&self, callback: BudgetWarningCallback) {
        *self.on_warning.write().await = Some(callback);
    }

    /// Clear budget limit
    pub async fn clear_limit(&self) {
        *self.limit.write().await = None;
        *self.warning_fired.write().await = false;
    }

    /// Get current usage statistics
    pub async fn get_usage(&self) -> TokenUsageTracker {
        self.tracker.read().await.clone()
    }

    /// Update usage and check limits
    pub async fn update_usage(&self, input_tokens: u64, output_tokens: u64, cost_usd: f64) {
        // Update tracker
        self.tracker
            .write()
            .await
            .update(input_tokens, output_tokens, cost_usd);

        // Check limits
        if let Some(limit) = self.limit.read().await.as_ref() {
            let usage = self.tracker.read().await.clone();
            let status = limit.check_limits(&usage);

            match status {
                BudgetStatus::Warning { message, .. } => {
                    let mut fired = self.warning_fired.write().await;
                    if !*fired {
                        *fired = true;
                        warn!("Budget warning: {}", message);

                        if let Some(callback) = self.on_warning.read().await.as_ref() {
                            callback(&message);
                        }
                    }
                },
                BudgetStatus::Exceeded => {
                    warn!(
                        "Budget exceeded! Current usage: {} tokens, ${:.2}",
                        usage.total_tokens(),
                        usage.total_cost_usd
                    );

                    if let Some(callback) = self.on_warning.read().await.as_ref() {
                        callback("Budget limit exceeded");
                    }
                },
                BudgetStatus::Ok => {
                    // Reset warning flag if usage dropped below threshold
                    *self.warning_fired.write().await = false;
                },
            }
        }
    }

    /// Reset usage statistics
    pub async fn reset_usage(&self) {
        self.tracker.write().await.reset();
        *self.warning_fired.write().await = false;
    }

    /// Check if budget is exceeded
    pub async fn is_exceeded(&self) -> bool {
        if let Some(limit) = self.limit.read().await.as_ref() {
            let usage = self.tracker.read().await.clone();
            matches!(limit.check_limits(&usage), BudgetStatus::Exceeded)
        } else {
            false
        }
    }
}

impl Default for BudgetManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tracker_basics() {
        let mut tracker = TokenUsageTracker::new();
        assert_eq!(tracker.total_tokens(), 0);
        assert_eq!(tracker.total_cost_usd, 0.0);

        tracker.update(100, 200, 0.05);
        assert_eq!(tracker.total_input_tokens, 100);
        assert_eq!(tracker.total_output_tokens, 200);
        assert_eq!(tracker.total_tokens(), 300);
        assert_eq!(tracker.total_cost_usd, 0.05);
        assert_eq!(tracker.session_count, 1);

        tracker.update(50, 100, 0.02);
        assert_eq!(tracker.total_tokens(), 450);
        assert_eq!(tracker.total_cost_usd, 0.07);
        assert_eq!(tracker.session_count, 2);
    }

    #[test]
    fn test_budget_limits() {
        let limit = BudgetLimit::with_cost(1.0).with_warning_threshold(0.8);

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
}
