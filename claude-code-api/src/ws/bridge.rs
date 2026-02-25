//! WebSocket Bridge — Core message router
//!
//! Routes NDJSON messages between CLI processes and external clients.
//! The bridge manages sessions, each of which has one CLI connection
//! and zero or more client connections.

use super::ndjson::to_ndjson;
use super::types::{PendingPermission, Session, SessionInfo, SessionState};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, info, warn};

/// The WsBridge is the central message router. It owns all sessions and
/// handles routing messages between CLI processes and external clients.
pub struct WsBridge {
    sessions: Arc<RwLock<HashMap<String, Session>>>,
}

impl WsBridge {
    /// Create a new empty bridge
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new session and return its ID
    pub async fn create_session(&self, session_id: String) -> SessionState {
        let session = Session::new(session_id.clone());
        let state = session.state.clone();
        self.sessions.write().await.insert(session_id, session);
        state
    }

    /// Remove a session entirely
    pub async fn remove_session(&self, session_id: &str) -> bool {
        self.sessions.write().await.remove(session_id).is_some()
    }

    /// Register a CLI sender for a session
    pub async fn register_cli(&self, session_id: &str, tx: mpsc::Sender<String>) {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.cli_tx = Some(tx);

            // Flush any pending messages that were queued before CLI connected
            let pending: Vec<String> = session.pending_messages.drain(..).collect();
            if !pending.is_empty() {
                info!(
                    "Flushing {} pending messages to CLI for session {session_id}",
                    pending.len()
                );
                if let Some(ref cli_tx) = session.cli_tx {
                    for msg in pending {
                        let _ = cli_tx.send(msg).await;
                    }
                }
            }
        }
    }

    /// Unregister the CLI from a session
    pub async fn unregister_cli(&self, session_id: &str) {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.cli_tx = None;
            // Cancel all pending permissions
            session.pending_permissions.clear();
        }
    }

    /// Register a client sender for a session, returns history for replay
    pub async fn register_client(
        &self,
        session_id: &str,
        tx: mpsc::Sender<String>,
    ) -> Option<(SessionState, Vec<Value>, Vec<PendingPermission>)> {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.client_senders.push(tx);
            let state = session.state.clone();
            let history = session.message_history.clone();
            let pending: Vec<PendingPermission> =
                session.pending_permissions.values().cloned().collect();
            Some((state, history, pending))
        } else {
            None
        }
    }

    /// Remove a specific client sender (by pointer equality)
    pub async fn unregister_client(&self, session_id: &str, tx: &mpsc::Sender<String>) {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session
                .client_senders
                .retain(|s| !s.same_channel(tx));
        }
    }

    /// Route a message from the CLI to connected clients.
    ///
    /// This implements the CLI→Client routing table from the plan.
    pub async fn route_cli_message(&self, session_id: &str, json: Value) {
        let msg_type = match json.get("type").and_then(|v| v.as_str()) {
            Some(t) => t.to_string(),
            None => {
                warn!("CLI message without 'type' field: {json}");
                return;
            }
        };

        let mut sessions = self.sessions.write().await;
        let session = match sessions.get_mut(session_id) {
            Some(s) => s,
            None => {
                warn!("CLI message for unknown session: {session_id}");
                return;
            }
        };

        match msg_type.as_str() {
            // system/init — update state, broadcast session_init, flush pending
            "system" => {
                let subtype = json
                    .get("subtype")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                match subtype {
                    "init" => {
                        session.state.update_from_init(&json);
                        debug!(
                            "Session {session_id} initialized: model={}, cwd={}",
                            session.state.model, session.state.cwd
                        );

                        // Broadcast session_init to clients
                        let init_msg = json!({
                            "type": "session_init",
                            "session_id": session_id,
                            "state": serde_json::to_value(&session.state).unwrap_or(json!({})),
                        });
                        let ndjson = to_ndjson(&init_msg);
                        broadcast_to_clients(&session.client_senders, &ndjson).await;
                    }
                    "status" => {
                        // Track compacting state
                        if let Some(status) = json.get("status") {
                            session.state.is_compacting = status.as_str() == Some("compacting");
                        }
                        // Broadcast to clients
                        let ndjson = to_ndjson(&json);
                        broadcast_to_clients(&session.client_senders, &ndjson).await;
                    }
                    _ => {
                        // Other system subtypes: forward to clients
                        let ndjson = to_ndjson(&json);
                        broadcast_to_clients(&session.client_senders, &ndjson).await;
                    }
                }
            }

            // assistant — store in history, broadcast
            "assistant" => {
                session.message_history.push(json.clone());
                let ndjson = to_ndjson(&json);
                broadcast_to_clients(&session.client_senders, &ndjson).await;
            }

            // result — update cost/turns, store in history, broadcast
            "result" => {
                if let Some(cost) = json.get("total_cost_usd").and_then(|v| v.as_f64()) {
                    session.state.total_cost_usd = cost;
                }
                if let Some(turns) = json.get("num_turns").and_then(|v| v.as_u64()) {
                    session.state.num_turns = turns as u32;
                }
                session.message_history.push(json.clone());
                let ndjson = to_ndjson(&json);
                broadcast_to_clients(&session.client_senders, &ndjson).await;
            }

            // stream_event — broadcast but don't store in history
            "stream_event" => {
                let ndjson = to_ndjson(&json);
                broadcast_to_clients(&session.client_senders, &ndjson).await;
            }

            // control_request — check for can_use_tool (permission request)
            "control_request" => {
                if let Some(request) = json.get("request") {
                    let subtype = request.get("subtype").and_then(|v| v.as_str());
                    if subtype == Some("can_use_tool") {
                        let request_id = json
                            .get("request_id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let tool_name = request
                            .get("tool_name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let input = request
                            .get("input")
                            .cloned()
                            .unwrap_or_else(|| json!({}));
                        let description = request
                            .get("description")
                            .and_then(|v| v.as_str())
                            .map(String::from);

                        let now = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis() as u64;

                        let pending = PendingPermission {
                            request_id: request_id.clone(),
                            tool_name,
                            input,
                            description,
                            timestamp: now,
                        };
                        session
                            .pending_permissions
                            .insert(request_id, pending);

                        // Broadcast as permission_request to clients
                        let perm_msg = json!({
                            "type": "permission_request",
                            "request_id": json.get("request_id"),
                            "request": request,
                        });
                        let ndjson = to_ndjson(&perm_msg);
                        broadcast_to_clients(&session.client_senders, &ndjson).await;
                    } else {
                        // Other control requests: forward as-is
                        let ndjson = to_ndjson(&json);
                        broadcast_to_clients(&session.client_senders, &ndjson).await;
                    }
                }
            }

            // tool_progress, tool_use_summary — broadcast
            "tool_progress" | "tool_use_summary" => {
                let ndjson = to_ndjson(&json);
                broadcast_to_clients(&session.client_senders, &ndjson).await;
            }

            // keep_alive — silently consume
            "keep_alive" => {}

            // Unknown types — log and skip (forward-compatible)
            other => {
                debug!("Unknown CLI message type '{other}', forwarding to clients");
                let ndjson = to_ndjson(&json);
                broadcast_to_clients(&session.client_senders, &ndjson).await;
            }
        }
    }

    /// Route a message from an external client to the CLI.
    ///
    /// This implements the Client→CLI routing table from the plan.
    pub async fn route_client_message(&self, session_id: &str, json: Value) {
        let msg_type = match json.get("type").and_then(|v| v.as_str()) {
            Some(t) => t.to_string(),
            None => {
                warn!("Client message without 'type' field: {json}");
                return;
            }
        };

        let mut sessions = self.sessions.write().await;
        let session = match sessions.get_mut(session_id) {
            Some(s) => s,
            None => {
                warn!("Client message for unknown session: {session_id}");
                return;
            }
        };

        match msg_type.as_str() {
            // user_message — format as NDJSON user message and send to CLI
            "user_message" => {
                let content = json
                    .get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let cli_session_id = session
                    .state
                    .cli_session_id
                    .clone()
                    .unwrap_or_default();

                let user_msg = json!({
                    "type": "user",
                    "message": {
                        "role": "user",
                        "content": content
                    },
                    "parent_tool_use_id": null,
                    "session_id": cli_session_id
                });

                send_to_cli(session, &to_ndjson(&user_msg)).await;
            }

            // permission_response — format as control_response and send to CLI
            "permission_response" => {
                let request_id = json
                    .get("request_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                // Remove from pending
                session.pending_permissions.remove(request_id);

                let behavior = json
                    .get("behavior")
                    .and_then(|v| v.as_str())
                    .unwrap_or("deny");

                let response_payload = if behavior == "allow" {
                    let updated_input = json
                        .get("updated_input")
                        .cloned()
                        .unwrap_or_else(|| json!({}));
                    json!({
                        "behavior": "allow",
                        "updatedInput": updated_input
                    })
                } else {
                    let message = json
                        .get("message")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Permission denied by user");
                    json!({
                        "behavior": "deny",
                        "message": message
                    })
                };

                let control_response = json!({
                    "type": "control_response",
                    "response": {
                        "subtype": "success",
                        "request_id": request_id,
                        "response": response_payload
                    }
                });

                send_to_cli(session, &to_ndjson(&control_response)).await;
            }

            // interrupt — send interrupt control request to CLI
            "interrupt" => {
                let request_id = uuid::Uuid::new_v4().to_string();
                let interrupt_msg = json!({
                    "type": "control_request",
                    "request_id": request_id,
                    "request": {
                        "subtype": "interrupt"
                    }
                });

                send_to_cli(session, &to_ndjson(&interrupt_msg)).await;
            }

            // set_model — send set_model control request
            "set_model" => {
                let model = json
                    .get("model")
                    .and_then(|v| v.as_str())
                    .unwrap_or("default");
                let request_id = uuid::Uuid::new_v4().to_string();
                let msg = json!({
                    "type": "control_request",
                    "request_id": request_id,
                    "request": {
                        "subtype": "set_model",
                        "model": model
                    }
                });

                send_to_cli(session, &to_ndjson(&msg)).await;
            }

            // set_permission_mode — send set_permission_mode control request
            "set_permission_mode" => {
                let mode = json
                    .get("mode")
                    .and_then(|v| v.as_str())
                    .unwrap_or("default");
                let request_id = uuid::Uuid::new_v4().to_string();
                let msg = json!({
                    "type": "control_request",
                    "request_id": request_id,
                    "request": {
                        "subtype": "set_permission_mode",
                        "mode": mode
                    }
                });

                send_to_cli(session, &to_ndjson(&msg)).await;
            }

            // Unknown client message type
            other => {
                warn!("Unknown client message type: {other}");
            }
        }
    }

    /// Get info about all sessions
    pub async fn list_sessions(&self) -> Vec<SessionInfo> {
        let sessions = self.sessions.read().await;
        sessions
            .values()
            .map(|s| SessionInfo {
                session_id: s.id.clone(),
                state: s.state.clone(),
                connected_clients: s.client_senders.len(),
                cli_connected: s.cli_tx.is_some(),
                pending_permissions: s.pending_permissions.len(),
            })
            .collect()
    }

    /// Get info about a specific session
    pub async fn get_session(&self, session_id: &str) -> Option<SessionInfo> {
        let sessions = self.sessions.read().await;
        sessions.get(session_id).map(|s| SessionInfo {
            session_id: s.id.clone(),
            state: s.state.clone(),
            connected_clients: s.client_senders.len(),
            cli_connected: s.cli_tx.is_some(),
            pending_permissions: s.pending_permissions.len(),
        })
    }

    /// Get a clone of the sessions map for the launcher
    pub fn sessions(&self) -> Arc<RwLock<HashMap<String, Session>>> {
        self.sessions.clone()
    }
}

/// Broadcast a message to all connected clients in a session.
/// Removes senders that have been closed.
async fn broadcast_to_clients(senders: &[mpsc::Sender<String>], message: &str) {
    for sender in senders {
        if sender.send(message.to_string()).await.is_err() {
            debug!("Client sender closed, will be cleaned up on disconnect");
        }
    }
}

/// Send a message to the CLI, or queue it if CLI is not yet connected.
async fn send_to_cli(session: &mut Session, message: &str) {
    if let Some(ref cli_tx) = session.cli_tx {
        if cli_tx.send(message.to_string()).await.is_err() {
            warn!("Failed to send to CLI for session {}", session.id);
        }
    } else {
        debug!(
            "CLI not connected for session {}, queuing message",
            session.id
        );
        session.pending_messages.push(message.to_string());
    }
}
