use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

use crate::core::storage::{ConversationStore, InMemoryConversationStore};
use crate::models::openai::{ChatMessage, MessageContent};

/// Type alias for the default ConversationManager using in-memory storage
pub type DefaultConversationManager = ConversationManager<InMemoryConversationStore>;

/// Configuration for the conversation manager
#[derive(Clone)]
pub struct ConversationConfig {
    pub max_context_tokens: usize,
    pub session_timeout_minutes: i64,
}

impl Default for ConversationConfig {
    fn default() -> Self {
        Self {
            max_context_tokens: 100000,
            session_timeout_minutes: 30,
        }
    }
}

/// A conversation with its messages and metadata
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Conversation {
    pub id: String,
    pub messages: Vec<ChatMessage>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub metadata: ConversationMetadata,
}

/// Metadata associated with a conversation
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ConversationMetadata {
    pub model: Option<String>,
    pub total_tokens: usize,
    pub turn_count: usize,
    pub project_path: Option<String>,
}

/// Manager for conversations that delegates storage to a ConversationStore implementation
#[derive(Clone)]
pub struct ConversationManager<S: ConversationStore> {
    store: Arc<S>,
    config: ConversationConfig,
}

impl<S: ConversationStore + 'static> ConversationManager<S> {
    /// Create a new ConversationManager with the given store and config
    pub fn new(store: S, config: ConversationConfig) -> Self {
        let manager = Self {
            store: Arc::new(store),
            config,
        };

        // Start cleanup task
        let store_clone = manager.store.clone();
        let timeout = manager.config.session_timeout_minutes;
        tokio::spawn(async move {
            Self::cleanup_loop(store_clone, timeout).await;
        });

        manager
    }

    /// Create a new conversation and return its ID
    pub async fn create_conversation(&self, model: Option<String>) -> Result<String> {
        self.store.create(model).await
    }

    /// Add a message to a conversation
    pub async fn add_message(&self, conversation_id: &str, message: ChatMessage) -> Result<()> {
        self.store.add_message(conversation_id, message).await
    }

    /// Get a conversation by ID
    pub async fn get_conversation(&self, conversation_id: &str) -> Option<Conversation> {
        self.store.get(conversation_id).await.ok().flatten()
    }

    /// Get context messages for a conversation, including new messages
    pub async fn get_context_messages(
        &self,
        conversation_id: &str,
        new_messages: &[ChatMessage],
    ) -> Vec<ChatMessage> {
        if let Some(conversation) = self.get_conversation(conversation_id).await {
            let mut context = conversation.messages;
            context.extend_from_slice(new_messages);
            self.trim_context(context)
        } else {
            new_messages.to_vec()
        }
    }

    /// Trim context to fit within token limits
    fn trim_context(&self, messages: Vec<ChatMessage>) -> Vec<ChatMessage> {
        let mut system_messages = Vec::new();
        let mut other_messages = Vec::new();

        for msg in messages {
            if msg.role == "system" {
                system_messages.push(msg);
            } else {
                other_messages.push(msg);
            }
        }

        // Estimate tokens (simplified: ~0.25 tokens per character)
        let estimate_tokens = |msgs: &[ChatMessage]| -> usize {
            msgs.iter()
                .map(|m| match &m.content {
                    Some(MessageContent::Text(text)) => text.len() / 4,
                    Some(MessageContent::Array(parts)) => parts.len() * 100,
                    None => 50,
                })
                .sum()
        };

        let mut result = system_messages;
        let mut token_count = estimate_tokens(&result);

        // Add messages from newest to oldest
        for msg in other_messages.into_iter().rev() {
            let msg_tokens = estimate_tokens(std::slice::from_ref(&msg));
            if token_count + msg_tokens > self.config.max_context_tokens {
                break;
            }
            result.push(msg);
            token_count += msg_tokens;
        }

        // Restore correct order
        if result.len() > 1 {
            let system_count = result.iter().filter(|m| m.role == "system").count();
            result[system_count..].reverse();
        }

        result
    }

    /// Update conversation metadata
    pub async fn update_metadata(
        &self,
        conversation_id: &str,
        update_fn: impl FnOnce(&mut ConversationMetadata),
    ) -> Result<()> {
        if let Some(mut conversation) = self.get_conversation(conversation_id).await {
            update_fn(&mut conversation.metadata);
            self.store
                .update_metadata(conversation_id, conversation.metadata)
                .await
        } else {
            Err(anyhow::anyhow!("Conversation not found"))
        }
    }

    /// List all active conversations with their last update time
    pub async fn list_active_conversations(&self) -> Vec<(String, DateTime<Utc>)> {
        self.store.list_active().await.unwrap_or_default()
    }

    /// Background cleanup loop
    async fn cleanup_loop(store: Arc<S>, timeout_minutes: i64) {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(300)).await;

            match store.cleanup_expired(timeout_minutes).await {
                Ok(count) if count > 0 => {
                    info!("Cleaned up {} expired conversations", count);
                },
                Err(e) => {
                    tracing::error!("Failed to cleanup expired conversations: {}", e);
                },
                _ => {},
            }
        }
    }
}
