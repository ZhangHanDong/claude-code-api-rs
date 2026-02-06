//! Medium-term memory: plans, tasks, decisions from project-orchestrator
//!
//! Queries the project-orchestrator MCP server for:
//! - Active plans and their tasks
//! - Architectural decisions
//! - Knowledge notes attached to entities

#![allow(dead_code)] // Public API - may not be used internally

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use super::traits::{ContextualMemoryProvider, MemoryResult, MemorySource, RelevanceScore};

/// Configuration for MCP connection
#[derive(Debug, Clone)]
pub struct McpConfig {
    /// MCP server URL (e.g., http://localhost:8080)
    pub url: String,
    /// Optional API key
    pub api_key: Option<String>,
}

impl Default for McpConfig {
    fn default() -> Self {
        Self {
            url: std::env::var("PROJECT_ORCHESTRATOR_URL")
                .unwrap_or_else(|_| "http://localhost:8080".to_string()),
            api_key: std::env::var("PROJECT_ORCHESTRATOR_KEY").ok(),
        }
    }
}

/// Response types from project-orchestrator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanSummary {
    pub id: String,
    pub title: String,
    pub description: String,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSummary {
    pub id: String,
    pub title: String,
    pub description: String,
    pub status: String,
    pub plan_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionSummary {
    pub id: String,
    pub description: String,
    pub rationale: String,
    pub chosen_option: Option<String>,
    pub task_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteSummary {
    pub id: String,
    pub content: String,
    pub note_type: String,
    pub importance: String,
    pub project_id: Option<String>,
}

/// Medium-term memory backed by project-orchestrator MCP
pub struct MediumTermMemory {
    client: reqwest::Client,
    config: McpConfig,
    project_id: Option<String>,
    scope: Option<String>,
}

impl MediumTermMemory {
    /// Create a new medium-term memory provider
    pub fn new(config: McpConfig) -> Self {
        Self {
            client: reqwest::Client::new(),
            config,
            project_id: None,
            scope: None,
        }
    }

    /// Set the current project ID
    pub fn with_project(mut self, project_id: String) -> Self {
        self.project_id = Some(project_id);
        self
    }

    /// Set the project ID
    pub fn set_project(&mut self, project_id: Option<String>) {
        self.project_id = project_id;
    }

    /// Search plans
    async fn search_plans(&self, query: &str, limit: usize) -> Result<Vec<PlanSummary>> {
        let url = format!("{}/plans", self.config.url);

        let mut request = self
            .client
            .get(&url)
            .query(&[("search", query), ("limit", &limit.to_string())]);

        if let Some(ref key) = self.config.api_key {
            request = request.header("Authorization", format!("Bearer {}", key));
        }

        if let Some(ref project_id) = self.project_id {
            request = request.query(&[("project_id", project_id)]);
        }

        match request.send().await {
            Ok(response) => {
                if response.status().is_success() {
                    let data: serde_json::Value = response.json().await?;
                    if let Some(plans) = data.get("plans").and_then(|p| p.as_array()) {
                        return Ok(plans
                            .iter()
                            .filter_map(|p| serde_json::from_value(p.clone()).ok())
                            .collect());
                    }
                }
                Ok(vec![])
            },
            Err(e) => {
                warn!("Failed to search plans: {}", e);
                Ok(vec![])
            },
        }
    }

    /// Search tasks
    async fn search_tasks(&self, query: &str, limit: usize) -> Result<Vec<TaskSummary>> {
        // For now, use a simple approach - in production this would call the MCP
        let url = format!("{}/tasks", self.config.url);

        let request = self
            .client
            .get(&url)
            .query(&[("search", query), ("limit", &limit.to_string())]);

        match request.send().await {
            Ok(response) => {
                if response.status().is_success() {
                    let data: serde_json::Value = response.json().await?;
                    if let Some(tasks) = data.get("tasks").and_then(|t| t.as_array()) {
                        return Ok(tasks
                            .iter()
                            .filter_map(|t| serde_json::from_value(t.clone()).ok())
                            .collect());
                    }
                }
                Ok(vec![])
            },
            Err(e) => {
                warn!("Failed to search tasks: {}", e);
                Ok(vec![])
            },
        }
    }

    /// Search decisions
    async fn search_decisions(&self, query: &str, limit: usize) -> Result<Vec<DecisionSummary>> {
        let url = format!("{}/decisions/search", self.config.url);

        let request = self
            .client
            .get(&url)
            .query(&[("query", query), ("limit", &limit.to_string())]);

        match request.send().await {
            Ok(response) => {
                if response.status().is_success() {
                    let data: serde_json::Value = response.json().await?;
                    if let Some(decisions) = data.get("decisions").and_then(|d| d.as_array()) {
                        return Ok(decisions
                            .iter()
                            .filter_map(|d| serde_json::from_value(d.clone()).ok())
                            .collect());
                    }
                }
                Ok(vec![])
            },
            Err(e) => {
                warn!("Failed to search decisions: {}", e);
                Ok(vec![])
            },
        }
    }

    /// Search notes
    async fn search_notes(&self, query: &str, limit: usize) -> Result<Vec<NoteSummary>> {
        let url = format!("{}/notes/search", self.config.url);

        let mut request = self
            .client
            .get(&url)
            .query(&[("query", query), ("limit", &limit.to_string())]);

        if let Some(ref project_id) = self.project_id {
            request = request.query(&[("project_id", project_id)]);
        }

        match request.send().await {
            Ok(response) => {
                if response.status().is_success() {
                    let data: serde_json::Value = response.json().await?;
                    if let Some(notes) = data.get("notes").and_then(|n| n.as_array()) {
                        return Ok(notes
                            .iter()
                            .filter_map(|n| serde_json::from_value(n.clone()).ok())
                            .collect());
                    }
                }
                Ok(vec![])
            },
            Err(e) => {
                warn!("Failed to search notes: {}", e);
                Ok(vec![])
            },
        }
    }

    /// Calculate a simple relevance score
    fn calculate_score(&self, query: &str, content: &str, entity_type: &str) -> RelevanceScore {
        let query_lower = query.to_lowercase();
        let query_words: Vec<&str> = query_lower.split_whitespace().collect();
        let content_lower = content.to_lowercase();

        let semantic = if query_words.is_empty() {
            0.0
        } else {
            let matches = query_words
                .iter()
                .filter(|word| content_lower.contains(*word))
                .count();
            matches as f64 / query_words.len() as f64
        };

        // Recency is less important for medium-term (plans persist)
        let recency = 0.5;

        // Scope based on entity type
        let scope = match entity_type {
            "decision" => 0.9, // Decisions are most specific
            "task" => 0.8,
            "plan" => 0.7,
            "note" => 0.6,
            _ => 0.5,
        };

        RelevanceScore::new(semantic, recency, scope)
    }
}

#[async_trait]
impl ContextualMemoryProvider for MediumTermMemory {
    async fn query(&self, query: &str, limit: usize) -> Result<Vec<MemoryResult>> {
        let mut results = Vec::new();

        // Search all entity types in parallel would be better, but for simplicity:
        let per_type_limit = (limit / 4).max(2);

        // Search plans
        for plan in self.search_plans(query, per_type_limit).await? {
            let content = format!("{}\n\n{}", plan.title, plan.description);
            let score = self.calculate_score(query, &content, "plan");

            let timestamp = DateTime::parse_from_rfc3339(&plan.created_at)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            results.push(
                MemoryResult::new(
                    plan.id.clone(),
                    MemorySource::ProjectOrchestrator {
                        entity_type: "plan".to_string(),
                        entity_id: plan.id,
                    },
                    content,
                    score,
                    timestamp,
                )
                .with_title(plan.title)
                .with_metadata(serde_json::json!({ "status": plan.status })),
            );
        }

        // Search tasks
        for task in self.search_tasks(query, per_type_limit).await? {
            let content = format!("{}\n\n{}", task.title, task.description);
            let score = self.calculate_score(query, &content, "task");

            results.push(
                MemoryResult::new(
                    task.id.clone(),
                    MemorySource::ProjectOrchestrator {
                        entity_type: "task".to_string(),
                        entity_id: task.id,
                    },
                    content,
                    score,
                    Utc::now(),
                )
                .with_title(task.title)
                .with_metadata(serde_json::json!({
                    "status": task.status,
                    "plan_id": task.plan_id,
                })),
            );
        }

        // Search decisions
        for decision in self.search_decisions(query, per_type_limit).await? {
            let content = format!(
                "{}\n\nRationale: {}\nChosen: {}",
                decision.description,
                decision.rationale,
                decision.chosen_option.as_deref().unwrap_or("N/A")
            );
            let score = self.calculate_score(query, &content, "decision");

            results.push(
                MemoryResult::new(
                    decision.id.clone(),
                    MemorySource::ProjectOrchestrator {
                        entity_type: "decision".to_string(),
                        entity_id: decision.id,
                    },
                    content,
                    score,
                    Utc::now(),
                )
                .with_metadata(serde_json::json!({ "task_id": decision.task_id })),
            );
        }

        // Search notes
        for note in self.search_notes(query, per_type_limit).await? {
            let score = self.calculate_score(query, &note.content, "note");

            results.push(
                MemoryResult::new(
                    note.id.clone(),
                    MemorySource::KnowledgeNote {
                        note_id: note.id,
                        project_id: note.project_id,
                    },
                    note.content,
                    score,
                    Utc::now(),
                )
                .with_metadata(serde_json::json!({
                    "note_type": note.note_type,
                    "importance": note.importance,
                })),
            );
        }

        // Sort by combined score
        results.sort_by(|a, b| {
            b.score
                .combined
                .partial_cmp(&a.score.combined)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        results.truncate(limit);
        debug!(
            "MediumTermMemory: found {} results for query",
            results.len()
        );

        Ok(results)
    }

    async fn search_context(
        &self,
        query: &str,
        source_filter: Option<&str>,
        limit: usize,
    ) -> Result<Vec<MemoryResult>> {
        match source_filter {
            Some("plan") => {
                let plans = self.search_plans(query, limit).await?;
                Ok(plans
                    .into_iter()
                    .map(|p| {
                        let content = format!("{}\n\n{}", p.title, p.description);
                        let score = self.calculate_score(query, &content, "plan");
                        let timestamp = DateTime::parse_from_rfc3339(&p.created_at)
                            .map(|dt| dt.with_timezone(&Utc))
                            .unwrap_or_else(|_| Utc::now());

                        MemoryResult::new(
                            p.id.clone(),
                            MemorySource::ProjectOrchestrator {
                                entity_type: "plan".to_string(),
                                entity_id: p.id,
                            },
                            content,
                            score,
                            timestamp,
                        )
                        .with_title(p.title)
                    })
                    .collect())
            },
            Some("task") => {
                let tasks = self.search_tasks(query, limit).await?;
                Ok(tasks
                    .into_iter()
                    .map(|t| {
                        let content = format!("{}\n\n{}", t.title, t.description);
                        let score = self.calculate_score(query, &content, "task");

                        MemoryResult::new(
                            t.id.clone(),
                            MemorySource::ProjectOrchestrator {
                                entity_type: "task".to_string(),
                                entity_id: t.id,
                            },
                            content,
                            score,
                            Utc::now(),
                        )
                        .with_title(t.title)
                    })
                    .collect())
            },
            Some("decision") => self.get_relevant_decisions(query, limit).await,
            _ => self.query(query, limit).await,
        }
    }

    async fn get_relevant_decisions(&self, topic: &str, limit: usize) -> Result<Vec<MemoryResult>> {
        let decisions = self.search_decisions(topic, limit).await?;

        Ok(decisions
            .into_iter()
            .map(|d| {
                let content = format!(
                    "{}\n\nRationale: {}\nChosen: {}",
                    d.description,
                    d.rationale,
                    d.chosen_option.as_deref().unwrap_or("N/A")
                );
                let score = self.calculate_score(topic, &content, "decision");

                MemoryResult::new(
                    d.id.clone(),
                    MemorySource::ProjectOrchestrator {
                        entity_type: "decision".to_string(),
                        entity_id: d.id,
                    },
                    content,
                    score,
                    Utc::now(),
                )
            })
            .collect())
    }

    fn current_scope(&self) -> Option<String> {
        self.scope.clone()
    }

    fn set_scope(&mut self, scope: Option<String>) {
        self.scope = scope;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = McpConfig::default();
        assert!(config.url.contains("localhost"));
    }

    #[test]
    fn test_calculate_score() {
        let memory = MediumTermMemory::new(McpConfig::default());

        let score = memory.calculate_score(
            "authentication",
            "How to implement JWT authentication",
            "decision",
        );

        assert!(score.semantic > 0.0);
        assert_eq!(score.scope, 0.9); // decision has highest scope
    }
}
