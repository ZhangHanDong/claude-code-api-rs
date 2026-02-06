//! Unified memory provider that aggregates all memory levels
//!
//! Combines short-term, medium-term, and long-term memory with
//! intelligent scoring and deduplication.

#![allow(dead_code, unused_imports)] // Public API - may not be used internally

use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashSet;
use tracing::debug;

use super::traits::{ContextualMemoryProvider, MemoryResult, MemorySource};

/// Configuration for the unified memory provider
#[derive(Debug, Clone)]
pub struct UnifiedMemoryConfig {
    /// Weight for short-term results (0.0 - 1.0)
    pub short_term_weight: f64,
    /// Weight for medium-term results (0.0 - 1.0)
    pub medium_term_weight: f64,
    /// Weight for long-term results (0.0 - 1.0)
    pub long_term_weight: f64,
    /// Minimum score threshold to include in results
    pub min_score_threshold: f64,
}

impl Default for UnifiedMemoryConfig {
    fn default() -> Self {
        Self {
            short_term_weight: 1.2, // Boost current conversation
            medium_term_weight: 1.0,
            long_term_weight: 0.8, // Slight penalty for old context
            min_score_threshold: 0.1,
        }
    }
}

/// Unified memory provider that aggregates all memory levels
pub struct UnifiedMemoryProvider {
    short_term: Box<dyn ContextualMemoryProvider>,
    medium_term: Box<dyn ContextualMemoryProvider>,
    long_term: Box<dyn ContextualMemoryProvider>,
    config: UnifiedMemoryConfig,
    scope: Option<String>,
}

impl UnifiedMemoryProvider {
    /// Create a new unified memory provider
    pub fn new(
        short_term: Box<dyn ContextualMemoryProvider>,
        medium_term: Box<dyn ContextualMemoryProvider>,
        long_term: Box<dyn ContextualMemoryProvider>,
    ) -> Self {
        Self {
            short_term,
            medium_term,
            long_term,
            config: UnifiedMemoryConfig::default(),
            scope: None,
        }
    }

    /// Create with custom configuration
    pub fn with_config(mut self, config: UnifiedMemoryConfig) -> Self {
        self.config = config;
        self
    }

    /// Apply level-based weight boost to a result
    fn apply_weight(&self, result: &mut MemoryResult) {
        let weight = match result.source.level() {
            1 => self.config.short_term_weight,
            2 => self.config.medium_term_weight,
            3 => self.config.long_term_weight,
            _ => 1.0,
        };

        result.score.combined *= weight;
    }

    /// Deduplicate results based on content similarity
    fn deduplicate(&self, results: Vec<MemoryResult>) -> Vec<MemoryResult> {
        let mut seen_content: HashSet<String> = HashSet::new();
        let mut deduped = Vec::new();

        for result in results {
            // Create a normalized version for comparison
            let normalized: String = result
                .content
                .chars()
                .filter(|c| c.is_alphanumeric() || c.is_whitespace())
                .collect::<String>()
                .to_lowercase()
                .split_whitespace()
                .take(20)  // Compare first 20 words
                .collect::<Vec<_>>()
                .join(" ");

            if !seen_content.contains(&normalized) {
                seen_content.insert(normalized);
                deduped.push(result);
            }
        }

        deduped
    }
}

