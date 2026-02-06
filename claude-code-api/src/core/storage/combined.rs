//! Combined Neo4j + Meilisearch storage
//!
//! This module provides a storage implementation that combines:
//! - Neo4j for persistent graph storage
//! - Meilisearch for full-text search indexing
//!
//! Messages are automatically indexed in Meilisearch when added to conversations.

#![allow(dead_code)] // Public API - may not be used internally

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::sync::Arc;
use tracing::{debug, warn};

use crate::core::conversation::{Conversation, ConversationMetadata};
use crate::models::openai::{ChatMessage, MessageContent};

use super::meilisearch::{ConversationDocument, MeilisearchClient, MessageDocument};
use super::neo4j::{Neo4jClient, Neo4jConversationStore, Neo4jSessionStore};
use super::traits::{ConversationStore, SessionStore};
use crate::core::session_manager::Session;

/// Combined conversation store with Neo4j + Meilisearch
///
/// Provides:
/// - Persistent storage in Neo4j
/// - Automatic indexing in Meilisearch for search
/// - Search capabilities across conversation history
pub struct CombinedConversationStore {
    neo4j_store: Neo4jConversationStore,
    meilisearch: Option<Arc<MeilisearchClient>>,
}

impl CombinedConversationStore {
    /// Create a new combined store with Neo4j and optional Meilisearch
    pub fn new(neo4j_client: Neo4jClient, meilisearch: Option<Arc<MeilisearchClient>>) -> Self {
        Self {
            neo4j_store: Neo4jConversationStore::new(neo4j_client),
            meilisearch,
        }
    }

    /// Search messages across all conversations
    pub async fn search_messages(&self, query: &str, limit: usize) -> Result<Vec<MessageDocument>> {
        match &self.meilisearch {
            Some(ms) => ms.search_messages(query, None, limit).await,
            None => Ok(vec![]),
        }
    }

    /// Search messages within a specific conversation
    pub async fn search_conversation_messages(
        &self,
        conversation_id: &str,
        query: &str,
        limit: usize,
    ) -> Result<Vec<MessageDocument>> {
        match &self.meilisearch {
            Some(ms) => {
                ms.search_messages(query, Some(conversation_id), limit)
                    .await
            },
            None => Ok(vec![]),
        }
    }

