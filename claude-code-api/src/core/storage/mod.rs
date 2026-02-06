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

mod traits;
mod memory;
pub mod neo4j;
pub mod meilisearch;
pub mod combined;
pub mod tiered_cache;

pub use traits::*;
pub use memory::*;
pub use neo4j::{Neo4jClient, Neo4jConfig, Neo4jConversationStore, Neo4jSessionStore};
pub use meilisearch::{MeilisearchClient, MeilisearchConfig, MessageDocument, ConversationDocument};
pub use combined::{CombinedConversationStore, CombinedSessionStore};
pub use tiered_cache::{TieredCache, TieredCacheConfig, TieredCacheStats};
