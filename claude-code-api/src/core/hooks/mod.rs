//! Hook implementations for Claude Code API
//!
//! This module provides implementations of the SDK's hook traits for
//! capturing conversation events and storing them in Neo4j.
//!
//! ## Available Hooks
//!
//! - `Neo4jHookCallback`: Captures tool usage, prompts, and session events in Neo4j
//! - `Neo4jPermissionProvider`: Permission rules stored in Neo4j graph

mod neo4j_hook_callback;
mod neo4j_permission_provider;

// Re-export for public API
#[allow(unused_imports)]
pub use neo4j_hook_callback::{Neo4jHookCallback, Neo4jHookCallbackConfig};
#[allow(unused_imports)]
pub use neo4j_permission_provider::{Neo4jPermissionProvider, PermissionRule, PermissionScope};
