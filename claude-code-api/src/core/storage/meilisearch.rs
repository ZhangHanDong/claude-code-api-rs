//! Meilisearch integration for semantic search
//!
//! This module provides Meilisearch-backed search capabilities for conversations.
//! Index names are prefixed with "nexus_" to avoid conflicts with other applications.

#![allow(dead_code)] // Public API - may not be used internally
//!
//! ## Indexes
//!
//! - `nexus_messages`: Full-text search on message content
//!   - Searchable: content, role
//!   - Filterable: conversation_id, role, created_at
//!   - Sortable: created_at

use anyhow::Result;
use meilisearch_sdk::client::Client;
use meilisearch_sdk::indexes::Index;
use meilisearch_sdk::settings::Settings;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

/// Meilisearch index names (prefixed to avoid conflicts)
pub const INDEX_MESSAGES: &str = "nexus_messages";
pub const INDEX_CONVERSATIONS: &str = "nexus_conversations";

/// Configuration for Meilisearch connection
#[derive(Clone, Debug)]
pub struct MeilisearchConfig {
    pub url: String,
    pub api_key: Option<String>,
}

impl Default for MeilisearchConfig {
    fn default() -> Self {
        Self {
            url: std::env::var("MEILISEARCH_URL")
                .unwrap_or_else(|_| "http://localhost:7700".to_string()),
            api_key: std::env::var("MEILISEARCH_KEY").ok(),
        }
    }
}

/// Document structure for indexed messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageDocument {
    pub id: String,
    pub conversation_id: String,
    pub role: String,
    pub content: String,
    pub turn_index: usize,
    pub created_at: i64, // Unix timestamp for sorting
}

/// Document structure for indexed conversations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationDocument {
    pub id: String,
    pub model: Option<String>,
    pub message_count: usize,
    pub total_tokens: usize,
    pub created_at: i64,
    pub updated_at: i64,
    /// Concatenated preview of conversation content for search
    pub content_preview: String,
}

/// Meilisearch client wrapper for Nexus
#[derive(Clone)]
pub struct MeilisearchClient {
    client: Client,
}

impl MeilisearchClient {
    /// Create a new Meilisearch client
    pub async fn new(config: MeilisearchConfig) -> Result<Self> {
        info!("Connecting to Meilisearch at {}", config.url);

        let client = Client::new(&config.url, config.api_key.as_deref())?;

        let ms = Self { client };

        // Initialize indexes
        ms.init_indexes().await?;

        info!("Connected to Meilisearch successfully");
        Ok(ms)
    }

    /// Initialize Meilisearch indexes with proper settings
    async fn init_indexes(&self) -> Result<()> {
        // Create messages index
        self.client
            .create_index(INDEX_MESSAGES, Some("id"))
            .await
            .ok(); // Ignore if exists

        let messages_index = self.client.index(INDEX_MESSAGES);
        let messages_settings = Settings::new()
            .with_searchable_attributes(["content", "role"])
            .with_filterable_attributes(["conversation_id", "role", "created_at"])
            .with_sortable_attributes(["created_at", "turn_index"]);

        messages_index.set_settings(&messages_settings).await?;

        // Create conversations index
        self.client
            .create_index(INDEX_CONVERSATIONS, Some("id"))
            .await
            .ok(); // Ignore if exists

        let conversations_index = self.client.index(INDEX_CONVERSATIONS);
        let conversations_settings = Settings::new()
            .with_searchable_attributes(["content_preview", "model"])
            .with_filterable_attributes(["model", "created_at", "updated_at"])
            .with_sortable_attributes(["created_at", "updated_at", "message_count"]);

        conversations_index
            .set_settings(&conversations_settings)
            .await?;

        info!("Meilisearch indexes initialized for Nexus");
        Ok(())
    }

    /// Get the messages index
    pub fn messages_index(&self) -> Index {
        self.client.index(INDEX_MESSAGES)
    }

    /// Get the conversations index
    pub fn conversations_index(&self) -> Index {
        self.client.index(INDEX_CONVERSATIONS)
    }

    /// Index a message for search
    pub async fn index_message(&self, doc: MessageDocument) -> Result<()> {
        let index = self.messages_index();
        index.add_documents(&[doc], Some("id")).await?;
        debug!("Indexed message");
        Ok(())
    }

