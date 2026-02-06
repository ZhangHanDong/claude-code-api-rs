//! Neo4j-backed hook callback implementation
//!
//! Captures all Claude Code events into Neo4j for knowledge graph persistence.

#![allow(dead_code)] // Public API - may not be used internally
//!
//! ## Schema
//!
//! ```cypher
//! // Nodes
//! (:NexusToolUsage {
//!     id: String,
//!     tool_name: String,
//!     input: String,      // JSON serialized
//!     output: String,     // JSON serialized (truncated if large)
//!     duration_ms: Int?,
//!     session_id: String,
//!     created_at: DateTime
//! })
//!
//! (:NexusUserPrompt {
//!     id: String,
//!     prompt: String,
//!     session_id: String,
//!     created_at: DateTime
//! })
//!
//! (:NexusSessionEvent {
//!     id: String,
//!     event_type: String,  // "start", "stop", "compact"
//!     session_id: String,
//!     duration_ms: Int?,
//!     turn_count: Int?,
//!     cost_usd: Float?,
//!     created_at: DateTime
//! })
//!
//! // Relationships
//! (:NexusConversation)-[:HAS_TOOL_USAGE]->(:NexusToolUsage)
//! (:NexusConversation)-[:HAS_PROMPT]->(:NexusUserPrompt)
//! (:NexusSession)-[:HAS_EVENT]->(:NexusSessionEvent)
//!
//! // Constraints
//! CREATE CONSTRAINT nexus_tool_usage_id IF NOT EXISTS FOR (t:NexusToolUsage) REQUIRE t.id IS UNIQUE;
//! CREATE CONSTRAINT nexus_user_prompt_id IF NOT EXISTS FOR (p:NexusUserPrompt) REQUIRE p.id IS UNIQUE;
//! CREATE CONSTRAINT nexus_session_event_id IF NOT EXISTS FOR (e:NexusSessionEvent) REQUIRE e.id IS UNIQUE;
//! ```

use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use neo4rs::{Graph, query};
use nexus_claude::{
    HookCallback, HookContext, HookInput, HookJSONOutput, PostToolUseHookInput,
    PreToolUseHookInput, SdkError, StopHookInput, SyncHookJSONOutput, UserPromptSubmitHookInput,
};
use std::sync::Arc;
use std::time::Instant;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::core::storage::meilisearch::MeilisearchClient;

/// Index name for tool usage documents in Meilisearch
pub const INDEX_TOOL_USAGE: &str = "nexus_tool_usage";

/// Configuration for Neo4jHookCallback
#[derive(Clone, Debug)]
pub struct Neo4jHookCallbackConfig {
    /// Maximum size of tool output to store (bytes)
    pub max_output_size: usize,
    /// Whether to index tool usage in Meilisearch
    pub index_in_meilisearch: bool,
    /// Whether to log PreToolUse events (verbose)
    pub log_pre_tool_use: bool,
}

impl Default for Neo4jHookCallbackConfig {
    fn default() -> Self {
        Self {
            max_output_size: 10_000, // 10KB max
            index_in_meilisearch: true,
            log_pre_tool_use: false,
        }
    }
}

/// Tool usage document for Meilisearch indexing
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolUsageDocument {
    pub id: String,
    pub tool_name: String,
    pub input_summary: String,
    pub output_summary: String,
    pub session_id: String,
    pub created_at: i64,
}

/// Neo4j-backed hook callback that captures all conversation events
pub struct Neo4jHookCallback {
    graph: Arc<Graph>,
    meilisearch: Option<Arc<MeilisearchClient>>,
    config: Neo4jHookCallbackConfig,
    /// Track PreToolUse timestamps for duration calculation
    tool_start_times: dashmap::DashMap<String, Instant>,
}

impl Neo4jHookCallback {
    /// Create a new Neo4jHookCallback
    pub fn new(
        graph: Arc<Graph>,
        meilisearch: Option<Arc<MeilisearchClient>>,
        config: Neo4jHookCallbackConfig,
    ) -> Self {
        Self {
            graph,
            meilisearch,
            config,
            tool_start_times: dashmap::DashMap::new(),
        }
    }

    /// Initialize Neo4j schema for hook events
    pub async fn init_schema(&self) -> Result<()> {
        let constraints = vec![
            "CREATE CONSTRAINT nexus_tool_usage_id IF NOT EXISTS FOR (t:NexusToolUsage) REQUIRE t.id IS UNIQUE",
            "CREATE CONSTRAINT nexus_user_prompt_id IF NOT EXISTS FOR (p:NexusUserPrompt) REQUIRE p.id IS UNIQUE",
            "CREATE CONSTRAINT nexus_session_event_id IF NOT EXISTS FOR (e:NexusSessionEvent) REQUIRE e.id IS UNIQUE",
        ];

        for constraint in constraints {
            if let Err(e) = self.graph.run(query(constraint)).await {
                debug!("Constraint creation result: {:?}", e);
            }
        }

        // Create index for tool_name searches
        let index = "CREATE INDEX nexus_tool_usage_name IF NOT EXISTS FOR (t:NexusToolUsage) ON (t.tool_name)";
        if let Err(e) = self.graph.run(query(index)).await {
            debug!("Index creation result: {:?}", e);
        }

        info!("Neo4j hook schema initialized");
        Ok(())
    }

