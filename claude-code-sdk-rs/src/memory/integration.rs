//! Integration module for memory system with SDK conversation flow.
//!
//! This module provides the glue between the memory system and the
//! SDK's conversation management.

#[cfg(feature = "memory")]
use super::{ContextFormatter, MeilisearchMemoryProvider, MemoryProvider};
use super::{DefaultToolContextExtractor, MemoryConfig, MessageContextAggregator, MessageDocument};

#[cfg(feature = "memory")]
use chrono::Utc;
#[cfg(not(feature = "memory"))]
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;
#[cfg(feature = "memory")]
use std::sync::Arc;
use uuid::Uuid;

/// Returns the current Unix timestamp in seconds.
fn current_timestamp() -> i64 {
    #[cfg(feature = "memory")]
    {
        Utc::now().timestamp()
    }
    #[cfg(not(feature = "memory"))]
    {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0)
    }
}

/// Query context for memory retrieval.
/// Only defined when memory feature is disabled (provider.rs defines it otherwise).
#[cfg(not(feature = "memory"))]
#[derive(Debug, Clone)]
pub struct QueryContext {
    /// The query text.
    pub query: String,
    /// Current working directory.
    pub cwd: Option<String>,
    /// Files currently being worked on.
    pub files: Vec<String>,
}

#[cfg(feature = "memory")]
use super::provider::QueryContext;

/// Manages conversation context for memory operations.
///
/// This struct tracks the current conversation state and provides
/// methods for capturing and injecting context.
pub struct ConversationMemoryManager {
    /// Current conversation ID
    conversation_id: String,

    /// Current working directory
    cwd: Option<String>,

    /// Context aggregator for the current turn
    aggregator: MessageContextAggregator,

    /// Tool context extractor (reserved for future use)
    #[allow(dead_code)]
    extractor: DefaultToolContextExtractor,

    /// Current turn index
    turn_index: usize,

    /// Messages pending storage
    pending_messages: Vec<MessageDocument>,

    /// Memory configuration
    config: MemoryConfig,
}

impl ConversationMemoryManager {
    /// Creates a new ConversationMemoryManager.
    pub fn new(config: MemoryConfig) -> Self {
        Self {
            conversation_id: format!("conv-{}", Uuid::new_v4()),
            cwd: None,
            aggregator: MessageContextAggregator::new(),
            extractor: DefaultToolContextExtractor::new(),
            turn_index: 0,
            pending_messages: Vec::new(),
            config,
        }
    }

    /// Creates a manager with a specific conversation ID.
    pub fn with_conversation_id(mut self, id: impl Into<String>) -> Self {
        self.conversation_id = id.into();
        self
    }

    /// Sets the working directory.
    pub fn with_cwd(mut self, cwd: impl Into<String>) -> Self {
        let cwd = cwd.into();
        self.cwd = Some(cwd.clone());
        self.aggregator = MessageContextAggregator::with_initial_cwd(cwd);
        self
    }

    /// Returns the current conversation ID.
    pub fn conversation_id(&self) -> &str {
        &self.conversation_id
    }

    /// Returns the current working directory.
    pub fn cwd(&self) -> Option<&str> {
        self.cwd.as_deref()
    }

    /// Returns the current turn index.
    pub fn turn_index(&self) -> usize {
        self.turn_index
    }

    /// Returns whether memory is enabled.
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Processes a tool call and accumulates context.
    ///
    /// Call this for each tool use observed during a turn.
    pub fn process_tool_call(&mut self, tool_name: &str, input: &Value) {
        if !self.config.enabled {
            return;
        }

        self.aggregator.process_tool_call(tool_name, input);

        // Update cwd if changed
        if let Some(new_cwd) = self.aggregator.cwd() {
            self.cwd = Some(new_cwd.to_string());
        }
    }

    /// Records a user message.
    ///
    /// Call this when a user message is received.
    pub fn record_user_message(&mut self, content: &str) {
        if !self.config.enabled {
            return;
        }

        let timestamp = current_timestamp();
        let msg = MessageDocument::new(
            format!("msg-{}", Uuid::new_v4()),
            &self.conversation_id,
            "user",
            content,
            self.turn_index,
            timestamp,
        );

        let msg = if let Some(ref cwd) = self.cwd {
            msg.with_cwd(cwd.clone())
        } else {
            msg
        };

        self.pending_messages.push(msg);
    }

    /// Records an assistant message.
    ///
    /// Call this when an assistant response is complete.
    /// This also captures the accumulated tool context.
    pub fn record_assistant_message(&mut self, content: &str) {
        if !self.config.enabled {
            return;
        }

        let context = self.aggregator.finalize();
        let timestamp = current_timestamp();

        let msg = MessageDocument::new(
            format!("msg-{}", Uuid::new_v4()),
            &self.conversation_id,
            "assistant",
            content,
            self.turn_index,
            timestamp,
        )
        .with_files_touched(context.files);

        let msg = if let Some(ref cwd) = self.cwd {
            msg.with_cwd(cwd.clone())
        } else {
            msg
        };

        self.pending_messages.push(msg);

        // Prepare for next turn
        self.turn_index += 1;
        self.aggregator.reset();
    }

