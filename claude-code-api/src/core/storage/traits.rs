//! Storage trait definitions
//!
//! These traits define the interface for storage backends.
//! Implementations can be in-memory, Neo4j-backed, or any other storage system.

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::core::conversation::{Conversation, ConversationMetadata};
use crate::models::openai::ChatMessage;

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
