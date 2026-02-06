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

mod long_term;
mod medium_term;
mod short_term;
mod traits;
mod unified;

// Re-export for public API
#[allow(unused_imports)]
pub use long_term::LongTermMemory;
#[allow(unused_imports)]
pub use medium_term::MediumTermMemory;
#[allow(unused_imports)]
pub use short_term::ShortTermMemory;
#[allow(unused_imports)]
pub use traits::{ContextualMemoryProvider, MemoryResult, MemorySource, RelevanceScore};
#[allow(unused_imports)]
pub use unified::UnifiedMemoryProvider;
