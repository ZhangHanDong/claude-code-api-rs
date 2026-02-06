//! In-memory storage implementations
//!
//! These implementations store data in memory using thread-safe data structures.
//! Data is lost when the process exits.

#![allow(dead_code)] // Public API - may not be used internally

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tracing::{debug, info};
use uuid::Uuid;

use crate::core::cache::CacheStats;
use crate::core::conversation::{Conversation, ConversationMetadata};
use crate::core::session_manager::Session;
use crate::models::openai::{ChatCompletionResponse, ChatMessage};

use super::traits::{CacheStore, ConversationStore, SessionStore};

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
                info!(
                    "Trimmed {} old messages from conversation {}",
                    remove_count, id
                );
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

// ============================================================================
// InMemorySessionStore
// ============================================================================

/// In-memory implementation of SessionStore
pub struct InMemorySessionStore {
    sessions: RwLock<HashMap<String, Session>>,
}

impl InMemorySessionStore {
    pub fn new() -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for InMemorySessionStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SessionStore for InMemorySessionStore {
    async fn create(&self, project_path: Option<String>) -> Result<String> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let session = Session {
            id: id.clone(),
            project_path,
            created_at: now,
            updated_at: now,
        };

        self.sessions.write().insert(id.clone(), session);
        info!("Created new session: {}", id);

        Ok(id)
    }

    async fn get(&self, id: &str) -> Result<Option<Session>> {
        Ok(self.sessions.read().get(id).cloned())
    }

    async fn update(&self, id: &str) -> Result<()> {
        if let Some(session) = self.sessions.write().get_mut(id) {
            session.updated_at = Utc::now();
            Ok(())
        } else {
            Err(anyhow::anyhow!("Session not found: {}", id))
        }
    }

    async fn remove(&self, id: &str) -> Result<Option<Session>> {
        Ok(self.sessions.write().remove(id))
    }

    async fn list(&self) -> Result<Vec<Session>> {
        Ok(self.sessions.read().values().cloned().collect())
    }
}

// ============================================================================
// InMemoryCacheStore
// ============================================================================

/// Cache entry with metadata
#[derive(Clone)]
struct CacheEntry {
    response: ChatCompletionResponse,
    created_at: Instant,
    hit_count: usize,
}

/// Configuration for in-memory cache
#[derive(Clone)]
pub struct InMemoryCacheConfig {
    pub max_entries: usize,
    pub ttl_seconds: u64,
    pub enabled: bool,
}

impl Default for InMemoryCacheConfig {
    fn default() -> Self {
        Self {
            max_entries: 1000,
            ttl_seconds: 3600,
            enabled: true,
        }
    }
}

/// In-memory implementation of CacheStore using DashMap
pub struct InMemoryCacheStore {
    cache: DashMap<String, CacheEntry>,
    config: InMemoryCacheConfig,
}

impl InMemoryCacheStore {
    pub fn new(config: InMemoryCacheConfig) -> Self {
        Self {
            cache: DashMap::new(),
            config,
        }
    }

    fn evict_oldest(&self) {
        let mut oldest_key = None;
        let mut oldest_time = Instant::now();

        for entry in self.cache.iter() {
            if entry.value().created_at < oldest_time {
                oldest_time = entry.value().created_at;
                oldest_key = Some(entry.key().clone());
            }
        }

        if let Some(key) = oldest_key {
            self.cache.remove(&key);
            debug!("Evicted oldest cache entry: {}", key);
        }
    }
}

impl Default for InMemoryCacheStore {
    fn default() -> Self {
        Self::new(InMemoryCacheConfig::default())
    }
}

#[async_trait]
impl CacheStore for InMemoryCacheStore {
    async fn get(&self, key: &str) -> Option<ChatCompletionResponse> {
        if !self.config.enabled {
            return None;
        }

        let mut entry = self.cache.get_mut(key)?;

        // Check if expired
        if entry.created_at.elapsed() > Duration::from_secs(self.config.ttl_seconds) {
            drop(entry);
            self.cache.remove(key);
            debug!("Cache entry expired: {}", key);
            return None;
        }

        entry.hit_count += 1;
        let hit_count = entry.hit_count;
        let response = entry.response.clone();

        info!("Cache hit for key: {} (hits: {})", key, hit_count);
        Some(response)
    }

    async fn put(&self, key: String, response: ChatCompletionResponse) {
        if !self.config.enabled {
            return;
        }

        if self.cache.len() >= self.config.max_entries {
            self.evict_oldest();
        }

        let entry = CacheEntry {
            response,
            created_at: Instant::now(),
            hit_count: 0,
        };

        self.cache.insert(key.clone(), entry);
        debug!("Cached response for key: {}", key);
    }

