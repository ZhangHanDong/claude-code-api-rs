//! Model selection recommendations for token optimization
//!
//! This module provides utilities to help choose the most cost-effective Claude model
//! based on task complexity and requirements.

use std::collections::HashMap;

/// Model recommendation helper
///
/// Provides recommendations for which Claude model to use based on task type.
/// You can use the default recommendations or provide custom mappings.
#[derive(Debug, Clone)]
pub struct ModelRecommendation {
    recommendations: HashMap<String, String>,
}

impl ModelRecommendation {
    /// Create with default recommendations
    ///
    /// Default mappings:
    /// - "simple" / "fast" / "cheap" → claude-3-5-haiku-20241022 (fastest, cheapest)
    /// - "balanced" / "general" / "latest" → claude-sonnet-4-5-20250929 (latest Sonnet, balanced performance/cost)
    /// - "complex" / "best" / "quality" → opus (most capable)
    ///
    /// # Example
    ///
    /// ```rust
    /// use nexus_claude::model_recommendation::ModelRecommendation;
    ///
    /// let recommender = ModelRecommendation::default();
    /// let model = recommender.suggest("simple").unwrap();
    /// assert_eq!(model, "claude-3-5-haiku-20241022");
    /// ```
    pub fn with_defaults() -> Self {
        let mut map = HashMap::new();

        // Simple/fast tasks - use Haiku (cheapest, fastest)
        map.insert(
            "simple".to_string(),
            "claude-3-5-haiku-20241022".to_string(),
        );
        map.insert("fast".to_string(), "claude-3-5-haiku-20241022".to_string());
        map.insert("cheap".to_string(), "claude-3-5-haiku-20241022".to_string());
        map.insert("quick".to_string(), "claude-3-5-haiku-20241022".to_string());

        // Balanced tasks - use Sonnet 4.5 (good balance, latest)
        map.insert(
            "balanced".to_string(),
            "claude-sonnet-4-5-20250929".to_string(),
        );
        map.insert(
            "general".to_string(),
            "claude-sonnet-4-5-20250929".to_string(),
        );
        map.insert(
            "normal".to_string(),
            "claude-sonnet-4-5-20250929".to_string(),
        );
        map.insert(
            "standard".to_string(),
            "claude-sonnet-4-5-20250929".to_string(),
        );
        map.insert(
            "latest".to_string(),
            "claude-sonnet-4-5-20250929".to_string(),
        );

        // Complex/critical tasks - use Opus (most capable)
        map.insert("complex".to_string(), "opus".to_string());
        map.insert("best".to_string(), "opus".to_string());
        map.insert("quality".to_string(), "opus".to_string());
        map.insert("critical".to_string(), "opus".to_string());
        map.insert("advanced".to_string(), "opus".to_string());

        Self {
            recommendations: map,
        }
    }

    /// Create with custom recommendations
    ///
    /// # Example
    ///
    /// ```rust
    /// use nexus_claude::model_recommendation::ModelRecommendation;
    /// use std::collections::HashMap;
    ///
    /// let mut custom_map = HashMap::new();
    /// custom_map.insert("code_review".to_string(), "sonnet".to_string());
    /// custom_map.insert("documentation".to_string(), "claude-3-5-haiku-20241022".to_string());
    ///
    /// let recommender = ModelRecommendation::custom(custom_map);
    /// ```
    pub fn custom(recommendations: HashMap<String, String>) -> Self {
        Self { recommendations }
    }

    /// Get a model suggestion for a given task type
    ///
    /// Returns the recommended model name, or None if no recommendation exists.
    ///
    /// # Example
    ///
    /// ```rust
    /// use nexus_claude::model_recommendation::ModelRecommendation;
    ///
    /// let recommender = ModelRecommendation::default();
    ///
    /// // For simple tasks, use Haiku
    /// assert_eq!(recommender.suggest("simple"), Some("claude-3-5-haiku-20241022"));
    ///
    /// // For complex tasks, use Opus
    /// assert_eq!(recommender.suggest("complex"), Some("opus"));
    /// ```
    pub fn suggest(&self, task_type: &str) -> Option<&str> {
        self.recommendations.get(task_type).map(|s| s.as_str())
    }

    /// Add or update a recommendation
    ///
    /// # Example
    ///
    /// ```rust
    /// use nexus_claude::model_recommendation::ModelRecommendation;
    ///
    /// let mut recommender = ModelRecommendation::default();
    /// recommender.add("my_task", "sonnet");
    /// assert_eq!(recommender.suggest("my_task"), Some("sonnet"));
    /// ```
    pub fn add(&mut self, task_type: impl Into<String>, model: impl Into<String>) {
        self.recommendations.insert(task_type.into(), model.into());
    }

