//! Neo4j storage implementations
//!
//! This module provides Neo4j-backed implementations of the storage traits.
//! Labels are prefixed with "Nexus" to avoid conflicts with other applications.

#![allow(dead_code)] // Public API - may not be used internally
//!
//! ## Schema
//!
//! ```cypher
//! // Nodes
//! (:NexusSession {
//!     id: String,
//!     project_path: String?,
//!     cli_session_id: String?,  // For --resume support
//!     created_at: DateTime,
//!     updated_at: DateTime
//! })
//!
//! (:NexusConversation {
//!     id: String,
//!     model: String?,
//!     total_tokens: Int,
//!     turn_count: Int,
//!     created_at: DateTime,
//!     updated_at: DateTime
//! })
//!
//! (:NexusMessage {
//!     id: String,
//!     role: String,
//!     content: String,
//!     turn_index: Int,
//!     created_at: DateTime
//! })
//!
//! // Relationships
//! (:NexusSession)-[:HAS_CONVERSATION]->(:NexusConversation)
//! (:NexusConversation)-[:HAS_MESSAGE]->(:NexusMessage)
//! (:NexusMessage)-[:NEXT]->(:NexusMessage)  // Message ordering
//!
//! // Constraints
//! CREATE CONSTRAINT nexus_session_id IF NOT EXISTS FOR (s:NexusSession) REQUIRE s.id IS UNIQUE;
//! CREATE CONSTRAINT nexus_conversation_id IF NOT EXISTS FOR (c:NexusConversation) REQUIRE c.id IS UNIQUE;
//! CREATE CONSTRAINT nexus_message_id IF NOT EXISTS FOR (m:NexusMessage) REQUIRE m.id IS UNIQUE;
//! ```

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use neo4rs::{Graph, Node, query};
use std::sync::Arc;
use tracing::{debug, info};
use uuid::Uuid;

use crate::core::conversation::{Conversation, ConversationMetadata};
use crate::core::session_manager::Session;
use crate::models::openai::{ChatMessage, MessageContent};

use super::traits::{ConversationStore, SessionStore};

/// Configuration for Neo4j connection
#[derive(Clone, Debug)]
pub struct Neo4jConfig {
    pub uri: String,
    pub user: String,
    pub password: String,
    pub max_connections: usize,
}

impl Default for Neo4jConfig {
    fn default() -> Self {
        Self {
            uri: std::env::var("NEO4J_URI").unwrap_or_else(|_| "bolt://localhost:7687".to_string()),
            user: std::env::var("NEO4J_USER").unwrap_or_else(|_| "neo4j".to_string()),
            password: std::env::var("NEO4J_PASSWORD").unwrap_or_else(|_| "password".to_string()),
            max_connections: 10,
        }
    }
}

/// Neo4j client wrapper
#[derive(Clone)]
pub struct Neo4jClient {
    graph: Arc<Graph>,
}

impl Neo4jClient {
    /// Create a new Neo4j client
    pub async fn new(config: Neo4jConfig) -> Result<Self> {
        info!("Connecting to Neo4j at {}", config.uri);

        let graph = Graph::new(&config.uri, &config.user, &config.password).await?;

        let client = Self {
            graph: Arc::new(graph),
        };

        // Initialize schema
        client.init_schema().await?;

        info!("Connected to Neo4j successfully");
        Ok(client)
    }

    /// Initialize Neo4j schema with constraints
    async fn init_schema(&self) -> Result<()> {
        let constraints = vec![
            "CREATE CONSTRAINT nexus_session_id IF NOT EXISTS FOR (s:NexusSession) REQUIRE s.id IS UNIQUE",
            "CREATE CONSTRAINT nexus_conversation_id IF NOT EXISTS FOR (c:NexusConversation) REQUIRE c.id IS UNIQUE",
            "CREATE CONSTRAINT nexus_message_id IF NOT EXISTS FOR (m:NexusMessage) REQUIRE m.id IS UNIQUE",
        ];

        for constraint in constraints {
            if let Err(e) = self.graph.run(query(constraint)).await {
                // Constraint might already exist, that's OK
                debug!("Constraint creation result: {:?}", e);
            }
        }

        info!("Neo4j schema initialized for Nexus");
        Ok(())
    }

