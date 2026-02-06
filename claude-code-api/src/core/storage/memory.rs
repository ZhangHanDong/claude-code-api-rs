//! In-memory storage implementations
//!
//! These implementations store data in memory using thread-safe data structures.
//! Data is lost when the process exits.

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use std::collections::HashMap;
use tracing::info;
use uuid::Uuid;

use crate::core::conversation::{Conversation, ConversationMetadata};
use crate::models::openai::ChatMessage;

use super::traits::ConversationStore;

/// Configuration for in-memory conversation storage
#[derive(Clone)]
pub struct InMemoryConversationConfig {
    pub max_history_messages: usize,
}

impl Default for InMemoryConversationConfig {
    fn default() -> Self {
        Self {
            max_history_messages: 20,
        }
    }
}

/// In-memory implementation of ConversationStore
///
/// Uses a HashMap protected by a RwLock for thread-safe access.
/// Suitable for development and single-instance deployments.
pub struct InMemoryConversationStore {
    conversations: RwLock<HashMap<String, Conversation>>,
    config: InMemoryConversationConfig,
}

impl InMemoryConversationStore {
    pub fn new(config: InMemoryConversationConfig) -> Self {
        Self {
            conversations: RwLock::new(HashMap::new()),
            config,
        }
    }
}

impl Default for InMemoryConversationStore {
    fn default() -> Self {
        Self::new(InMemoryConversationConfig::default())
    }
}

#[async_trait]
impl ConversationStore for InMemoryConversationStore {
    async fn create(&self, model: Option<String>) -> Result<String> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let conversation = Conversation {
            id: id.clone(),
            messages: Vec::new(),
            created_at: now,
            updated_at: now,
            metadata: ConversationMetadata {
                model,
                ..Default::default()
            },
        };

        self.conversations.write().insert(id.clone(), conversation);
        info!("Created new conversation: {}", id);

        Ok(id)
    }

    async fn get(&self, id: &str) -> Result<Option<Conversation>> {
        Ok(self.conversations.read().get(id).cloned())
    }

    async fn add_message(&self, id: &str, message: ChatMessage) -> Result<()> {
        let mut conversations = self.conversations.write();

        if let Some(conversation) = conversations.get_mut(id) {
            conversation.messages.push(message);
            conversation.updated_at = Utc::now();
            conversation.metadata.turn_count += 1;

            // Trim old messages if exceeding limit
            if conversation.messages.len() > self.config.max_history_messages {
                let remove_count = conversation.messages.len() - self.config.max_history_messages;
                conversation.messages.drain(0..remove_count);
                info!("Trimmed {} old messages from conversation {}", remove_count, id);
            }

            Ok(())
        } else {
            Err(anyhow::anyhow!("Conversation not found: {}", id))
        }
    }

    async fn update_metadata(&self, id: &str, metadata: ConversationMetadata) -> Result<()> {
        let mut conversations = self.conversations.write();

        if let Some(conversation) = conversations.get_mut(id) {
            conversation.metadata = metadata;
            conversation.updated_at = Utc::now();
            Ok(())
        } else {
            Err(anyhow::anyhow!("Conversation not found: {}", id))
        }
    }

    async fn list_active(&self) -> Result<Vec<(String, DateTime<Utc>)>> {
        let conversations = self.conversations.read();
        Ok(conversations
            .iter()
            .map(|(id, conv)| (id.clone(), conv.updated_at))
            .collect())
    }

    async fn cleanup_expired(&self, timeout_minutes: i64) -> Result<usize> {
        let timeout = chrono::Duration::minutes(timeout_minutes);
        let now = Utc::now();
        let mut expired = Vec::new();

        {
            let conversations = self.conversations.read();
            for (id, conv) in conversations.iter() {
                if now - conv.updated_at > timeout {
                    expired.push(id.clone());
                }
            }
        }

        let count = expired.len();
        if !expired.is_empty() {
            let mut conversations = self.conversations.write();
            for id in expired {
                conversations.remove(&id);
                info!("Removed expired conversation: {}", id);
            }
        }

        Ok(count)
    }

    async fn delete(&self, id: &str) -> Result<bool> {
        Ok(self.conversations.write().remove(id).is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_conversation() {
        let store = InMemoryConversationStore::default();
        let id = store.create(Some("claude-3".to_string())).await.unwrap();

        assert!(!id.is_empty());

        let conv = store.get(&id).await.unwrap();
        assert!(conv.is_some());

        let conv = conv.unwrap();
        assert_eq!(conv.metadata.model, Some("claude-3".to_string()));
        assert!(conv.messages.is_empty());
    }

    #[tokio::test]
    async fn test_add_message() {
        let store = InMemoryConversationStore::default();
        let id = store.create(None).await.unwrap();

        let message = ChatMessage {
            role: "user".to_string(),
            content: Some(crate::models::openai::MessageContent::Text("Hello".to_string())),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        };

        store.add_message(&id, message).await.unwrap();

        let conv = store.get(&id).await.unwrap().unwrap();
        assert_eq!(conv.messages.len(), 1);
        assert_eq!(conv.metadata.turn_count, 1);
    }

    #[tokio::test]
    async fn test_message_not_found() {
        let store = InMemoryConversationStore::default();

        let message = ChatMessage {
            role: "user".to_string(),
            content: Some(crate::models::openai::MessageContent::Text("Hello".to_string())),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        };

        let result = store.add_message("nonexistent", message).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_delete_conversation() {
        let store = InMemoryConversationStore::default();
        let id = store.create(None).await.unwrap();

        assert!(store.get(&id).await.unwrap().is_some());

        let deleted = store.delete(&id).await.unwrap();
        assert!(deleted);

        assert!(store.get(&id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_list_active() {
        let store = InMemoryConversationStore::default();

        let id1 = store.create(None).await.unwrap();
        let id2 = store.create(None).await.unwrap();

        let active = store.list_active().await.unwrap();
        assert_eq!(active.len(), 2);

        let ids: Vec<_> = active.iter().map(|(id, _)| id.clone()).collect();
        assert!(ids.contains(&id1));
        assert!(ids.contains(&id2));
    }
}
