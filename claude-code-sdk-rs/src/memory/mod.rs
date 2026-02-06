//! # Memory Module for Claude Code SDK
//!
//! This module provides persistent memory capabilities for conversations,
//! enabling context retrieval across sessions.
//!
//! ## Architecture
//!
//! The memory system uses a multi-factor scoring approach:
//! - **Semantic**: Text similarity via Meilisearch
//! - **CWD Match**: Same working directory bonus
//! - **Files Overlap**: Common files between conversations
//! - **Recency**: Exponential time decay
//!
//! ## Components
//!
//! - `MessageDocument`: Persistent message storage format
//! - `ToolContextExtractor`: Extracts context from tool calls
//! - `RelevanceScorer`: Multi-factor relevance scoring
//! - `MemoryProvider`: Unified memory access trait

mod integration;
mod message_document;
mod scoring;
mod tool_context;

pub use integration::{ConversationMemoryManager, MemoryIntegrationBuilder, SummaryGenerator};
pub use message_document::{ConversationDocument, MemoryConfig, MessageDocument};
pub use scoring::{RelevanceConfig, RelevanceScore, RelevanceScorer};
pub use tool_context::{
    DefaultToolContextExtractor, MessageContextAggregator, ToolContext, ToolContextExtractor,
};

#[cfg(not(feature = "memory"))]
pub use integration::QueryContext;

#[cfg(feature = "memory")]
mod provider;

#[cfg(feature = "memory")]
pub use provider::{
    ContextFormatter, MeilisearchMemoryProvider, MemoryError, MemoryProvider,
    MemoryProviderBuilder, MemoryResult, QueryContext, ScoredMemoryResult,
};

#[cfg(feature = "memory")]
pub use integration::ContextInjector;
