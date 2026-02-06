//! Message and Conversation document types for Meilisearch storage.
//!
//! These types are designed to capture the implicit context of conversations
//! without requiring explicit project identification.

use serde::{Deserialize, Serialize};

/// A message stored in the memory system.
///
/// This document is indexed in Meilisearch for semantic search and
/// includes contextual metadata for relevance scoring.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MessageDocument {
    /// Unique identifier for this message
    pub id: String,

    /// ID of the conversation this message belongs to
    pub conversation_id: String,

    /// Role of the message sender ("user" or "assistant")
    pub role: String,

    /// The full message content
    pub content: String,

    /// Turn index within the conversation (0-based)
    pub turn_index: usize,

    /// Unix timestamp of message creation
    pub created_at: i64,

    /// Working directory when this message was created.
    /// Used for cwd-based relevance scoring.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,

    /// List of file paths touched during this message turn.
    /// Extracted from tool calls (Read, Write, Edit, Glob, Grep).
    #[serde(default)]
    pub files_touched: Vec<String>,

    /// Pre-computed summary for long messages.
    /// Generated asynchronously to avoid blocking conversations.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
}

impl MessageDocument {
    /// Creates a new MessageDocument with required fields.
    pub fn new(
        id: impl Into<String>,
        conversation_id: impl Into<String>,
        role: impl Into<String>,
        content: impl Into<String>,
        turn_index: usize,
        created_at: i64,
    ) -> Self {
        Self {
            id: id.into(),
            conversation_id: conversation_id.into(),
            role: role.into(),
            content: content.into(),
            turn_index,
            created_at,
            cwd: None,
            files_touched: Vec::new(),
            summary: None,
        }
    }

    /// Sets the working directory for this message.
    pub fn with_cwd(mut self, cwd: impl Into<String>) -> Self {
        self.cwd = Some(cwd.into());
        self
    }

    /// Sets the files touched during this message turn.
    pub fn with_files_touched(mut self, files: Vec<String>) -> Self {
        self.files_touched = files;
        self
    }

    /// Adds a file to the list of touched files.
    pub fn add_file_touched(&mut self, file: impl Into<String>) {
        let file = file.into();
        if !self.files_touched.contains(&file) {
            self.files_touched.push(file);
        }
    }

    /// Sets the summary for this message.
    pub fn with_summary(mut self, summary: impl Into<String>) -> Self {
        self.summary = Some(summary.into());
        self
    }

    /// Returns the content to use for context injection.
    /// Prefers summary over full content if available.
    pub fn display_content(&self) -> &str {
        self.summary.as_deref().unwrap_or(&self.content)
    }

    /// Checks if this message needs a summary (content > threshold).
    pub fn needs_summary(&self, threshold: usize) -> bool {
        self.summary.is_none() && self.content.len() > threshold
    }
}

/// A conversation document for aggregated metadata.
///
/// This provides a higher-level view of conversations for quick filtering.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConversationDocument {
    /// Unique identifier for this conversation
    pub id: String,

    /// Preview of the conversation content (first message or summary)
    pub content_preview: String,

    /// Model used for this conversation
    pub model: String,

    /// Unix timestamp of conversation creation
    pub created_at: i64,

    /// Unix timestamp of last update
    pub updated_at: i64,

    /// Number of messages in this conversation
    pub message_count: usize,

    /// Primary working directory for this conversation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,

    /// Aggregated list of all files touched in this conversation
    #[serde(default)]
    pub files_summary: Vec<String>,
}

impl ConversationDocument {
    /// Creates a new ConversationDocument.
    pub fn new(
        id: impl Into<String>,
        content_preview: impl Into<String>,
        model: impl Into<String>,
        created_at: i64,
    ) -> Self {
        Self {
            id: id.into(),
            content_preview: content_preview.into(),
            model: model.into(),
            created_at,
            updated_at: created_at,
            message_count: 0,
            cwd: None,
            files_summary: Vec::new(),
        }
    }

    /// Updates the conversation with a new message.
    pub fn update_from_message(&mut self, message: &MessageDocument) {
        self.updated_at = message.created_at;
        self.message_count = message.turn_index + 1;

        // Update cwd if not set
        if self.cwd.is_none() {
            self.cwd = message.cwd.clone();
        }

        // Aggregate files
        for file in &message.files_touched {
            if !self.files_summary.contains(file) {
                self.files_summary.push(file.clone());
            }
        }
    }
}

/// Configuration for the memory system.
#[derive(Debug, Clone)]
pub struct MemoryConfig {
    /// Meilisearch URL
    pub meilisearch_url: String,

    /// Optional Meilisearch API key
    pub meilisearch_key: Option<String>,

    /// Index name for messages
    pub messages_index: String,

    /// Index name for conversations
    pub conversations_index: String,

    /// Minimum content length to trigger summary generation
    pub summary_threshold: usize,

    /// Maximum number of context items to inject
    pub max_context_items: usize,

    /// Token budget for injected context (~4 chars per token)
    pub token_budget: usize,

    /// Minimum relevance score to include in results
    pub min_relevance_score: f64,

