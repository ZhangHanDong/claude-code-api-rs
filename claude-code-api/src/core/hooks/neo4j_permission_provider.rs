//! Neo4j-backed permission provider
//!
//! Implements the CanUseTool trait with permission rules stored in Neo4j.

#![allow(dead_code)] // Public API - may not be used internally
//!
//! ## Schema
//!
//! ```cypher
//! (:NexusPermissionRule {
//!     id: String,
//!     tool_pattern: String,    // Glob pattern (e.g., "Bash(*)", "Read", "*")
//!     decision: String,        // "allow", "deny", "ask"
//!     reason: String?,
//!     scope: String,           // "global", "workspace", "project"
//!     scope_id: String?,       // workspace_id or project_id if scoped
//!     priority: Int,           // Higher = checked first
//!     created_at: DateTime
//! })
//!
//! (:NexusPermissionAudit {
//!     id: String,
//!     tool_name: String,
//!     decision: String,
//!     rule_id: String?,
//!     session_id: String,
//!     created_at: DateTime
//! })
//!
//! // Constraints
//! CREATE CONSTRAINT nexus_permission_rule_id IF NOT EXISTS FOR (r:NexusPermissionRule) REQUIRE r.id IS UNIQUE;
//! CREATE CONSTRAINT nexus_permission_audit_id IF NOT EXISTS FOR (a:NexusPermissionAudit) REQUIRE a.id IS UNIQUE;
//! ```

use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use dashmap::DashMap;
use neo4rs::{Graph, query};
use nexus_claude::{
    CanUseTool, PermissionResult, PermissionResultAllow, PermissionResultDeny,
    ToolPermissionContext,
};
use std::sync::Arc;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Permission scope levels
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PermissionScope {
    /// Global rules apply to all projects
    Global,
    /// Workspace-level rules
    Workspace(String),
    /// Project-level rules (highest priority)
    Project(String),
}

impl PermissionScope {
    pub fn as_str(&self) -> &str {
        match self {
            PermissionScope::Global => "global",
            PermissionScope::Workspace(_) => "workspace",
            PermissionScope::Project(_) => "project",
        }
    }

    pub fn priority(&self) -> i32 {
        match self {
            PermissionScope::Global => 0,
            PermissionScope::Workspace(_) => 50,
            PermissionScope::Project(_) => 100,
        }
    }
}

/// A permission rule
#[derive(Debug, Clone)]
pub struct PermissionRule {
    pub id: String,
    pub tool_pattern: String,
    pub decision: String, // "allow", "deny", "ask"
    pub reason: Option<String>,
    pub scope: PermissionScope,
    pub priority: i32,
}

impl PermissionRule {
    /// Check if the rule matches a tool name (glob matching)
    pub fn matches(&self, tool_name: &str) -> bool {
        if self.tool_pattern == "*" {
            return true;
        }

        // Simple glob matching
        if self.tool_pattern.ends_with('*') {
            let prefix = &self.tool_pattern[..self.tool_pattern.len() - 1];
            return tool_name.starts_with(prefix);
        }

        if self.tool_pattern.starts_with('*') {
            let suffix = &self.tool_pattern[1..];
            return tool_name.ends_with(suffix);
        }

        // Handle patterns like "Bash(git:*)"
        if self.tool_pattern.contains('(') && self.tool_pattern.contains(')') {
            let base_pattern = self.tool_pattern.split('(').next().unwrap_or("");
            if tool_name.starts_with(base_pattern) {
                return true; // Simplified - full glob would be more complex
            }
        }

        self.tool_pattern == tool_name
    }
}

/// Neo4j-backed permission provider
pub struct Neo4jPermissionProvider {
    graph: Arc<Graph>,
    /// Cached rules by scope
    rules_cache: DashMap<String, Vec<PermissionRule>>,
    /// Current project scope (if any)
    project_id: Option<String>,
    /// Current workspace scope (if any)
    workspace_id: Option<String>,
    /// Session ID for audit logging
    session_id: Option<String>,
    /// Whether to log audit entries
    audit_enabled: bool,
}