    /// Initialize Meilisearch index for tool usage
    pub async fn init_meilisearch_index(&self) -> Result<()> {
        if let Some(ref _ms) = self.meilisearch {
            // Create index (ignore if exists)
            // For now, we'll just log - actual index creation is handled by MeilisearchClient
            debug!("Meilisearch tool usage index ready");
        }
        Ok(())
    }

    /// Handle PreToolUse event - record start time for duration tracking
    async fn handle_pre_tool_use(
        &self,
        input: &PreToolUseHookInput,
        tool_use_id: Option<&str>,
    ) -> Result<HookJSONOutput, SdkError> {
        // Record start time for this tool use
        if let Some(id) = tool_use_id {
            self.tool_start_times.insert(id.to_string(), Instant::now());
        }

        if self.config.log_pre_tool_use {
            debug!(
                "PreToolUse: {} (session: {})",
                input.tool_name, input.session_id
            );
        }

        Ok(HookJSONOutput::Sync(SyncHookJSONOutput::default()))
    }

    /// Handle PostToolUse event - persist tool usage to Neo4j
    async fn handle_post_tool_use(
        &self,
        input: &PostToolUseHookInput,
        tool_use_id: Option<&str>,
    ) -> Result<HookJSONOutput, SdkError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        // Calculate duration if we have a start time
        let duration_ms: Option<i64> = tool_use_id.and_then(|tid| {
            self.tool_start_times
                .remove(tid)
                .map(|(_, start)| start.elapsed().as_millis() as i64)
        });

        // Serialize input/output, truncating if needed
        let input_json =
            serde_json::to_string(&input.tool_input).unwrap_or_else(|_| "{}".to_string());

        let output_json =
            serde_json::to_string(&input.tool_response).unwrap_or_else(|_| "{}".to_string());

        let output_truncated = if output_json.len() > self.config.max_output_size {
            format!(
                "{}...[truncated]",
                &output_json[..self.config.max_output_size]
            )
        } else {
            output_json.clone()
        };

