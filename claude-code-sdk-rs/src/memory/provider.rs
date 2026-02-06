//! Memory provider implementations for persistent storage.
//!
//! This module provides the Meilisearch-based memory provider that handles:
//! - Storing and retrieving messages
//! - Applying multi-factor relevance scoring
//! - Managing index settings

use super::{
    ConversationDocument, MemoryConfig, MessageDocument, RelevanceConfig, RelevanceScore,
    RelevanceScorer,
};
use async_trait::async_trait;
use chrono::Utc;
use meilisearch_sdk::client::Client;
use meilisearch_sdk::settings::Settings;
use serde::Deserialize;

/// Result type for memory operations.
pub type MemoryResult<T> = Result<T, MemoryError>;

/// Errors that can occur during memory operations.
#[derive(Debug, thiserror::Error)]
pub enum MemoryError {
    /// Meilisearch client error
    #[error("Meilisearch error: {0}")]
    Meilisearch(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// Memory is disabled
    #[error("Memory is disabled")]
    Disabled,
}

impl From<meilisearch_sdk::errors::Error> for MemoryError {
    fn from(e: meilisearch_sdk::errors::Error) -> Self {
        MemoryError::Meilisearch(e.to_string())
    }
}

/// Context for querying memory.
#[derive(Debug, Clone, Default)]
pub struct QueryContext {
    /// The search query text
    pub query: String,

    /// Current working directory
    pub cwd: Option<String>,

    /// Files in the current context
    pub files: Vec<String>,
}

impl QueryContext {
    /// Creates a new QueryContext with a query.
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            ..Default::default()
        }
    }

    /// Sets the working directory.
    pub fn with_cwd(mut self, cwd: impl Into<String>) -> Self {
        self.cwd = Some(cwd.into());
        self
    }

    /// Sets the files in context.
    pub fn with_files(mut self, files: Vec<String>) -> Self {
        self.files = files;
        self
    }
}

/// A memory result with relevance scoring.
#[derive(Debug, Clone)]
pub struct ScoredMemoryResult {
    /// The message document
    pub document: MessageDocument,

    /// The relevance score breakdown
    pub score: RelevanceScore,
}

/// Trait for memory providers.
#[async_trait]
pub trait MemoryProvider: Send + Sync {
    /// Stores a message in memory.
    async fn store_message(&self, message: &MessageDocument) -> MemoryResult<()>;

    /// Stores multiple messages in memory.
    async fn store_messages(&self, messages: &[MessageDocument]) -> MemoryResult<()>;

    /// Retrieves relevant context for a query.
    async fn retrieve_context(
        &self,
        context: &QueryContext,
        limit: usize,
    ) -> MemoryResult<Vec<ScoredMemoryResult>>;

    /// Updates a conversation document.
    async fn update_conversation(&self, conversation: &ConversationDocument) -> MemoryResult<()>;

    /// Checks if memory is healthy.
    async fn health_check(&self) -> MemoryResult<bool>;
}

/// Meilisearch-based memory provider.
pub struct MeilisearchMemoryProvider {
    client: Client,
    config: MemoryConfig,
    scorer: RelevanceScorer,
}

impl MeilisearchMemoryProvider {
    /// Creates a new MeilisearchMemoryProvider.
    pub async fn new(config: MemoryConfig) -> MemoryResult<Self> {
        if !config.enabled {
            return Err(MemoryError::Disabled);
        }

        let client = Client::new(&config.meilisearch_url, config.meilisearch_key.as_deref())
            .map_err(|e| MemoryError::Meilisearch(e.to_string()))?;

        let scorer = RelevanceScorer::new(RelevanceConfig::default());

        let provider = Self {
            client,
            config,
            scorer,
        };

        // Setup indexes
        provider.setup_indexes().await?;

        Ok(provider)
    }

    /// Sets up Meilisearch indexes with proper settings.
    async fn setup_indexes(&self) -> MemoryResult<()> {
        // Messages index
        let _ = self
            .client
            .create_index(&self.config.messages_index, Some("id"))
            .await;

        let messages_index = self.client.index(&self.config.messages_index);
        let messages_settings = Settings::new()
            .with_searchable_attributes(["content", "summary", "role"])
            .with_filterable_attributes(["conversation_id", "role", "cwd", "created_at"])
            .with_sortable_attributes(["created_at", "turn_index"]);

        messages_index.set_settings(&messages_settings).await?;

        // Conversations index
        let _ = self
            .client
            .create_index(&self.config.conversations_index, Some("id"))
            .await;

        let conversations_index = self.client.index(&self.config.conversations_index);
        let conversations_settings = Settings::new()
            .with_searchable_attributes(["content_preview", "model"])
            .with_filterable_attributes(["model", "cwd", "created_at", "updated_at"])
            .with_sortable_attributes(["created_at", "updated_at", "message_count"]);

        conversations_index
            .set_settings(&conversations_settings)
            .await?;

        Ok(())
    }

