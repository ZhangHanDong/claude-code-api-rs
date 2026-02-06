//! Contextual memory system for Claude Code API
//!
//! This module provides a 3-level memory system:
//! - **Short-term**: Current conversation (via ConversationStore)
//! - **Medium-term**: Plans, tasks, decisions (via project-orchestrator MCP)
//! - **Long-term**: Knowledge Notes + cross-conversation search (via Meilisearch)
//!
//! ## Usage
//!
//! ```rust,ignore
//! let memory = UnifiedMemoryProvider::new(
//!     short_term,
//!     medium_term,
//!     long_term,
//! );
//!
//! // Query across all memory levels
//! let results = memory.query("What did we decide about authentication?").await?;
//! ```

mod traits;
mod short_term;
mod medium_term;
mod long_term;
mod unified;

pub use traits::{ContextualMemoryProvider, MemoryResult, MemorySource, RelevanceScore};
pub use short_term::ShortTermMemory;
pub use medium_term::MediumTermMemory;
pub use long_term::LongTermMemory;
pub use unified::UnifiedMemoryProvider;
