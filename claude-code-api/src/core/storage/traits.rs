//! Storage trait definitions
//!
//! These traits define the interface for storage backends.
//! Implementations can be in-memory, Neo4j-backed, or any other storage system.

#![allow(dead_code)] // Public API - may not be used internally

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::core::cache::CacheStats;
use crate::core::conversation::{Conversation, ConversationMetadata};
use crate::core::session_manager::Session;
use crate::models::openai::{ChatCompletionResponse, ChatMessage};

/// Trait for conversation storage backends
///
/// Implementations must be thread-safe (Send + Sync) as they will be
/// shared across multiple async tasks.
#[async_trait]
pub trait ConversationStore: Send + Sync {
    /// Create a new conversation and return its ID
    async fn create(&self, model: Option<String>) -> Result<String>;

    /// Get a conversation by ID
    async fn get(&self, id: &str) -> Result<Option<Conversation>>;

    /// Add a message to a conversation
    async fn add_message(&self, id: &str, message: ChatMessage) -> Result<()>;

    /// Update conversation metadata directly
    async fn update_metadata(&self, id: &str, metadata: ConversationMetadata) -> Result<()>;

    /// List all active conversations with their last update time
    async fn list_active(&self) -> Result<Vec<(String, DateTime<Utc>)>>;

    /// Remove expired conversations older than the given duration
    async fn cleanup_expired(&self, timeout_minutes: i64) -> Result<usize>;

    /// Delete a specific conversation
    async fn delete(&self, id: &str) -> Result<bool>;
}

/// Trait for session storage backends
#[async_trait]
pub trait SessionStore: Send + Sync {
    /// Create a new session and return its ID
    async fn create(&self, project_path: Option<String>) -> Result<String>;

    /// Get a session by ID
    async fn get(&self, id: &str) -> Result<Option<Session>>;

    /// Update a session's last activity timestamp
    async fn update(&self, id: &str) -> Result<()>;

    /// Remove a session by ID
    async fn remove(&self, id: &str) -> Result<Option<Session>>;

    /// List all sessions
    async fn list(&self) -> Result<Vec<Session>>;
}

/// Trait for response cache storage backends
#[async_trait]
pub trait CacheStore: Send + Sync {
    /// Get a cached response by key
    async fn get(&self, key: &str) -> Option<ChatCompletionResponse>;

    /// Store a response in the cache
    async fn put(&self, key: String, response: ChatCompletionResponse);

    /// Get cache statistics
    async fn stats(&self) -> CacheStats;

    /// Clean up expired entries
    async fn cleanup(&self) -> Result<usize>;
}