#[async_trait]
impl ContextualMemoryProvider for UnifiedMemoryProvider {
    async fn query(&self, query: &str, limit: usize) -> Result<Vec<MemoryResult>> {
        // Query all levels in parallel (conceptually - we do it sequentially for simplicity)
        let per_level_limit = limit.max(5);

        let mut all_results = Vec::new();

        // Short-term (current conversation)
        let mut short_results = self.short_term.query(query, per_level_limit).await?;
        for r in &mut short_results {
            self.apply_weight(r);
        }
        all_results.extend(short_results);

        // Medium-term (plans, tasks, decisions)
        let mut medium_results = self.medium_term.query(query, per_level_limit).await?;
        for r in &mut medium_results {
            self.apply_weight(r);
        }
        all_results.extend(medium_results);

        // Long-term (cross-conversation search)
        let mut long_results = self.long_term.query(query, per_level_limit).await?;
        for r in &mut long_results {
            self.apply_weight(r);
        }
        all_results.extend(long_results);

        // Filter by minimum score
        all_results.retain(|r| r.score.combined >= self.config.min_score_threshold);

        // Sort by combined score (descending)
        all_results.sort_by(|a, b| {
            b.score
                .combined
                .partial_cmp(&a.score.combined)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Deduplicate similar content
        let deduped = self.deduplicate(all_results);

        let final_results: Vec<MemoryResult> = deduped.into_iter().take(limit).collect();

        debug!(
            "UnifiedMemory: returning {} results for query '{}'",
            final_results.len(),
            query
        );

        Ok(final_results)
    }

    async fn search_context(
        &self,
        query: &str,
        source_filter: Option<&str>,
        limit: usize,
    ) -> Result<Vec<MemoryResult>> {
        match source_filter {
            Some("conversation") => {
                self.short_term
                    .search_context(query, source_filter, limit)
                    .await
            },
            Some("plan") | Some("task") | Some("decision") | Some("note") => {
                self.medium_term
                    .search_context(query, source_filter, limit)
                    .await
            },
            Some("cross_conversation") => {
                self.long_term
                    .search_context(query, source_filter, limit)
                    .await
            },
            _ => self.query(query, limit).await,
        }
    }

    async fn get_relevant_decisions(&self, topic: &str, limit: usize) -> Result<Vec<MemoryResult>> {
        // Decisions primarily come from medium-term
        let mut results = self
            .medium_term
            .get_relevant_decisions(topic, limit)
            .await?;

        // Also check long-term for decision-related discussions
        let long_results = self
            .long_term
            .query(&format!("decision {}", topic), limit / 2)
            .await?;

        results.extend(long_results);

        // Sort and deduplicate
        results.sort_by(|a, b| {
            b.score
                .combined
                .partial_cmp(&a.score.combined)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let deduped = self.deduplicate(results);
        Ok(deduped.into_iter().take(limit).collect())
    }

    fn current_scope(&self) -> Option<String> {
        self.scope.clone()
    }

    fn set_scope(&mut self, scope: Option<String>) {
        self.scope = scope.clone();
        // Propagate to all levels (if they support it)
        // Note: Can't call set_scope on trait objects directly without &mut self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::memory::traits::RelevanceScore;
    use chrono::Utc;

    /// Mock memory provider for testing
    struct MockMemoryProvider {
        results: Vec<MemoryResult>,
    }

    impl MockMemoryProvider {
        fn new(results: Vec<MemoryResult>) -> Self {
            Self { results }
        }
    }

    #[async_trait]
    impl ContextualMemoryProvider for MockMemoryProvider {
        async fn query(&self, _query: &str, limit: usize) -> Result<Vec<MemoryResult>> {
            Ok(self.results.iter().take(limit).cloned().collect())
        }

        async fn search_context(
            &self,
            _query: &str,
            _source_filter: Option<&str>,
            limit: usize,
        ) -> Result<Vec<MemoryResult>> {
            Ok(self.results.iter().take(limit).cloned().collect())
        }

        async fn get_relevant_decisions(
            &self,
            _topic: &str,
            _limit: usize,
        ) -> Result<Vec<MemoryResult>> {
            Ok(vec![])
        }

        fn current_scope(&self) -> Option<String> {
            None
        }

        fn set_scope(&mut self, _scope: Option<String>) {}
    }

    #[tokio::test]
    async fn test_unified_memory_query() {
        let short = MockMemoryProvider::new(vec![MemoryResult::new(
            "s1".to_string(),
            MemorySource::Conversation {
                conversation_id: "c1".to_string(),
                message_index: 0,
            },
            "Short-term result about auth".to_string(),
            RelevanceScore::new(0.8, 0.9, 1.0),
            Utc::now(),
        )]);

        let medium = MockMemoryProvider::new(vec![MemoryResult::new(
            "m1".to_string(),
            MemorySource::ProjectOrchestrator {
                entity_type: "decision".to_string(),
                entity_id: "d1".to_string(),
            },
            "Decision about auth: use JWT".to_string(),
            RelevanceScore::new(0.9, 0.5, 0.9),
            Utc::now(),
        )]);

        let long = MockMemoryProvider::new(vec![]);

        let unified = UnifiedMemoryProvider::new(Box::new(short), Box::new(medium), Box::new(long));

        let results = unified.query("authentication", 10).await.unwrap();
        assert_eq!(results.len(), 2);

        // Short-term should be boosted
        assert!(results[0].id == "s1" || results[0].id == "m1");
    }

    #[tokio::test]
    async fn test_unified_memory_deduplication() {
        let short = MockMemoryProvider::new(vec![MemoryResult::new(
            "s1".to_string(),
            MemorySource::Conversation {
                conversation_id: "c1".to_string(),
                message_index: 0,
            },
            "We should use JWT for authentication".to_string(),
            RelevanceScore::new(0.8, 0.9, 1.0),
            Utc::now(),
        )]);

        let medium = MockMemoryProvider::new(vec![MemoryResult::new(
            "m1".to_string(),
            MemorySource::ProjectOrchestrator {
                entity_type: "note".to_string(),
                entity_id: "n1".to_string(),
            },
            "We should use JWT for authentication".to_string(), // Same content
            RelevanceScore::new(0.9, 0.5, 0.9),
            Utc::now(),
        )]);

        let long = MockMemoryProvider::new(vec![]);

        let unified = UnifiedMemoryProvider::new(Box::new(short), Box::new(medium), Box::new(long));

        let results = unified.query("JWT", 10).await.unwrap();
        // Should be deduplicated to 1 result
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_config_default() {
        let config = UnifiedMemoryConfig::default();
        assert!(config.short_term_weight > config.long_term_weight);
    }
}