    /// Returns and clears pending messages for storage.
    pub fn take_pending_messages(&mut self) -> Vec<MessageDocument> {
        std::mem::take(&mut self.pending_messages)
    }

    /// Returns the current context for querying memory.
    pub fn current_context(&self, query: &str) -> QueryContext {
        QueryContext {
            query: query.to_string(),
            cwd: self.cwd.clone(),
            files: self.aggregator.files(),
        }
    }

    /// Returns the memory configuration.
    pub fn config(&self) -> &MemoryConfig {
        &self.config
    }
}

/// Builder for creating memory-enabled conversations.
pub struct MemoryIntegrationBuilder {
    config: MemoryConfig,
    conversation_id: Option<String>,
    cwd: Option<String>,
}

impl MemoryIntegrationBuilder {
    /// Creates a new builder with default config.
    pub fn new() -> Self {
        Self {
            config: MemoryConfig::default(),
            conversation_id: None,
            cwd: None,
        }
    }

    /// Sets the Meilisearch URL.
    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.config.meilisearch_url = url.into();
        self
    }

    /// Sets the API key.
    pub fn key(mut self, key: impl Into<String>) -> Self {
        self.config.meilisearch_key = Some(key.into());
        self
    }

    /// Enables or disables memory.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.config.enabled = enabled;
        self
    }

    /// Sets the conversation ID.
    pub fn conversation_id(mut self, id: impl Into<String>) -> Self {
        self.conversation_id = Some(id.into());
        self
    }

    /// Sets the working directory.
    pub fn cwd(mut self, cwd: impl Into<String>) -> Self {
        self.cwd = Some(cwd.into());
        self
    }

    /// Sets the minimum relevance score.
    pub fn min_relevance_score(mut self, score: f64) -> Self {
        self.config.min_relevance_score = score;
        self
    }

    /// Sets the maximum context items.
    pub fn max_context_items(mut self, max: usize) -> Self {
        self.config.max_context_items = max;
        self
    }

    /// Sets the token budget.
    pub fn token_budget(mut self, budget: usize) -> Self {
        self.config.token_budget = budget;
        self
    }

    /// Builds the ConversationMemoryManager.
    pub fn build(self) -> ConversationMemoryManager {
        let mut manager = ConversationMemoryManager::new(self.config);

        if let Some(id) = self.conversation_id {
            manager = manager.with_conversation_id(id);
        }

        if let Some(cwd) = self.cwd {
            manager = manager.with_cwd(cwd);
        }

        manager
    }
}

impl Default for MemoryIntegrationBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Context injector for formatting and injecting memory context into prompts.
#[cfg(feature = "memory")]
pub struct ContextInjector {
    provider: Arc<MeilisearchMemoryProvider>,
    config: MemoryConfig,
}

#[cfg(feature = "memory")]
impl ContextInjector {
    /// Creates a new ContextInjector.
    pub async fn new(config: MemoryConfig) -> Result<Self, super::provider::MemoryError> {
        let provider = MeilisearchMemoryProvider::new(config.clone()).await?;
        Ok(Self {
            provider: Arc::new(provider),
            config,
        })
    }

    /// Retrieves and formats context for injection into a prompt.
    ///
    /// Returns a formatted string to prepend to the conversation,
    /// or None if no relevant context was found.
    pub async fn get_context_prefix(
        &self,
        query: &str,
        cwd: Option<&str>,
        files: &[String],
    ) -> Result<Option<String>, super::provider::MemoryError> {
        if !self.config.enabled {
            return Ok(None);
        }

        let context = QueryContext {
            query: query.to_string(),
            cwd: cwd.map(String::from),
            files: files.to_vec(),
        };

        let results = self
            .provider
            .retrieve_context(&context, self.config.max_context_items)
            .await?;

        if results.is_empty() {
            return Ok(None);
        }

        let formatted = ContextFormatter::format_for_prompt(&results);
        Ok(Some(formatted))
    }

    /// Stores messages in the memory system.
    pub async fn store_messages(
        &self,
        messages: &[MessageDocument],
    ) -> Result<(), super::provider::MemoryError> {
        if !self.config.enabled || messages.is_empty() {
            return Ok(());
        }

        self.provider.store_messages(messages).await
    }

    /// Returns a reference to the underlying provider.
    pub fn provider(&self) -> &MeilisearchMemoryProvider {
        &self.provider
    }
}

/// Summary generator for long messages.
///
/// This is a placeholder that can be extended to use an LLM
/// for generating summaries.
pub struct SummaryGenerator {
    /// Minimum content length to trigger summarization
    threshold: usize,
}