    /// Get the underlying graph for direct queries
    pub fn graph(&self) -> &Graph {
        &self.graph
    }
}

// ============================================================================
// Neo4jConversationStore
// ============================================================================

/// Neo4j-backed implementation of ConversationStore
pub struct Neo4jConversationStore {
    client: Neo4jClient,
}

impl Neo4jConversationStore {
    pub fn new(client: Neo4jClient) -> Self {
        Self { client }
    }
}

#[async_trait]
impl ConversationStore for Neo4jConversationStore {
    async fn create(&self, model: Option<String>) -> Result<String> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        let q = query(
            "CREATE (c:NexusConversation {
                id: $id,
                model: $model,
                total_tokens: 0,
                turn_count: 0,
                created_at: datetime($now),
                updated_at: datetime($now)
            })
            RETURN c.id as id",
        )
        .param("id", id.clone())
        .param("model", model.unwrap_or_default())
        .param("now", now);

        self.client.graph.run(q).await?;

        info!("Created Neo4j conversation: {}", id);
        Ok(id)
    }

    async fn get(&self, id: &str) -> Result<Option<Conversation>> {
        let q = query(
            "MATCH (c:NexusConversation {id: $id})
            OPTIONAL MATCH (c)-[:HAS_MESSAGE]->(m:NexusMessage)
            WITH c, m ORDER BY m.turn_index
            WITH c, collect(m) as messages
            RETURN c, messages",
        )
        .param("id", id);

        let mut result = self.client.graph.execute(q).await?;

        if let Some(row) = result.next().await? {
            let conv_node: Node = row.get("c")?;
            let messages_nodes: Vec<Node> = row.get("messages")?;

            let messages: Vec<ChatMessage> = messages_nodes
                .into_iter()
                .filter_map(|m| {
                    let role: String = m.get("role").ok()?;
                    let content: String = m.get("content").ok()?;
                    Some(ChatMessage {
                        role,
                        content: Some(MessageContent::Text(content)),
                        name: None,
                        tool_calls: None,
                    })
                })
                .collect();

            let model: Option<String> = conv_node.get("model").ok();
            let total_tokens: i64 = conv_node.get("total_tokens").unwrap_or(0);
            let turn_count: i64 = conv_node.get("turn_count").unwrap_or(0);

            // Parse datetime strings
            let created_at = parse_neo4j_datetime(&conv_node, "created_at")?;
            let updated_at = parse_neo4j_datetime(&conv_node, "updated_at")?;

            return Ok(Some(Conversation {
                id: id.to_string(),
                messages,
                created_at,
                updated_at,
                metadata: ConversationMetadata {
                    model,
                    total_tokens: total_tokens as usize,
                    turn_count: turn_count as usize,
                    project_path: None,
                },
            }));
        }

        Ok(None)
    }

    async fn add_message(&self, id: &str, message: ChatMessage) -> Result<()> {
        let msg_id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        let content = match &message.content {
            Some(MessageContent::Text(text)) => text.clone(),
            Some(MessageContent::Array(parts)) => {
                // Combine text parts
                parts
                    .iter()
                    .filter_map(|p| match p {
                        crate::models::openai::ContentPart::Text { text } => Some(text.clone()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            },
            None => String::new(),
        };

        let q = query(
            "MATCH (c:NexusConversation {id: $conv_id})
            CREATE (m:NexusMessage {
                id: $msg_id,
                role: $role,
                content: $content,
                turn_index: c.turn_count,
                created_at: datetime($now)
            })
            CREATE (c)-[:HAS_MESSAGE]->(m)
            SET c.turn_count = c.turn_count + 1,
                c.updated_at = datetime($now)
            RETURN c.id as id",
        )
        .param("conv_id", id)
        .param("msg_id", msg_id)
        .param("role", message.role)
        .param("content", content)
        .param("now", now);

        let mut result = self.client.graph.execute(q).await?;

        if result.next().await?.is_none() {
            return Err(anyhow::anyhow!("Conversation not found: {}", id));
        }

        debug!("Added message to conversation {}", id);
        Ok(())
    }

    async fn update_metadata(&self, id: &str, metadata: ConversationMetadata) -> Result<()> {
        let now = Utc::now().to_rfc3339();

        let q = query(
            "MATCH (c:NexusConversation {id: $id})
            SET c.model = $model,
                c.total_tokens = $total_tokens,
                c.turn_count = $turn_count,
                c.updated_at = datetime($now)
            RETURN c.id as id",
        )
        .param("id", id)
        .param("model", metadata.model.unwrap_or_default())
        .param("total_tokens", metadata.total_tokens as i64)
        .param("turn_count", metadata.turn_count as i64)
        .param("now", now);

        let mut result = self.client.graph.execute(q).await?;

        if result.next().await?.is_none() {
            return Err(anyhow::anyhow!("Conversation not found: {}", id));
        }

        Ok(())
    }

    async fn list_active(&self) -> Result<Vec<(String, DateTime<Utc>)>> {
        let q = query(
            "MATCH (c:NexusConversation)
            RETURN c.id as id, c.updated_at as updated_at
            ORDER BY c.updated_at DESC
            LIMIT 100",
        );

        let mut result = self.client.graph.execute(q).await?;
        let mut conversations = Vec::new();

        while let Some(row) = result.next().await? {
            let id: String = row.get("id")?;
            // Parse datetime - Neo4j returns ISO format string
            let updated_str: String = row.get("updated_at")?;
            if let Ok(updated_at) = DateTime::parse_from_rfc3339(&updated_str) {
                conversations.push((id, updated_at.with_timezone(&Utc)));
            }
        }

        Ok(conversations)
    }

    async fn cleanup_expired(&self, timeout_minutes: i64) -> Result<usize> {
        let q = query(
            "MATCH (c:NexusConversation)
            WHERE c.updated_at < datetime() - duration({minutes: $timeout})
            OPTIONAL MATCH (c)-[:HAS_MESSAGE]->(m:NexusMessage)
            DETACH DELETE c, m
            RETURN count(c) as deleted",
        )
        .param("timeout", timeout_minutes);

        let mut result = self.client.graph.execute(q).await?;

        if let Some(row) = result.next().await? {
            let deleted: i64 = row.get("deleted")?;
            if deleted > 0 {
                info!("Cleaned up {} expired conversations", deleted);
            }
            return Ok(deleted as usize);
        }

        Ok(0)
    }

    async fn delete(&self, id: &str) -> Result<bool> {
        let q = query(
            "MATCH (c:NexusConversation {id: $id})
            OPTIONAL MATCH (c)-[:HAS_MESSAGE]->(m:NexusMessage)
            DETACH DELETE c, m
            RETURN count(c) as deleted",
        )
        .param("id", id);

        let mut result = self.client.graph.execute(q).await?;

        if let Some(row) = result.next().await? {
            let deleted: i64 = row.get("deleted")?;
            return Ok(deleted > 0);
        }

        Ok(false)
    }
}

// ============================================================================
// Neo4jSessionStore
// ============================================================================

/// Neo4j-backed implementation of SessionStore
pub struct Neo4jSessionStore {
    client: Neo4jClient,
}

impl Neo4jSessionStore {
    pub fn new(client: Neo4jClient) -> Self {
        Self { client }
    }
}

#[async_trait]
impl SessionStore for Neo4jSessionStore {
    async fn create(&self, project_path: Option<String>) -> Result<String> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        let q = query(
            "CREATE (s:NexusSession {
                id: $id,
                project_path: $project_path,
                created_at: datetime($now),
                updated_at: datetime($now)
            })
            RETURN s.id as id",
        )
        .param("id", id.clone())
        .param("project_path", project_path.unwrap_or_default())
        .param("now", now);

        self.client.graph.run(q).await?;

        info!("Created Neo4j session: {}", id);
        Ok(id)
    }

    async fn get(&self, id: &str) -> Result<Option<Session>> {
        let q = query(
            "MATCH (s:NexusSession {id: $id})
            RETURN s",
        )
        .param("id", id);

        let mut result = self.client.graph.execute(q).await?;

        if let Some(row) = result.next().await? {
            let node: Node = row.get("s")?;

            let project_path: Option<String> = node.get("project_path").ok();
            let created_at = parse_neo4j_datetime(&node, "created_at")?;
            let updated_at = parse_neo4j_datetime(&node, "updated_at")?;

            return Ok(Some(Session {
                id: id.to_string(),
                project_path,
                created_at,
                updated_at,
            }));
        }

        Ok(None)
    }

    async fn update(&self, id: &str) -> Result<()> {
        let now = Utc::now().to_rfc3339();

        let q = query(
            "MATCH (s:NexusSession {id: $id})
            SET s.updated_at = datetime($now)
            RETURN s.id as id",
        )
        .param("id", id)
        .param("now", now);

        let mut result = self.client.graph.execute(q).await?;

        if result.next().await?.is_none() {
            return Err(anyhow::anyhow!("Session not found: {}", id));
        }

        Ok(())
    }

    async fn remove(&self, id: &str) -> Result<Option<Session>> {
        // First get the session
        let session = self.get(id).await?;

        if session.is_some() {
            let q = query(
                "MATCH (s:NexusSession {id: $id})
                DETACH DELETE s",
            )
            .param("id", id);

            self.client.graph.run(q).await?;
            info!("Removed Neo4j session: {}", id);
        }

        Ok(session)
    }

    async fn list(&self) -> Result<Vec<Session>> {
        let q = query(
            "MATCH (s:NexusSession)
            RETURN s
            ORDER BY s.updated_at DESC
            LIMIT 100",
        );

        let mut result = self.client.graph.execute(q).await?;
        let mut sessions = Vec::new();

        while let Some(row) = result.next().await? {
            let node: Node = row.get("s")?;

            let id: String = node.get("id")?;
            let project_path: Option<String> = node.get("project_path").ok();
            let created_at =
                parse_neo4j_datetime(&node, "created_at").unwrap_or_else(|_| Utc::now());
            let updated_at =
                parse_neo4j_datetime(&node, "updated_at").unwrap_or_else(|_| Utc::now());

            sessions.push(Session {
                id,
                project_path,
                created_at,
                updated_at,
            });
        }

        Ok(sessions)
    }
}