impl Neo4jPermissionProvider {
    /// Create a new permission provider
    pub fn new(graph: Arc<Graph>) -> Self {
        Self {
            graph,
            rules_cache: DashMap::new(),
            project_id: None,
            workspace_id: None,
            session_id: None,
            audit_enabled: true,
        }
    }

    /// Set the project scope
    pub fn with_project(mut self, project_id: String) -> Self {
        self.project_id = Some(project_id);
        self
    }

    /// Set the workspace scope
    pub fn with_workspace(mut self, workspace_id: String) -> Self {
        self.workspace_id = Some(workspace_id);
        self
    }

    /// Set the session ID for audit logging
    pub fn with_session(mut self, session_id: String) -> Self {
        self.session_id = Some(session_id);
        self
    }

    /// Enable or disable audit logging
    pub fn with_audit(mut self, enabled: bool) -> Self {
        self.audit_enabled = enabled;
        self
    }

    /// Initialize Neo4j schema
    pub async fn init_schema(&self) -> Result<()> {
        let constraints = vec![
            "CREATE CONSTRAINT nexus_permission_rule_id IF NOT EXISTS FOR (r:NexusPermissionRule) REQUIRE r.id IS UNIQUE",
            "CREATE CONSTRAINT nexus_permission_audit_id IF NOT EXISTS FOR (a:NexusPermissionAudit) REQUIRE a.id IS UNIQUE",
        ];

        for constraint in constraints {
            if let Err(e) = self.graph.run(query(constraint)).await {
                debug!("Constraint creation result: {:?}", e);
            }
        }

        // Create index for tool pattern searches
        let index = "CREATE INDEX nexus_permission_rule_pattern IF NOT EXISTS FOR (r:NexusPermissionRule) ON (r.tool_pattern)";
        if let Err(e) = self.graph.run(query(index)).await {
            debug!("Index creation result: {:?}", e);
        }

        info!("Neo4j permission schema initialized");
        Ok(())
    }

    /// Reload rules from Neo4j
    pub async fn reload_rules(&self) -> Result<()> {
        self.rules_cache.clear();

        let q = query(
            "MATCH (r:NexusPermissionRule)
            WHERE r.scope = 'global'
               OR (r.scope = 'workspace' AND r.scope_id = $workspace_id)
               OR (r.scope = 'project' AND r.scope_id = $project_id)
            RETURN r
            ORDER BY r.priority DESC",
        )
        .param(
            "workspace_id",
            self.workspace_id.clone().unwrap_or_default(),
        )
        .param("project_id", self.project_id.clone().unwrap_or_default());

        let mut result = self.graph.execute(q).await?;
        let mut rules = Vec::new();

        while let Some(row) = result.next().await? {
            let node: neo4rs::Node = row.get("r")?;

            let scope_str: String = node.get("scope").unwrap_or_else(|_| "global".to_string());
            let scope_id: Option<String> = node.get("scope_id").ok();

            let scope = match scope_str.as_str() {
                "project" => PermissionScope::Project(scope_id.unwrap_or_default()),
                "workspace" => PermissionScope::Workspace(scope_id.unwrap_or_default()),
                _ => PermissionScope::Global,
            };

            rules.push(PermissionRule {
                id: node.get("id")?,
                tool_pattern: node.get("tool_pattern")?,
                decision: node.get("decision")?,
                reason: node.get("reason").ok(),
                scope,
                priority: node.get("priority").unwrap_or(0),
            });
        }

        self.rules_cache.insert("all".to_string(), rules);
        debug!(
            "Loaded {} permission rules",
            self.rules_cache.get("all").map(|r| r.len()).unwrap_or(0)
        );

        Ok(())
    }