    /// Index multiple messages
    pub async fn index_messages(&self, docs: Vec<MessageDocument>) -> Result<()> {
        if docs.is_empty() {
            return Ok(());
        }

        let index = self.messages_index();
        index.add_documents(&docs, Some("id")).await?;
        debug!("Indexed {} messages", docs.len());
        Ok(())
    }

    /// Index a conversation for search
    pub async fn index_conversation(&self, doc: ConversationDocument) -> Result<()> {
        let index = self.conversations_index();
        let id = doc.id.clone();
        index.add_documents(&[doc], Some("id")).await?;
        debug!("Indexed conversation {}", id);
        Ok(())
    }

    /// Search messages by content
    pub async fn search_messages(
        &self,
        query: &str,
        conversation_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<MessageDocument>> {
        let index = self.messages_index();

        let filter = conversation_id.map(|id| format!("conversation_id = \"{}\"", id));

        let mut search = index.search();
        search.with_query(query).with_limit(limit);

        if let Some(ref f) = filter {
            search.with_filter(f);
        }

        let results = search.execute::<MessageDocument>().await?;

        Ok(results.hits.into_iter().map(|h| h.result).collect())
    }

    /// Search conversations by content preview
    pub async fn search_conversations(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<ConversationDocument>> {
        let index = self.conversations_index();

        let results = index
            .search()
            .with_query(query)
            .with_limit(limit)
            .execute::<ConversationDocument>()
            .await?;

        Ok(results.hits.into_iter().map(|h| h.result).collect())
    }

    /// Delete a message from the index
    pub async fn delete_message(&self, message_id: &str) -> Result<()> {
        let index = self.messages_index();
        index.delete_document(message_id).await?;
        Ok(())
    }

    /// Delete all messages for a conversation
    pub async fn delete_conversation_messages(&self, conversation_id: &str) -> Result<()> {
        // Search for all messages in this conversation and delete them one by one
        // Note: Meilisearch SDK v0.27 doesn't have filter-based deletion
        let messages = self
            .search_messages("", Some(conversation_id), 1000)
            .await?;

        let index = self.messages_index();
        for msg in messages {
            let _ = index.delete_document(&msg.id).await;
        }

        Ok(())
    }

    /// Delete a conversation from the index
    pub async fn delete_conversation(&self, conversation_id: &str) -> Result<()> {
        let index = self.conversations_index();
        index.delete_document(conversation_id).await?;

        // Also delete all messages
        self.delete_conversation_messages(conversation_id).await?;

        Ok(())
    }

    /// Get index statistics
    pub async fn get_stats(&self) -> Result<MeilisearchStats> {
        let messages_stats = self.messages_index().get_stats().await?;
        let conversations_stats = self.conversations_index().get_stats().await?;

        Ok(MeilisearchStats {
            messages_count: messages_stats.number_of_documents,
            conversations_count: conversations_stats.number_of_documents,
            is_indexing: messages_stats.is_indexing || conversations_stats.is_indexing,
        })
    }
}

/// Statistics for Nexus Meilisearch indexes
#[derive(Debug, Clone, Serialize)]
pub struct MeilisearchStats {
    pub messages_count: usize,
    pub conversations_count: usize,
    pub is_indexing: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_meilisearch_connection() {
        let config = MeilisearchConfig::default();
        let client = MeilisearchClient::new(config).await.unwrap();

        let stats = client.get_stats().await.unwrap();
        println!("Stats: {:?}", stats);
    }

    #[tokio::test]
    #[ignore]
    async fn test_index_and_search_message() {
        let config = MeilisearchConfig::default();
        let client = MeilisearchClient::new(config).await.unwrap();

        let doc = MessageDocument {
            id: "test-msg-1".to_string(),
            conversation_id: "test-conv-1".to_string(),
            role: "user".to_string(),
            content: "Hello, how can I help you today?".to_string(),
            turn_index: 0,
            created_at: chrono::Utc::now().timestamp(),
        };

        client.index_message(doc).await.unwrap();

        // Wait for indexing
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        let results = client.search_messages("help", None, 10).await.unwrap();
        assert!(!results.is_empty());

        // Cleanup
        client.delete_message("test-msg-1").await.unwrap();
    }
}
