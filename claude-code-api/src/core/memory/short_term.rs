//! Short-term memory: current conversation
//!
//! Wraps ConversationStore to provide access to recent messages
//! in the current conversation.

#![allow(dead_code)] // Public API - may not be used internally

use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use tracing::debug;

use crate::core::storage::ConversationStore;
use crate::models::openai::MessageContent;

use super::traits::{ContextualMemoryProvider, MemoryResult, MemorySource, RelevanceScore};

/// Short-term memory backed by ConversationStore
pub struct ShortTermMemory<S: ConversationStore> {
    store: Arc<S>,
    conversation_id: Option<String>,
    scope: Option<String>,
}

impl<S: ConversationStore> ShortTermMemory<S> {
    /// Create a new short-term memory provider
    pub fn new(store: Arc<S>) -> Self {
        Self {
            store,
            conversation_id: None,
            scope: None,
        }
    }

    /// Set the current conversation ID
    pub fn with_conversation(mut self, conversation_id: String) -> Self {
        self.conversation_id = Some(conversation_id);
        self
    }

    /// Set the conversation ID
    pub fn set_conversation(&mut self, conversation_id: Option<String>) {
        self.conversation_id = conversation_id;
    }

    /// Calculate recency score based on message index
    fn recency_score(&self, message_index: usize, total_messages: usize) -> f64 {
        if total_messages == 0 {
            return 0.0;
        }
        // More recent messages get higher scores
        (message_index as f64 + 1.0) / total_messages as f64
    }

    /// Simple keyword matching for semantic score (placeholder for real semantic search)
    fn keyword_match_score(&self, query: &str, content: &str) -> f64 {
        let query_lower = query.to_lowercase();
        let query_words: Vec<&str> = query_lower.split_whitespace().collect();
        let content_lower = content.to_lowercase();

        if query_words.is_empty() {
            return 0.0;
        }

        let matches = query_words
            .iter()
            .filter(|word| content_lower.contains(*word))
            .count();

        matches as f64 / query_words.len() as f64
    }
}

#[async_trait]
impl<S: ConversationStore + 'static> ContextualMemoryProvider for ShortTermMemory<S> {
    async fn query(&self, query: &str, limit: usize) -> Result<Vec<MemoryResult>> {
        let Some(ref conv_id) = self.conversation_id else {
            return Ok(vec![]);
        };

        let Some(conversation) = self.store.get(conv_id).await? else {
            return Ok(vec![]);
        };

        let total_messages = conversation.messages.len();
        let mut results: Vec<MemoryResult> = conversation
            .messages
            .iter()
            .enumerate()
            .filter_map(|(idx, msg)| {
                let content = match &msg.content {
                    Some(MessageContent::Text(text)) => text.clone(),
                    Some(MessageContent::Array(parts)) => parts
                        .iter()
                        .filter_map(|p| match p {
                            crate::models::openai::ContentPart::Text { text } => Some(text.clone()),
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                        .join(" "),
                    None => return None,
                };

                let semantic = self.keyword_match_score(query, &content);
                let recency = self.recency_score(idx, total_messages);

                // Only include if there's some relevance
                if semantic < 0.1 {
                    return None;
                }

                let score = RelevanceScore::new(semantic, recency, 1.0); // Max scope for current conv

                Some(
                    MemoryResult::new(
                        format!("{}-{}", conv_id, idx),
                        MemorySource::Conversation {
                            conversation_id: conv_id.clone(),
                            message_index: idx,
                        },
                        content,
                        score,
                        conversation.updated_at,
                    )
                    .with_metadata(serde_json::json!({
                        "role": msg.role,
                        "turn_index": idx,
                    })),
                )
            })
            .collect();

        // Sort by combined score descending
        results.sort_by(|a, b| {
            b.score
                .combined
                .partial_cmp(&a.score.combined)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        results.truncate(limit);
        debug!("ShortTermMemory: found {} results for query", results.len());

        Ok(results)
    }

    async fn search_context(
        &self,
        query: &str,
        source_filter: Option<&str>,
        limit: usize,
    ) -> Result<Vec<MemoryResult>> {
        // Short-term only has conversation source
        if let Some(filter) = source_filter
            && filter != "conversation"
        {
            return Ok(vec![]);
        }

        self.query(query, limit).await
    }

    async fn get_relevant_decisions(
        &self,
        _topic: &str,
        _limit: usize,
    ) -> Result<Vec<MemoryResult>> {
        // Short-term memory doesn't track decisions
        // Decisions come from medium-term (project-orchestrator)
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
    use crate::core::storage::InMemoryConversationStore;
    use crate::models::openai::ChatMessage;

    #[tokio::test]
    async fn test_short_term_query() {
        let store: Arc<InMemoryConversationStore> = Arc::new(InMemoryConversationStore::default());

        // Create conversation with messages
        let conv_id = store.create(None).await.unwrap();
        store
            .add_message(
                &conv_id,
                ChatMessage {
                    role: "user".to_string(),
                    content: Some(MessageContent::Text(
                        "How should we implement authentication?".to_string(),
                    )),
                    name: None,
                    tool_calls: None,
                },
            )
            .await
            .unwrap();
        store
            .add_message(
                &conv_id,
                ChatMessage {
                    role: "assistant".to_string(),
                    content: Some(MessageContent::Text(
                        "I recommend using JWT tokens for authentication.".to_string(),
                    )),
                    name: None,
                    tool_calls: None,
                },
            )
            .await
            .unwrap();

        let memory = ShortTermMemory::new(store).with_conversation(conv_id);

        let results = memory.query("authentication", 10).await.unwrap();
        assert!(!results.is_empty());
        assert!(
            results[0].content.contains("authentication") || results[0].content.contains("JWT")
        );
    }

    #[tokio::test]
    async fn test_short_term_no_conversation() {
        let store: Arc<InMemoryConversationStore> = Arc::new(InMemoryConversationStore::default());
        let memory = ShortTermMemory::new(store);

        let results = memory.query("anything", 10).await.unwrap();
        assert!(results.is_empty());
    }
}
