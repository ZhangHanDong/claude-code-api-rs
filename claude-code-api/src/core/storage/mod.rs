//! Storage abstractions for Claude Code API
//!
//! This module defines traits for different storage backends, allowing
//! the API to work with in-memory, Neo4j, or other storage implementations.
//!
//! ## Available Backends
//!
//! - `memory`: In-memory storage using HashMap/DashMap (default)
//! - `neo4j`: Neo4j graph database storage
//! - `meilisearch`: Meilisearch for full-text search

pub mod combined;
pub mod meilisearch;
mod memory;
pub mod neo4j;
pub mod tiered_cache;
mod traits;

// Re-export for public API
#[allow(unused_imports)]
pub use combined::{CombinedConversationStore, CombinedSessionStore};
#[allow(unused_imports)]
pub use meilisearch::{
    ConversationDocument, MeilisearchClient, MeilisearchConfig, MessageDocument,
};
pub use memory::*;
#[allow(unused_imports)]
pub use neo4j::{Neo4jClient, Neo4jConfig, Neo4jConversationStore, Neo4jSessionStore};
#[allow(unused_imports)]
pub use tiered_cache::{TieredCache, TieredCacheConfig, TieredCacheStats};
pub use traits::*;
