//! Storage abstractions for Claude Code API
//!
//! This module defines traits for different storage backends, allowing
//! the API to work with in-memory, Neo4j, or other storage implementations.

mod traits;
mod memory;

pub use traits::*;
pub use memory::*;