    /// Whether memory is enabled
    pub enabled: bool,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            meilisearch_url: std::env::var("MEILISEARCH_URL")
                .unwrap_or_else(|_| "http://localhost:7700".to_string()),
            meilisearch_key: std::env::var("MEILISEARCH_KEY").ok(),
            messages_index: "nexus_messages".to_string(),
            conversations_index: "nexus_conversations".to_string(),
            summary_threshold: 500,
            max_context_items: 5,
            token_budget: 2000,
            min_relevance_score: 0.3,
            enabled: true,
        }
    }
}

impl MemoryConfig {
    /// Creates a new MemoryConfig with custom URL.
    pub fn with_url(mut self, url: impl Into<String>) -> Self {
        self.meilisearch_url = url.into();
        self
    }

    /// Sets the API key.
    pub fn with_key(mut self, key: impl Into<String>) -> Self {
        self.meilisearch_key = Some(key.into());
        self
    }

    /// Enables or disables memory.
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Sets the maximum context items.
    pub fn with_max_context_items(mut self, max: usize) -> Self {
        self.max_context_items = max;
        self
    }

    /// Sets the token budget.
    pub fn with_token_budget(mut self, budget: usize) -> Self {
        self.token_budget = budget;
        self
    }

    /// Sets the minimum relevance score.
    pub fn with_min_relevance_score(mut self, score: f64) -> Self {
        self.min_relevance_score = score;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_document_serialization() {
        let msg = MessageDocument::new(
            "msg-123",
            "conv-456",
            "user",
            "How do I implement JWT auth?",
            0,
            1700000000,
        )
        .with_cwd("/projects/my-api")
        .with_files_touched(vec!["/projects/my-api/src/auth.rs".to_string()]);

        let json = serde_json::to_string(&msg).unwrap();
        let parsed: MessageDocument = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.id, "msg-123");
        assert_eq!(parsed.cwd, Some("/projects/my-api".to_string()));
        assert_eq!(parsed.files_touched.len(), 1);
    }

    #[test]
    fn test_message_document_without_optional_fields() {
        let msg = MessageDocument::new("msg-1", "conv-1", "assistant", "Hello!", 0, 1700000000);

        let json = serde_json::to_string(&msg).unwrap();

        // Optional fields should not be present in JSON
        assert!(!json.contains("cwd"));
        assert!(!json.contains("summary"));
    }

    #[test]
    fn test_display_content_prefers_summary() {
        let msg = MessageDocument::new(
            "msg-1",
            "conv-1",
            "assistant",
            "Long content...",
            0,
            1700000000,
        )
        .with_summary("Short summary");

        assert_eq!(msg.display_content(), "Short summary");
    }

    #[test]
    fn test_display_content_falls_back_to_content() {
        let msg = MessageDocument::new(
            "msg-1",
            "conv-1",
            "assistant",
            "Full content",
            0,
            1700000000,
        );

        assert_eq!(msg.display_content(), "Full content");
    }

    #[test]
    fn test_needs_summary() {
        let short_msg = MessageDocument::new("msg-1", "conv-1", "user", "Hi", 0, 1700000000);
        let long_msg = MessageDocument::new(
            "msg-2",
            "conv-1",
            "assistant",
            "x".repeat(600),
            0,
            1700000000,
        );
        let summarized = MessageDocument::new(
            "msg-3",
            "conv-1",
            "assistant",
            "x".repeat(600),
            0,
            1700000000,
        )
        .with_summary("Summary");

        assert!(!short_msg.needs_summary(500));
        assert!(long_msg.needs_summary(500));
        assert!(!summarized.needs_summary(500));
    }

    #[test]
    fn test_add_file_touched_deduplication() {
        let mut msg = MessageDocument::new("msg-1", "conv-1", "user", "test", 0, 1700000000);
        msg.add_file_touched("/src/main.rs");
        msg.add_file_touched("/src/lib.rs");
        msg.add_file_touched("/src/main.rs"); // Duplicate

        assert_eq!(msg.files_touched.len(), 2);
    }

    #[test]
    fn test_conversation_document_update() {
        let mut conv = ConversationDocument::new("conv-1", "Preview", "claude-3", 1700000000);

        let msg = MessageDocument::new("msg-1", "conv-1", "user", "test", 2, 1700001000)
            .with_cwd("/projects/test")
            .with_files_touched(vec!["/projects/test/src/main.rs".to_string()]);

        conv.update_from_message(&msg);

        assert_eq!(conv.message_count, 3);
        assert_eq!(conv.updated_at, 1700001000);
        assert_eq!(conv.cwd, Some("/projects/test".to_string()));
        assert_eq!(conv.files_summary.len(), 1);
    }

    #[test]
    fn test_memory_config_defaults() {
        let config = MemoryConfig::default();

        assert_eq!(config.messages_index, "nexus_messages");
        assert_eq!(config.summary_threshold, 500);
        assert_eq!(config.max_context_items, 5);
        assert_eq!(config.token_budget, 2000);
        assert!(config.enabled);
    }

    #[test]
    fn test_memory_config_builder() {
        let config = MemoryConfig::default()
            .with_url("http://custom:7700")
            .with_enabled(false)
            .with_max_context_items(10);

        assert_eq!(config.meilisearch_url, "http://custom:7700");
        assert!(!config.enabled);
        assert_eq!(config.max_context_items, 10);
    }
}