impl SummaryGenerator {
    /// Creates a new SummaryGenerator.
    pub fn new(threshold: usize) -> Self {
        Self { threshold }
    }

    /// Creates with default threshold (500 chars).
    pub fn default_threshold() -> Self {
        Self::new(500)
    }

    /// Checks if content needs summarization.
    pub fn needs_summary(&self, content: &str) -> bool {
        content.len() > self.threshold
    }

    /// Generates a simple extractive summary.
    ///
    /// This is a basic implementation that extracts the first
    /// and last sentences. For production use, consider using
    /// an LLM for abstractive summarization.
    pub fn generate_simple_summary(&self, content: &str) -> String {
        if !self.needs_summary(content) {
            return content.to_string();
        }

        // Simple extractive summary: first sentence + "..." + last sentence
        let sentences: Vec<&str> = content
            .split(['.', '!', '?'])
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();

        match sentences.len() {
            0 => content[..self.threshold.min(content.len())].to_string() + "...",
            1 => sentences[0].to_string(),
            2 => format!("{}. ... {}", sentences[0], sentences[1]),
            _ => format!("{}. ... {}", sentences[0], sentences[sentences.len() - 1]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conversation_memory_manager_new() {
        let config = MemoryConfig::default().with_enabled(true);
        let manager = ConversationMemoryManager::new(config);

        assert!(manager.conversation_id().starts_with("conv-"));
        assert_eq!(manager.turn_index(), 0);
        assert!(manager.is_enabled());
    }

    #[test]
    fn test_conversation_memory_manager_with_cwd() {
        let config = MemoryConfig::default().with_enabled(true);
        let manager = ConversationMemoryManager::new(config).with_cwd("/projects/test");

        assert_eq!(manager.cwd(), Some("/projects/test"));
    }

    #[test]
    fn test_record_messages() {
        let config = MemoryConfig::default().with_enabled(true);
        let mut manager = ConversationMemoryManager::new(config).with_cwd("/projects/test");

        manager.record_user_message("Hello");
        manager.record_assistant_message("Hi there!");

        let messages = manager.take_pending_messages();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, "user");
        assert_eq!(messages[1].role, "assistant");
        assert_eq!(messages[0].cwd, Some("/projects/test".to_string()));
    }

    #[test]
    fn test_process_tool_call() {
        let config = MemoryConfig::default().with_enabled(true);
        let mut manager = ConversationMemoryManager::new(config);

        manager.process_tool_call(
            "Read",
            &serde_json::json!({
                "file_path": "/src/main.rs"
            }),
        );
        manager.process_tool_call(
            "Bash",
            &serde_json::json!({
                "command": "cd /projects/app && cargo build"
            }),
        );

        assert_eq!(manager.cwd(), Some("/projects/app"));

        let ctx = manager.current_context("test query");
        assert!(ctx.files.contains(&"/src/main.rs".to_string()));
    }

    #[test]
    fn test_disabled_memory_does_nothing() {
        let config = MemoryConfig::default().with_enabled(false);
        let mut manager = ConversationMemoryManager::new(config);

        manager.record_user_message("Hello");
        manager.record_assistant_message("Hi!");

        let messages = manager.take_pending_messages();
        assert!(messages.is_empty());
    }

    #[test]
    fn test_memory_integration_builder() {
        let manager = MemoryIntegrationBuilder::new()
            .enabled(true)
            .cwd("/projects/test")
            .conversation_id("test-conv-1")
            .min_relevance_score(0.5)
            .max_context_items(10)
            .build();

        assert_eq!(manager.conversation_id(), "test-conv-1");
        assert_eq!(manager.cwd(), Some("/projects/test"));
        assert!(manager.is_enabled());
        assert_eq!(manager.config().min_relevance_score, 0.5);
        assert_eq!(manager.config().max_context_items, 10);
    }

    #[test]
    fn test_summary_generator() {
        let generator = SummaryGenerator::new(50);

        // Short content doesn't need summary
        assert!(!generator.needs_summary("Short text."));

        // Long content needs summary
        let long_content = "First sentence. Second sentence. Third sentence. Fourth sentence. Fifth sentence. Sixth sentence.";
        assert!(generator.needs_summary(long_content));

        let summary = generator.generate_simple_summary(long_content);
        assert!(summary.contains("First sentence"));
        assert!(summary.contains("Sixth sentence"));
        assert!(summary.contains("..."));
    }

    #[test]
    fn test_turn_index_increments() {
        let config = MemoryConfig::default().with_enabled(true);
        let mut manager = ConversationMemoryManager::new(config);

        assert_eq!(manager.turn_index(), 0);

        manager.record_user_message("Q1");
        manager.record_assistant_message("A1");
        assert_eq!(manager.turn_index(), 1);

        manager.record_user_message("Q2");
        manager.record_assistant_message("A2");
        assert_eq!(manager.turn_index(), 2);
    }
}
