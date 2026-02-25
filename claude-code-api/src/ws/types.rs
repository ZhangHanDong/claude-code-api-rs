//! WebSocket bridge types
//!
//! Core data structures for managing WebSocket sessions between
//! CLI processes and external clients.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use tokio::sync::mpsc;

/// State of a CLI session connected via WebSocket
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    /// Our session ID (assigned by the bridge)
    pub session_id: String,
    /// CLI's internal session ID (for --resume)
    pub cli_session_id: Option<String>,
    /// Current model being used
    pub model: String,
    /// Working directory of the CLI
    pub cwd: String,
    /// Available tools
    pub tools: Vec<String>,
    /// Permission mode (default, acceptEdits, bypassPermissions, etc.)
    pub permission_mode: String,
    /// Claude Code CLI version
    pub claude_code_version: String,
    /// Connected MCP servers
    pub mcp_servers: Vec<Value>,
    /// Accumulated cost in USD
    pub total_cost_usd: f64,
    /// Number of conversation turns
    pub num_turns: u32,
    /// Whether the session is currently compacting context
    pub is_compacting: bool,
}

impl SessionState {
    /// Create a new session state with defaults
    pub fn new(session_id: String) -> Self {
        Self {
            session_id,
            cli_session_id: None,
            model: String::new(),
            cwd: String::new(),
            tools: Vec::new(),
            permission_mode: "default".to_string(),
            claude_code_version: String::new(),
            mcp_servers: Vec::new(),
            total_cost_usd: 0.0,
            num_turns: 0,
            is_compacting: false,
        }
    }

    /// Update from a system/init message from the CLI
    pub fn update_from_init(&mut self, data: &Value) {
        if let Some(s) = data.get("session_id").and_then(|v| v.as_str()) {
            self.cli_session_id = Some(s.to_string());
        }
        if let Some(m) = data.get("model").and_then(|v| v.as_str()) {
            self.model = m.to_string();
        }
        if let Some(c) = data.get("cwd").and_then(|v| v.as_str()) {
            self.cwd = c.to_string();
        }
        if let Some(arr) = data.get("tools").and_then(|v| v.as_array()) {
            self.tools = arr
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect();
        }
        if let Some(p) = data.get("permissionMode").and_then(|v| v.as_str()) {
            self.permission_mode = p.to_string();
        }
        if let Some(v) = data.get("claude_code_version").and_then(|v| v.as_str()) {
            self.claude_code_version = v.to_string();
        }
        if let Some(arr) = data.get("mcp_servers").and_then(|v| v.as_array()) {
            self.mcp_servers = arr.clone();
        }
    }
}

/// A pending permission request from the CLI
#[derive(Debug, Clone, Serialize)]
pub struct PendingPermission {
    /// Correlation request ID
    pub request_id: String,
    /// Tool requesting permission
    pub tool_name: String,
    /// Tool input arguments
    pub input: Value,
    /// Human-readable description
    pub description: Option<String>,
    /// Timestamp when the request was received (millis since epoch)
    pub timestamp: u64,
}

/// A session connecting a CLI process to external clients
pub struct Session {
    /// Bridge-assigned session ID
    pub id: String,
    /// Sender to write NDJSON to the CLI's WebSocket
    pub cli_tx: Option<mpsc::Sender<String>>,
    /// Senders to write to each connected external client
    pub client_senders: Vec<mpsc::Sender<String>>,
    /// Session metadata
    pub state: SessionState,
    /// Pending permission requests keyed by request_id
    pub pending_permissions: HashMap<String, PendingPermission>,
    /// Messages queued while CLI is not yet connected
    pub pending_messages: Vec<String>,
    /// Message history for client reconnection (full JSON values)
    pub message_history: Vec<Value>,
}

impl Session {
    /// Create a new empty session
    pub fn new(id: String) -> Self {
        Self {
            state: SessionState::new(id.clone()),
            id,
            cli_tx: None,
            client_senders: Vec::new(),
            pending_permissions: HashMap::new(),
            pending_messages: Vec::new(),
            message_history: Vec::new(),
        }
    }
}

/// Response body for session creation
#[derive(Debug, Serialize)]
pub struct CreateSessionResponse {
    pub session_id: String,
    pub ws_url: String,
}

/// Response body for session listing
#[derive(Debug, Serialize)]
pub struct SessionInfo {
    pub session_id: String,
    pub state: SessionState,
    pub connected_clients: usize,
    pub cli_connected: bool,
    pub pending_permissions: usize,
}

/// Request body for creating a new session
#[derive(Debug, Deserialize)]
pub struct CreateSessionRequest {
    pub model: Option<String>,
    pub cwd: Option<String>,
    pub permission_mode: Option<String>,
    pub allowed_tools: Option<Vec<String>>,
}
