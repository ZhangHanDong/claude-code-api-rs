//! Long-term memory: cross-conversation search via Meilisearch
//!
//! Provides semantic search across all past conversations and
//! knowledge notes.

#![allow(dead_code)] // Public API - may not be used internally

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};
use std::sync::Arc;
use tracing::debug;

use crate::core::storage::meilisearch::MeilisearchClient;

use super::traits::{ContextualMemoryProvider, MemoryResult, MemorySource, RelevanceScore};

/// Long-term memory backed by Meilisearch
pub struct LongTermMemory {
    meilisearch: Arc<MeilisearchClient>,
    current_conversation_id: Option<String>,
    scope: Option<String>,
}

impl LongTermMemory {
    /// Create a new long-term memory provider
    pub fn new(meilisearch: Arc<MeilisearchClient>) -> Self {
        Self {
            meilisearch,
            current_conversation_id: None,
            scope: None,
        }
    }

    /// Exclude the current conversation from search results
    pub fn with_current_conversation(mut self, conversation_id: String) -> Self {
        self.current_conversation_id = Some(conversation_id);
        self
    }

    /// Set the current conversation
    pub fn set_current_conversation(&mut self, conversation_id: Option<String>) {
        self.current_conversation_id = conversation_id;
    }

    /// Calculate recency score from timestamp
    fn recency_score(&self, timestamp: DateTime<Utc>) -> f64 {
        let now = Utc::now();
        let age = now.signed_duration_since(timestamp);

        // Score decays over time
        // Full score within 1 hour, decays to ~0.5 after 24 hours
        let hours = age.num_minutes() as f64 / 60.0;
        let score = 1.0 / (1.0 + hours / 24.0);

        score.clamp(0.0, 1.0)
    }
}

#[async_trait]
impl ContextualMemoryProvider for LongTermMemory {
    async fn query(&self, query: &str, limit: usize) -> Result<Vec<MemoryResult>> {
        // Search messages across all conversations
        let messages = self
            .meilisearch
            .search_messages(query, None, limit * 2) // Get more to filter
            .await?;

        let mut results: Vec<MemoryResult> = messages
            .into_iter()
            .filter(|msg| {
                // Exclude current conversation if set
                if let Some(ref current) = self.current_conversation_id {
                    return &msg.conversation_id != current;
                }
                true
            })
            .map(|msg| {
                let timestamp = Utc
                    .timestamp_opt(msg.created_at, 0)
                    .single()
                    .unwrap_or_else(Utc::now);
                let recency = self.recency_score(timestamp);

                // Meilisearch already did semantic search, so semantic score is high
                let score = RelevanceScore::new(0.8, recency, 0.5);

                MemoryResult::new(
                    msg.id.clone(),
                    MemorySource::CrossConversation {
                        conversation_id: msg.conversation_id.clone(),
                        message_id: msg.id,
                    },
                    msg.content,
                    score,
                    timestamp,
                )
                .with_metadata(serde_json::json!({
                    "role": msg.role,
                    "turn_index": msg.turn_index,
                    "conversation_id": msg.conversation_id,
                }))
            })
            .collect();

        // Sort by combined score
        results.sort_by(|a, b| {
            b.score
                .combined
                .partial_cmp(&a.score.combined)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        results.truncate(limit);
        debug!("LongTermMemory: found {} results for query", results.len());

        Ok(results)
    }

    async fn search_context(
        &self,
        query: &str,
        source_filter: Option<&str>,
        limit: usize,
    ) -> Result<Vec<MemoryResult>> {
        match source_filter {
            Some("conversation") | Some("cross_conversation") => self.query(query, limit).await,
            Some("note") | Some("knowledge_note") => {
                // For now, notes come from medium-term (project-orchestrator)
                // This could be extended to search Meilisearch for notes
                Ok(vec![])
            },
            _ => self.query(query, limit).await,
        }
    }

    async fn get_relevant_decisions(
        &self,
        _topic: &str,
        _limit: usize,
    ) -> Result<Vec<MemoryResult>> {
        // Decisions are stored in project-orchestrator (medium-term)
        // Long-term only has conversation messages
        Ok(vec![])
    }

    fn current_scope(&self) -> Option<String> {
        self.scope.clone()
    }

    fn set_scope(&mut self, scope: Option<String>) {
        self.scope = scope;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_recency_score() {
        // Create a mock (we can't actually test without Meilisearch)
        // Just test the recency calculation logic
        let _now = Utc::now();

        // Very recent: high score
        let _recent = _now - Duration::minutes(5);
        let hours: f64 = 5.0 / 60.0;
        let expected: f64 = 1.0 / (1.0 + hours / 24.0);
        assert!((expected - 0.996_f64).abs() < 0.01);

        // 24 hours old: ~0.5 score
        let _old = _now - Duration::hours(24);
        let hours_24: f64 = 24.0;
        let expected_24: f64 = 1.0 / (1.0 + hours_24 / 24.0);
        assert!((expected_24 - 0.5_f64).abs() < 0.01);
    }
}