    /// Search conversations by content
    pub async fn search_conversations(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<ConversationDocument>> {
        match &self.meilisearch {
            Some(ms) => ms.search_conversations(query, limit).await,
            None => Ok(vec![]),
        }
    }

    /// Index a message in Meilisearch
    async fn index_message(&self, conversation_id: &str, message: &ChatMessage, turn_index: usize) {
        if let Some(ref ms) = self.meilisearch {
            let content = match &message.content {
                Some(MessageContent::Text(text)) => text.clone(),
                Some(MessageContent::Array(parts)) => parts
                    .iter()
                    .filter_map(|p| match p {
                        crate::models::openai::ContentPart::Text { text } => Some(text.clone()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n"),
                None => String::new(),
            };

            let doc = MessageDocument {
                id: format!("{}-{}", conversation_id, turn_index),
                conversation_id: conversation_id.to_string(),
                role: message.role.clone(),
                content,
                turn_index,
                created_at: Utc::now().timestamp(),
            };

            if let Err(e) = ms.index_message(doc).await {
                warn!("Failed to index message in Meilisearch: {}", e);
            } else {
                debug!("Indexed message in Meilisearch");
            }
        }
    }

    /// Update conversation index in Meilisearch
    async fn update_conversation_index(&self, conversation: &Conversation) {
        if let Some(ref ms) = self.meilisearch {
            // Create content preview from recent messages
            let content_preview: String = conversation
                .messages
                .iter()
                .rev()
                .take(5)
                .filter_map(|m| match &m.content {
                    Some(MessageContent::Text(text)) => Some(text.clone()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join(" ");

            let doc = ConversationDocument {
                id: conversation.id.clone(),
                model: conversation.metadata.model.clone(),
                message_count: conversation.messages.len(),
                total_tokens: conversation.metadata.total_tokens,
                created_at: conversation.created_at.timestamp(),
                updated_at: conversation.updated_at.timestamp(),
                content_preview: content_preview.chars().take(500).collect(),
            };

            if let Err(e) = ms.index_conversation(doc).await {
                warn!("Failed to index conversation in Meilisearch: {}", e);
            }
        }
    }
}

#[async_trait]
impl ConversationStore for CombinedConversationStore {
    async fn create(&self, model: Option<String>) -> Result<String> {
        let id = self.neo4j_store.create(model.clone()).await?;

        // Index the new conversation
        if let Some(ref ms) = self.meilisearch {
            let doc = ConversationDocument {
                id: id.clone(),
                model,
                message_count: 0,
                total_tokens: 0,
                created_at: Utc::now().timestamp(),
                updated_at: Utc::now().timestamp(),
                content_preview: String::new(),
            };

            if let Err(e) = ms.index_conversation(doc).await {
                warn!("Failed to index new conversation: {}", e);
            }
        }

        Ok(id)
    }

    async fn get(&self, id: &str) -> Result<Option<Conversation>> {
        self.neo4j_store.get(id).await
    }

    async fn add_message(&self, id: &str, message: ChatMessage) -> Result<()> {
        // Get current turn count for indexing
        let turn_index = if let Some(conv) = self.get(id).await? {
            conv.metadata.turn_count
        } else {
            0
        };

        // Add to Neo4j
        self.neo4j_store.add_message(id, message.clone()).await?;

        // Index in Meilisearch
        self.index_message(id, &message, turn_index).await;

        // Update conversation index
        if let Some(conversation) = self.get(id).await? {
            self.update_conversation_index(&conversation).await;
        }

        Ok(())
    }

    async fn update_metadata(&self, id: &str, metadata: ConversationMetadata) -> Result<()> {
        self.neo4j_store.update_metadata(id, metadata).await?;

        // Update conversation index
        if let Some(conversation) = self.get(id).await? {
            self.update_conversation_index(&conversation).await;
        }

        Ok(())
    }

    async fn list_active(&self) -> Result<Vec<(String, DateTime<Utc>)>> {
        self.neo4j_store.list_active().await
    }

    async fn cleanup_expired(&self, timeout_minutes: i64) -> Result<usize> {
        // Get list of expired conversations before cleanup
        let expired: Vec<_> = self
            .list_active()
            .await?
            .into_iter()
            .filter(|(_, updated_at)| {
                let timeout = chrono::Duration::minutes(timeout_minutes);
                Utc::now() - *updated_at > timeout
            })
            .map(|(id, _)| id)
            .collect();

        // Clean up from Meilisearch
        if let Some(ref ms) = self.meilisearch {
            for id in &expired {
                let _ = ms.delete_conversation(id).await;
            }
        }

        // Clean up from Neo4j
        self.neo4j_store.cleanup_expired(timeout_minutes).await
    }

    async fn delete(&self, id: &str) -> Result<bool> {
        // Delete from Meilisearch
        if let Some(ref ms) = self.meilisearch {
            let _ = ms.delete_conversation(id).await;
        }

        // Delete from Neo4j
        self.neo4j_store.delete(id).await
    }
}

/// Combined session store with Neo4j
///
/// Currently just wraps Neo4jSessionStore, but can be extended
/// to add caching or other features.
pub struct CombinedSessionStore {
    neo4j_store: Neo4jSessionStore,
}

impl CombinedSessionStore {
    pub fn new(neo4j_client: Neo4jClient) -> Self {
        Self {
            neo4j_store: Neo4jSessionStore::new(neo4j_client),
        }
    }
}

#[async_trait]
impl SessionStore for CombinedSessionStore {
    async fn create(&self, project_path: Option<String>) -> Result<String> {
        self.neo4j_store.create(project_path).await
    }

    async fn get(&self, id: &str) -> Result<Option<Session>> {
        self.neo4j_store.get(id).await
    }

    async fn update(&self, id: &str) -> Result<()> {
        self.neo4j_store.update(id).await
    }

    async fn remove(&self, id: &str) -> Result<Option<Session>> {
        self.neo4j_store.remove(id).await
    }

    async fn list(&self) -> Result<Vec<Session>> {
        self.neo4j_store.list().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::storage::{MeilisearchConfig, Neo4jConfig};

    #[tokio::test]
    #[ignore]
    async fn test_combined_store() {
        let neo4j_config = Neo4jConfig::default();
        let neo4j_client = Neo4jClient::new(neo4j_config).await.unwrap();

        let ms_config = MeilisearchConfig::default();
        let ms_client = MeilisearchClient::new(ms_config).await.unwrap();

        let store = CombinedConversationStore::new(neo4j_client, Some(Arc::new(ms_client)));

        // Create conversation
        let id = store.create(Some("claude-3".to_string())).await.unwrap();
        assert!(!id.is_empty());

        // Add message
        let message = ChatMessage {
            role: "user".to_string(),
            content: Some(MessageContent::Text("Hello, how are you?".to_string())),
            name: None,
            tool_calls: None,
        };
        store.add_message(&id, message).await.unwrap();

        // Wait for indexing
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        // Search
        let results = store.search_messages("hello", 10).await.unwrap();
        assert!(!results.is_empty());

        // Cleanup
        store.delete(&id).await.unwrap();
    }
}