        // Store in Neo4j
        let q = query(
            "CREATE (t:NexusToolUsage {
                id: $id,
                tool_name: $tool_name,
                input: $input,
                output: $output,
                duration_ms: $duration_ms,
                session_id: $session_id,
                created_at: datetime($now)
            })
            WITH t
            OPTIONAL MATCH (c:NexusConversation)
            WHERE c.id CONTAINS $session_id OR EXISTS {
                MATCH (s:NexusSession {id: $session_id})-[:HAS_CONVERSATION]->(c)
            }
            WITH t, c LIMIT 1
            FOREACH (_ IN CASE WHEN c IS NOT NULL THEN [1] ELSE [] END |
                CREATE (c)-[:HAS_TOOL_USAGE]->(t)
            )
            RETURN t.id as id",
        )
        .param("id", id.clone())
        .param("tool_name", input.tool_name.clone())
        .param("input", input_json.clone())
        .param("output", output_truncated)
        .param("duration_ms", duration_ms.unwrap_or(-1))
        .param("session_id", input.session_id.clone())
        .param("now", now.to_rfc3339());

        if let Err(e) = self.graph.run(q).await {
            warn!("Failed to store tool usage in Neo4j: {}", e);
        } else {
            debug!(
                "Stored tool usage: {} ({}ms)",
                input.tool_name,
                duration_ms.unwrap_or(-1)
            );
        }

        // Index in Meilisearch if configured
        if self.config.index_in_meilisearch
            && let Some(ref _ms) = self.meilisearch
        {
            let doc = ToolUsageDocument {
                id: id.clone(),
                tool_name: input.tool_name.clone(),
                input_summary: truncate_for_search(&input_json, 500),
                output_summary: truncate_for_search(&output_json, 1000),
                session_id: input.session_id.clone(),
                created_at: now.timestamp(),
            };

            // Use a custom index for tool usage - for now just log
            // In production, we'd create a dedicated tool_usage index
            debug!("Would index tool usage in Meilisearch: {}", doc.tool_name);
        }

        Ok(HookJSONOutput::Sync(SyncHookJSONOutput::default()))
    }

    /// Handle UserPromptSubmit event - persist user prompt to Neo4j
    async fn handle_user_prompt_submit(
        &self,
        input: &UserPromptSubmitHookInput,
    ) -> Result<HookJSONOutput, SdkError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let q = query(
            "CREATE (p:NexusUserPrompt {
                id: $id,
                prompt: $prompt,
                session_id: $session_id,
                created_at: datetime($now)
            })
            WITH p
            OPTIONAL MATCH (c:NexusConversation)
            WHERE EXISTS {
                MATCH (s:NexusSession {id: $session_id})-[:HAS_CONVERSATION]->(c)
            }
            WITH p, c LIMIT 1
            FOREACH (_ IN CASE WHEN c IS NOT NULL THEN [1] ELSE [] END |
                CREATE (c)-[:HAS_PROMPT]->(p)
            )
            RETURN p.id as id",
        )
        .param("id", id)
        .param("prompt", input.prompt.clone())
        .param("session_id", input.session_id.clone())
        .param("now", now.to_rfc3339());

        if let Err(e) = self.graph.run(q).await {
            warn!("Failed to store user prompt in Neo4j: {}", e);
        } else {
            debug!("Stored user prompt for session: {}", input.session_id);
        }

        Ok(HookJSONOutput::Sync(SyncHookJSONOutput::default()))
    }

    /// Handle Stop event - finalize session with stats
    async fn handle_stop(&self, input: &StopHookInput) -> Result<HookJSONOutput, SdkError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        // Create session stop event
        let q = query(
            "CREATE (e:NexusSessionEvent {
                id: $id,
                event_type: 'stop',
                session_id: $session_id,
                stop_hook_active: $stop_hook_active,
                created_at: datetime($now)
            })
            WITH e
            OPTIONAL MATCH (s:NexusSession {id: $session_id})
            WITH e, s
            FOREACH (_ IN CASE WHEN s IS NOT NULL THEN [1] ELSE [] END |
                CREATE (s)-[:HAS_EVENT]->(e)
            )
            // Calculate session stats
            WITH e
            OPTIONAL MATCH (t:NexusToolUsage {session_id: $session_id})
            WITH e, count(t) as tool_count, sum(COALESCE(t.duration_ms, 0)) as total_duration
            SET e.tool_count = tool_count,
                e.total_duration_ms = total_duration
            RETURN e.id as id",
        )
        .param("id", id)
        .param("session_id", input.session_id.clone())
        .param("stop_hook_active", input.stop_hook_active)
        .param("now", now.to_rfc3339());

        if let Err(e) = self.graph.run(q).await {
            warn!("Failed to store session stop event in Neo4j: {}", e);
        } else {
            info!("Session stopped: {}", input.session_id);
        }

        Ok(HookJSONOutput::Sync(SyncHookJSONOutput::default()))
    }
}

#[async_trait]
impl HookCallback for Neo4jHookCallback {
    async fn execute(
        &self,
        input: &HookInput,
        tool_use_id: Option<&str>,
        _context: &HookContext,
    ) -> Result<HookJSONOutput, SdkError> {
        match input {
            HookInput::PreToolUse(pre) => self.handle_pre_tool_use(pre, tool_use_id).await,
            HookInput::PostToolUse(post) => self.handle_post_tool_use(post, tool_use_id).await,
            HookInput::UserPromptSubmit(prompt) => self.handle_user_prompt_submit(prompt).await,
            HookInput::Stop(stop) => self.handle_stop(stop).await,
            // Other hook types - just continue
            _ => Ok(HookJSONOutput::Sync(SyncHookJSONOutput::default())),
        }
    }
}

/// Truncate a string for search indexing
fn truncate_for_search(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_for_search() {
        assert_eq!(truncate_for_search("hello", 10), "hello");
        assert_eq!(truncate_for_search("hello world", 5), "hello...");
    }

    #[test]
    fn test_config_default() {
        let config = Neo4jHookCallbackConfig::default();
        assert_eq!(config.max_output_size, 10_000);
        assert!(config.index_in_meilisearch);
        assert!(!config.log_pre_tool_use);
    }

    #[tokio::test]
    #[ignore]
    async fn test_neo4j_hook_callback_integration() {
        // This test requires a running Neo4j instance
        use crate::core::storage::Neo4jConfig;

        let config = Neo4jConfig::default();
        let graph = neo4rs::Graph::new(&config.uri, &config.user, &config.password)
            .await
            .unwrap();

        let callback =
            Neo4jHookCallback::new(Arc::new(graph), None, Neo4jHookCallbackConfig::default());

        callback.init_schema().await.unwrap();

        // Test PostToolUse handling
        let input = HookInput::PostToolUse(PostToolUseHookInput {
            session_id: "test-session".to_string(),
            transcript_path: "/tmp/transcript".to_string(),
            cwd: "/tmp".to_string(),
            permission_mode: None,
            tool_name: "Read".to_string(),
            tool_input: serde_json::json!({"file": "test.txt"}),
            tool_response: serde_json::json!({"content": "Hello, World!"}),
        });

        let context = HookContext { signal: None };
        let result = callback.execute(&input, Some("tool-123"), &context).await;

        assert!(result.is_ok());
    }
}
