//! WebSocket module for real-time CLI↔Client bridging
//!
//! This module implements the "Plan A" WebSocket bridge architecture:
//! - CLI processes connect via `--sdk-url` to `/ws/cli/:session_id`
//! - External clients connect to `/ws/session/:session_id`
//! - The bridge routes NDJSON messages between them

pub mod bridge;
pub mod cli_handler;
pub mod client_handler;
pub mod launcher;
pub mod ndjson;
pub mod types;