    /// Builds a filter string for Meilisearch based on the query context.
    fn build_filter(&self, context: &QueryContext) -> Option<String> {
        let mut filters = Vec::new();

        // Filter by cwd if provided (exact match or prefix)
        if let Some(ref cwd) = context.cwd {
            // Use a STARTS_WITH-like filter for cwd matching
            filters.push(format!("cwd = \"{}\"", cwd));
        }

        if filters.is_empty() {
            None
        } else {
            Some(filters.join(" AND "))
        }
    }

    /// Computes the age in hours for a message.
    fn compute_age_hours(&self, created_at: i64) -> f64 {
        let now = Utc::now().timestamp();
        let age_seconds = (now - created_at).max(0) as f64;
        age_seconds / 3600.0
    }

    /// Applies relevance scoring to search results.
    fn score_results(
        &self,
        hits: Vec<SearchHit>,
        context: &QueryContext,
    ) -> Vec<ScoredMemoryResult> {
        let mut results: Vec<ScoredMemoryResult> = hits
            .into_iter()
            .map(|hit| {
                let age_hours = self.compute_age_hours(hit.document.created_at);

                let score = self.scorer.compute_score(
                    hit.score.unwrap_or(0.0),
                    context.cwd.as_deref(),
                    hit.document.cwd.as_deref(),
                    &context.files,
                    &hit.document.files_touched,
                    age_hours,
                );

                ScoredMemoryResult {
                    document: hit.document,
                    score,
                }
            })
            .collect();

        // Sort by total score descending
        results.sort_by(|a, b| {
            b.score
                .total
                .partial_cmp(&a.score.total)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Filter by minimum relevance score
        results.retain(|r| r.score.total >= self.config.min_relevance_score);

        results
    }

    /// Applies token budget to results.
    fn apply_token_budget(&self, results: Vec<ScoredMemoryResult>) -> Vec<ScoredMemoryResult> {
        let mut budget_remaining = self.config.token_budget * 4; // ~4 chars per token
        let mut selected = Vec::new();

        for result in results {
            let content_len = result.document.display_content().len();
            if content_len <= budget_remaining {
                budget_remaining -= content_len;
                selected.push(result);
            } else if selected.is_empty() {
                // Always include at least one result even if over budget
                selected.push(result);
                break;
            }
        }

        selected
    }
}

/// Internal search hit structure.
#[derive(Debug, Deserialize)]
struct SearchHit {
    #[serde(flatten)]
    document: MessageDocument,
    #[serde(rename = "_rankingScore")]
    score: Option<f64>,
}

#[async_trait]
impl MemoryProvider for MeilisearchMemoryProvider {
    async fn store_message(&self, message: &MessageDocument) -> MemoryResult<()> {
        let index = self.client.index(&self.config.messages_index);

        index.add_documents(&[message], Some("id")).await?;

        Ok(())
    }

    async fn store_messages(&self, messages: &[MessageDocument]) -> MemoryResult<()> {
        if messages.is_empty() {
            return Ok(());
        }

        let index = self.client.index(&self.config.messages_index);

        index.add_documents(messages, Some("id")).await?;

        Ok(())
    }

    async fn retrieve_context(
        &self,
        context: &QueryContext,
        limit: usize,
    ) -> MemoryResult<Vec<ScoredMemoryResult>> {
        let index = self.client.index(&self.config.messages_index);

        // Build filter first so it lives long enough
        let filter = self.build_filter(context);

        // Build search query
        let mut search = index.search();
        search.with_query(&context.query);
        search.with_limit(limit * 2); // Get more than needed for post-filtering
        search.with_show_ranking_score(true);

        // Apply filters
        if let Some(ref f) = filter {
            search.with_filter(f);
        }

        // Execute search
        let results: meilisearch_sdk::search::SearchResults<MessageDocument> =
            search.execute().await?;

        // Convert to SearchHit format
        let hits: Vec<SearchHit> = results
            .hits
            .into_iter()
            .map(|h| SearchHit {
                document: h.result,
                score: h.ranking_score,
            })
            .collect();

        // Score and filter results
        let scored = self.score_results(hits, context);

        // Apply limits
        let limited: Vec<_> = scored
            .into_iter()
            .take(self.config.max_context_items.min(limit))
            .collect();

        // Apply token budget
        let budgeted = self.apply_token_budget(limited);

        Ok(budgeted)
    }

    async fn update_conversation(&self, conversation: &ConversationDocument) -> MemoryResult<()> {
        let index = self.client.index(&self.config.conversations_index);

        index.add_documents(&[conversation], Some("id")).await?;

        Ok(())
    }

    async fn health_check(&self) -> MemoryResult<bool> {
        match self.client.health().await {
            Ok(_) => Ok(true),
            Err(e) => Err(MemoryError::Meilisearch(e.to_string())),
        }
    }
}

/// Builder for creating memory providers.
pub struct MemoryProviderBuilder {
    config: MemoryConfig,
}

impl MemoryProviderBuilder {
    /// Creates a new builder with default config.
    pub fn new() -> Self {
        Self {
            config: MemoryConfig::default(),
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

    /// Sets the minimum relevance score.
    pub fn min_relevance_score(mut self, score: f64) -> Self {
        self.config.min_relevance_score = score;
        self
    }

    /// Sets the summary threshold.
    pub fn summary_threshold(mut self, threshold: usize) -> Self {
        self.config.summary_threshold = threshold;
        self
    }

    /// Builds the Meilisearch memory provider.
    pub async fn build(self) -> MemoryResult<MeilisearchMemoryProvider> {
        MeilisearchMemoryProvider::new(self.config).await
    }

    /// Builds the config without creating a provider.
    pub fn build_config(self) -> MemoryConfig {
        self.config
    }
}

impl Default for MemoryProviderBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Formats context for injection into prompts.
pub struct ContextFormatter;

impl ContextFormatter {
    /// Formats scored results for injection into a prompt.
    pub fn format_for_prompt(results: &[ScoredMemoryResult]) -> String {
        if results.is_empty() {
            return String::new();
        }

        let mut output = String::new();
        output.push_str("## Contexte historique (pour référence)\n\n");
        output.push_str("Les informations suivantes proviennent de conversations précédentes et peuvent être pertinentes :\n\n");

        for (i, result) in results.iter().enumerate() {
            let age_desc = Self::format_age(result.document.created_at);
            let role = &result.document.role;
            let content = result.document.display_content();

            output.push_str(&format!(
                "{}. [{}] ({})\n   \"{}\"\n\n",
                i + 1,
                age_desc,
                role,
                Self::truncate(content, 200)
            ));
        }

        output.push_str("---\n\n");
        output.push_str("## Conversation actuelle (prioritaire)\n\n");

        output
    }

    /// Formats the age of a message in human-readable form.
    fn format_age(created_at: i64) -> String {
        let now = Utc::now().timestamp();
        let age_seconds = (now - created_at).max(0);

        if age_seconds < 3600 {
            let minutes = age_seconds / 60;
            format!("Il y a {} min", minutes.max(1))
        } else if age_seconds < 86400 {
            let hours = age_seconds / 3600;
            format!("Il y a {} h", hours)
        } else {
            let days = age_seconds / 86400;
            if days == 1 {
                "Hier".to_string()
            } else {
                format!("Il y a {} jours", days)
            }
        }
    }

    /// Truncates text with ellipsis.
    fn truncate(s: &str, max_len: usize) -> String {
        if s.len() <= max_len {
            s.to_string()
        } else {
            format!("{}...", &s[..max_len])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_context_builder() {
        let ctx = QueryContext::new("test query")
            .with_cwd("/projects/app")
            .with_files(vec!["/src/main.rs".to_string()]);

        assert_eq!(ctx.query, "test query");
        assert_eq!(ctx.cwd, Some("/projects/app".to_string()));
        assert_eq!(ctx.files.len(), 1);
    }

    #[test]
    fn test_memory_provider_builder() {
        let config = MemoryProviderBuilder::new()
            .url("http://localhost:7700")
            .key("test-key")
            .enabled(true)
            .max_context_items(10)
            .token_budget(3000)
            .min_relevance_score(0.5)
            .build_config();

        assert_eq!(config.meilisearch_url, "http://localhost:7700");
        assert_eq!(config.meilisearch_key, Some("test-key".to_string()));
        assert!(config.enabled);
        assert_eq!(config.max_context_items, 10);
        assert_eq!(config.token_budget, 3000);
        assert_eq!(config.min_relevance_score, 0.5);
    }

    #[test]
    fn test_context_formatter_format_age() {
        let now = Utc::now().timestamp();

        // A few minutes ago
        assert!(ContextFormatter::format_age(now - 300).contains("min"));

        // A few hours ago
        assert!(ContextFormatter::format_age(now - 7200).contains("h"));

        // Yesterday
        assert_eq!(ContextFormatter::format_age(now - 86400), "Hier");

        // Several days ago
        assert!(ContextFormatter::format_age(now - 259200).contains("jours"));
    }

    #[test]
    fn test_context_formatter_truncate() {
        assert_eq!(ContextFormatter::truncate("short", 100), "short");
        assert_eq!(
            ContextFormatter::truncate("this is a long text", 10),
            "this is a ..."
        );
    }

    #[test]
    fn test_context_formatter_empty_results() {
        let results: Vec<ScoredMemoryResult> = vec![];
        let formatted = ContextFormatter::format_for_prompt(&results);
        assert!(formatted.is_empty());
    }
}