    /// Add a permission rule
    pub async fn add_rule(&self, rule: PermissionRule) -> Result<()> {
        let now = Utc::now();

        let (scope_id, scope_str) = match &rule.scope {
            PermissionScope::Global => (None, "global"),
            PermissionScope::Workspace(id) => (Some(id.clone()), "workspace"),
            PermissionScope::Project(id) => (Some(id.clone()), "project"),
        };

        let q = query(
            "CREATE (r:NexusPermissionRule {
                id: $id,
                tool_pattern: $tool_pattern,
                decision: $decision,
                reason: $reason,
                scope: $scope,
                scope_id: $scope_id,
                priority: $priority,
                created_at: datetime($now)
            })
            RETURN r.id as id",
        )
        .param("id", rule.id.clone())
        .param("tool_pattern", rule.tool_pattern)
        .param("decision", rule.decision)
        .param("reason", rule.reason.unwrap_or_default())
        .param("scope", scope_str)
        .param("scope_id", scope_id.unwrap_or_default())
        .param("priority", rule.priority)
        .param("now", now.to_rfc3339());

        self.graph.run(q).await?;
        info!("Added permission rule: {}", rule.id);

        // Invalidate cache
        self.rules_cache.clear();

        Ok(())
    }

    /// Remove a permission rule
    pub async fn remove_rule(&self, rule_id: &str) -> Result<bool> {
        let q = query(
            "MATCH (r:NexusPermissionRule {id: $id})
            DELETE r
            RETURN count(r) as deleted",
        )
        .param("id", rule_id);

        let mut result = self.graph.execute(q).await?;

        if let Some(row) = result.next().await? {
            let deleted: i64 = row.get("deleted")?;

            if deleted > 0 {
                info!("Removed permission rule: {}", rule_id);
                self.rules_cache.clear();
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// List rules for a scope
    pub async fn list_rules(&self, scope: Option<PermissionScope>) -> Result<Vec<PermissionRule>> {
        let q = match scope {
            Some(ref s) => {
                let scope_str = s.as_str();
                let scope_id = match s {
                    PermissionScope::Global => String::new(),
                    PermissionScope::Workspace(id) | PermissionScope::Project(id) => id.clone(),
                };

                query(
                    "MATCH (r:NexusPermissionRule)
                    WHERE r.scope = $scope AND (r.scope_id = $scope_id OR r.scope_id IS NULL)
                    RETURN r
                    ORDER BY r.priority DESC",
                )
                .param("scope", scope_str)
                .param("scope_id", scope_id)
            },
            None => query(
                "MATCH (r:NexusPermissionRule)
                RETURN r
                ORDER BY r.priority DESC",
            ),
        };

        let mut result = self.graph.execute(q).await?;
        let mut rules = Vec::new();

        while let Some(row) = result.next().await? {
            let node: neo4rs::Node = row.get("r")?;

            let scope_str: String = node.get("scope").unwrap_or_else(|_| "global".to_string());
            let scope_id: Option<String> = node.get("scope_id").ok();

            let scope = match scope_str.as_str() {
                "project" => PermissionScope::Project(scope_id.unwrap_or_default()),
                "workspace" => PermissionScope::Workspace(scope_id.unwrap_or_default()),
                _ => PermissionScope::Global,
            };

            rules.push(PermissionRule {
                id: node.get("id")?,
                tool_pattern: node.get("tool_pattern")?,
                decision: node.get("decision")?,
                reason: node.get("reason").ok(),
                scope,
                priority: node.get("priority").unwrap_or(0),
            });
        }

        Ok(rules)
    }

    /// Log an audit entry
    async fn log_audit(&self, tool_name: &str, decision: &str, rule_id: Option<&str>) {
        if !self.audit_enabled {
            return;
        }

        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let q = query(
            "CREATE (a:NexusPermissionAudit {
                id: $id,
                tool_name: $tool_name,
                decision: $decision,
                rule_id: $rule_id,
                session_id: $session_id,
                created_at: datetime($now)
            })",
        )
        .param("id", id)
        .param("tool_name", tool_name)
        .param("decision", decision)
        .param("rule_id", rule_id.unwrap_or(""))
        .param("session_id", self.session_id.clone().unwrap_or_default())
        .param("now", now.to_rfc3339());

        if let Err(e) = self.graph.run(q).await {
            warn!("Failed to log permission audit: {}", e);
        }
    }

    /// Find the matching rule for a tool
    fn find_matching_rule(&self, tool_name: &str) -> Option<PermissionRule> {
        let rules = self.rules_cache.get("all")?;

        // Rules are already sorted by priority DESC
        for rule in rules.iter() {
            if rule.matches(tool_name) {
                return Some(rule.clone());
            }
        }

        None
    }
}