    /// Remove a recommendation
    pub fn remove(&mut self, task_type: &str) -> Option<String> {
        self.recommendations.remove(task_type)
    }

    /// Get all task types with recommendations
    pub fn task_types(&self) -> Vec<&str> {
        self.recommendations.keys().map(|s| s.as_str()).collect()
    }

    /// Get all available recommendations
    pub fn all_recommendations(&self) -> &HashMap<String, String> {
        &self.recommendations
    }
}

impl Default for ModelRecommendation {
    fn default() -> Self {
        // Use our predefined defaults
        ModelRecommendation::with_defaults()
    }
}

/// Quick helper functions for common use cases
/// Get the cheapest/fastest model (Haiku)
pub fn cheapest_model() -> &'static str {
    "claude-3-5-haiku-20241022"
}

/// Get the balanced model (Sonnet 4.5 - latest)
pub fn balanced_model() -> &'static str {
    "claude-sonnet-4-5-20250929"
}

/// Get the latest Sonnet model alias
pub fn latest_sonnet() -> &'static str {
    "claude-sonnet-4-5-20250929"
}

/// Get the most capable model (Opus)
pub fn best_model() -> &'static str {
    "opus"
}

/// Estimate relative cost multiplier for different models
///
/// Returns approximate cost multiplier relative to Haiku (1.0x).
/// These are rough estimates and actual costs depend on usage patterns.
///
/// # Example
///
/// ```rust
/// use nexus_claude::model_recommendation::estimate_cost_multiplier;
///
/// // Haiku is baseline (1.0x)
/// assert_eq!(estimate_cost_multiplier("claude-3-5-haiku-20241022"), 1.0);
///
/// // Sonnet is ~5x more expensive
/// assert_eq!(estimate_cost_multiplier("sonnet"), 5.0);
///
/// // Opus is ~15x more expensive
/// assert_eq!(estimate_cost_multiplier("opus"), 15.0);
/// ```
pub fn estimate_cost_multiplier(model: &str) -> f64 {
    match model {
        // Haiku - baseline (cheapest)
        "haiku" | "claude-3-5-haiku-20241022" => 1.0,

        // Sonnet - ~5x more expensive than Haiku
        "sonnet"
        | "claude-sonnet-4-5-20250929"  // Sonnet 4.5 (latest)
        | "claude-sonnet-4-20250514"    // Sonnet 4
        | "claude-3-5-sonnet-20241022"  // Sonnet 3.5
        => 5.0,

        // Opus - ~15x more expensive than Haiku
        "opus" | "claude-opus-4-1-20250805" => 15.0,

        // Unknown - assume Sonnet level
        _ => 5.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_recommendations() {
        let recommender = ModelRecommendation::default();

        assert_eq!(
            recommender.suggest("simple"),
            Some("claude-3-5-haiku-20241022")
        );
        assert_eq!(
            recommender.suggest("fast"),
            Some("claude-3-5-haiku-20241022")
        );
        assert_eq!(
            recommender.suggest("balanced"),
            Some("claude-sonnet-4-5-20250929")
        );
        assert_eq!(
            recommender.suggest("latest"),
            Some("claude-sonnet-4-5-20250929")
        );
        assert_eq!(recommender.suggest("complex"), Some("opus"));
        assert_eq!(recommender.suggest("unknown"), None);
    }

    #[test]
    fn test_custom_recommendations() {
        let mut map = HashMap::new();
        map.insert("code_review".to_string(), "sonnet".to_string());

        let recommender = ModelRecommendation::custom(map);
        assert_eq!(recommender.suggest("code_review"), Some("sonnet"));
    }

    #[test]
    fn test_add_remove() {
        let mut recommender = ModelRecommendation::default();

        recommender.add("my_task", "sonnet");
        assert_eq!(recommender.suggest("my_task"), Some("sonnet"));

        recommender.remove("my_task");
        assert_eq!(recommender.suggest("my_task"), None);
    }

    #[test]
    fn test_cost_multipliers() {
        assert_eq!(estimate_cost_multiplier("haiku"), 1.0);
        assert_eq!(estimate_cost_multiplier("sonnet"), 5.0);
        assert_eq!(estimate_cost_multiplier("opus"), 15.0);
    }

    #[test]
    fn test_quick_helpers() {
        assert_eq!(cheapest_model(), "claude-3-5-haiku-20241022");
        assert_eq!(balanced_model(), "claude-sonnet-4-5-20250929");
        assert_eq!(latest_sonnet(), "claude-sonnet-4-5-20250929");
        assert_eq!(best_model(), "opus");
    }
}