// ============================================================================
// Helper functions
// ============================================================================

fn parse_neo4j_datetime(node: &Node, field: &str) -> Result<DateTime<Utc>> {
    // Neo4j datetime is returned as a string in ISO format
    let dt_str: String = node.get(field)?;
    let dt = DateTime::parse_from_rfc3339(&dt_str)?;
    Ok(dt.with_timezone(&Utc))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Integration tests require a running Neo4j instance
    // Run with: cargo test --features integration -- --ignored

    #[tokio::test]
    #[ignore]
    async fn test_neo4j_conversation_create() {
        let config = Neo4jConfig::default();
        let client = Neo4jClient::new(config).await.unwrap();
        let store = Neo4jConversationStore::new(client);

        let id = store.create(Some("claude-3".to_string())).await.unwrap();
        assert!(!id.is_empty());

        let conv = store.get(&id).await.unwrap();
        assert!(conv.is_some());

        // Cleanup
        store.delete(&id).await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn test_neo4j_session_create() {
        let config = Neo4jConfig::default();
        let client = Neo4jClient::new(config).await.unwrap();
        let store = Neo4jSessionStore::new(client);

        let id = store
            .create(Some("/path/to/project".to_string()))
            .await
            .unwrap();
        assert!(!id.is_empty());

        let session = store.get(&id).await.unwrap();
        assert!(session.is_some());

        // Cleanup
        store.remove(&id).await.unwrap();
    }
}