#[async_trait]
impl CanUseTool for Neo4jPermissionProvider {
    async fn can_use_tool(
        &self,
        tool_name: &str,
        _input: &serde_json::Value,
        _context: &ToolPermissionContext,
    ) -> PermissionResult {
        // Find matching rule
        if let Some(rule) = self.find_matching_rule(tool_name) {
            let decision = rule.decision.as_str();

            // Log audit entry
            self.log_audit(tool_name, decision, Some(&rule.id)).await;

            match decision {
                "allow" => {
                    debug!("Allowing tool {} (rule: {})", tool_name, rule.id);
                    PermissionResult::Allow(PermissionResultAllow {
                        updated_input: None,
                        updated_permissions: None,
                    })
                },
                "deny" => {
                    debug!("Denying tool {} (rule: {})", tool_name, rule.id);
                    PermissionResult::Deny(PermissionResultDeny {
                        message: rule.reason.unwrap_or_else(|| {
                            format!("Tool '{}' is denied by permission rule", tool_name)
                        }),
                        interrupt: false,
                    })
                },
                _ => {
                    // "ask" or unknown - default to allow (let SDK handle asking)
                    PermissionResult::Allow(PermissionResultAllow {
                        updated_input: None,
                        updated_permissions: None,
                    })
                },
            }
        } else {
            // No matching rule - default to allow
            self.log_audit(tool_name, "allow_default", None).await;

            PermissionResult::Allow(PermissionResultAllow {
                updated_input: None,
                updated_permissions: None,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_rule_matches() {
        let rule = PermissionRule {
            id: "test".to_string(),
            tool_pattern: "Bash*".to_string(),
            decision: "allow".to_string(),
            reason: None,
            scope: PermissionScope::Global,
            priority: 0,
        };

        assert!(rule.matches("Bash"));
        assert!(rule.matches("BashCommand"));
        assert!(!rule.matches("Read"));
    }

    #[test]
    fn test_permission_rule_matches_wildcard() {
        let rule = PermissionRule {
            id: "test".to_string(),
            tool_pattern: "*".to_string(),
            decision: "allow".to_string(),
            reason: None,
            scope: PermissionScope::Global,
            priority: 0,
        };

        assert!(rule.matches("Bash"));
        assert!(rule.matches("Read"));
        assert!(rule.matches("AnyTool"));
    }

    #[test]
    fn test_permission_rule_matches_exact() {
        let rule = PermissionRule {
            id: "test".to_string(),
            tool_pattern: "Read".to_string(),
            decision: "allow".to_string(),
            reason: None,
            scope: PermissionScope::Global,
            priority: 0,
        };

        assert!(rule.matches("Read"));
        assert!(!rule.matches("ReadFile"));
        assert!(!rule.matches("Write"));
    }

    #[test]
    fn test_permission_scope_priority() {
        assert!(
            PermissionScope::Project("p1".to_string()).priority()
                > PermissionScope::Workspace("w1".to_string()).priority()
        );
        assert!(
            PermissionScope::Workspace("w1".to_string()).priority()
                > PermissionScope::Global.priority()
        );
    }
}