    async fn stats(&self) -> CacheStats {
        let mut total_hits = 0;
        let mut total_entries = 0;

        for entry in self.cache.iter() {
            total_entries += 1;
            total_hits += entry.value().hit_count;
        }

        CacheStats {
            total_entries,
            total_hits,
            enabled: self.config.enabled,
        }
    }

    async fn cleanup(&self) -> Result<usize> {
        let ttl = Duration::from_secs(self.config.ttl_seconds);
        let mut expired_keys = Vec::new();

        for entry in self.cache.iter() {
            if entry.value().created_at.elapsed() > ttl {
                expired_keys.push(entry.key().clone());
            }
        }

        let count = expired_keys.len();
        for key in expired_keys {
            self.cache.remove(&key);
            debug!("Removed expired cache entry: {}", key);
        }

        info!(
            "Cache cleanup: removed {} entries, {} remaining",
            count,
            self.cache.len()
        );
        Ok(count)
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
            content: Some(crate::models::openai::MessageContent::Text(
                "Hello".to_string(),
            )),
            name: None,
            tool_calls: None,
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
            content: Some(crate::models::openai::MessageContent::Text(
                "Hello".to_string(),
            )),
            name: None,
            tool_calls: None,
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

    // ========================================================================
    // SessionStore tests
    // ========================================================================

    #[tokio::test]
    async fn test_session_create_and_get() {
        let store = InMemorySessionStore::default();
        let id = store
            .create(Some("/path/to/project".to_string()))
            .await
            .unwrap();

        assert!(!id.is_empty());

        let session = store.get(&id).await.unwrap();
        assert!(session.is_some());

        let session = session.unwrap();
        assert_eq!(session.project_path, Some("/path/to/project".to_string()));
    }

    #[tokio::test]
    async fn test_session_list() {
        let store = InMemorySessionStore::default();

        store.create(None).await.unwrap();
        store.create(Some("/path".to_string())).await.unwrap();

        let sessions = store.list().await.unwrap();
        assert_eq!(sessions.len(), 2);
    }

    #[tokio::test]
    async fn test_session_remove() {
        let store = InMemorySessionStore::default();
        let id = store.create(None).await.unwrap();

        let removed = store.remove(&id).await.unwrap();
        assert!(removed.is_some());

        let session = store.get(&id).await.unwrap();
        assert!(session.is_none());
    }

    // ========================================================================
    // CacheStore tests
    // ========================================================================

    #[tokio::test]
    async fn test_cache_put_and_get() {
        let store = InMemoryCacheStore::default();

        let response = crate::models::openai::ChatCompletionResponse {
            id: "test-id".to_string(),
            object: "chat.completion".to_string(),
            created: 0,
            model: "test-model".to_string(),
            choices: vec![],
            usage: crate::models::openai::Usage {
                prompt_tokens: 0,
                completion_tokens: 0,
                total_tokens: 0,
            },
            conversation_id: None,
        };

        store.put("test-key".to_string(), response.clone()).await;

        let cached = store.get("test-key").await;
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().id, "test-id");
    }

    #[tokio::test]
    async fn test_cache_stats() {
        let store = InMemoryCacheStore::default();

        let response = crate::models::openai::ChatCompletionResponse {
            id: "test".to_string(),
            object: "chat.completion".to_string(),
            created: 0,
            model: "test".to_string(),
            choices: vec![],
            usage: crate::models::openai::Usage {
                prompt_tokens: 0,
                completion_tokens: 0,
                total_tokens: 0,
            },
            conversation_id: None,
        };

        store.put("key1".to_string(), response.clone()).await;
        store.put("key2".to_string(), response).await;

        let stats = store.stats().await;
        assert_eq!(stats.total_entries, 2);
        assert!(stats.enabled);
    }

    #[tokio::test]
    async fn test_cache_disabled() {
        let config = InMemoryCacheConfig {
            enabled: false,
            ..Default::default()
        };
        let store = InMemoryCacheStore::new(config);

        let response = crate::models::openai::ChatCompletionResponse {
            id: "test".to_string(),
            object: "chat.completion".to_string(),
            created: 0,
            model: "test".to_string(),
            choices: vec![],
            usage: crate::models::openai::Usage {
                prompt_tokens: 0,
                completion_tokens: 0,
                total_tokens: 0,
            },
            conversation_id: None,
        };

        store.put("key".to_string(), response).await;

        let cached = store.get("key").await;
        assert!(cached.is_none());
    }
}
