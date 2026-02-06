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
//! - [`MessageDocument`]: Persistent message storage format
//! - [`ToolContextExtractor`]: Extracts context from tool calls
//! - [`RelevanceScorer`]: Multi-factor relevance scoring
//! - [`MemoryProvider`]: Unified memory access trait

mod message_document;
mod tool_context;
mod scoring;
mod integration;

pub use message_document::{
    MessageDocument, ConversationDocument, MemoryConfig,
};
pub use tool_context::{
    ToolContext, ToolContextExtractor, MessageContextAggregator,
    DefaultToolContextExtractor,
};
pub use scoring::{
    RelevanceScore, RelevanceConfig, RelevanceScorer,
};
pub use integration::{
    ConversationMemoryManager, MemoryIntegrationBuilder, SummaryGenerator,
};

#[cfg(not(feature = "memory"))]
pub use integration::QueryContext;

#[cfg(feature = "memory")]
mod provider;

#[cfg(feature = "memory")]
pub use provider::{
    MemoryProvider, MeilisearchMemoryProvider, MemoryResult, MemoryError,
    QueryContext, ScoredMemoryResult, ContextFormatter, MemoryProviderBuilder,
};

#[cfg(feature = "memory")]
pub use integration::ContextInjector;
