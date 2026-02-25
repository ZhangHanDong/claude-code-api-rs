//! REST endpoints for WebSocket session management
//!
//! These endpoints create, list, inspect, and delete WebSocket sessions
//! that bridge CLI processes with external clients.

use crate::ws::bridge::WsBridge;
use crate::ws::launcher::WsCliLauncher;
use crate::ws::types::{CreateSessionRequest, CreateSessionResponse};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde_json::{json, Value};
use std::sync::Arc;
use tracing::{error, info};

/// Shared state for WebSocket session endpoints
#[derive(Clone)]
pub struct WsSessionState {
    pub bridge: Arc<WsBridge>,
    pub launcher: Arc<WsCliLauncher>,
}

/// POST /v1/sessions — Create a new WebSocket session
///
/// Spawns a CLI process with `--sdk-url` and returns the session ID
/// and WebSocket URL for clients to connect to.
pub async fn create_session(
    State(state): State<WsSessionState>,
    Json(req): Json<CreateSessionRequest>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    let session_id = uuid::Uuid::new_v4().to_string();

    // Create session in bridge
    state.bridge.create_session(session_id.clone()).await;

    // Launch CLI process
    match state
        .launcher
        .launch(
            &session_id,
            req.model.as_deref(),
            req.cwd.as_deref(),
            req.permission_mode.as_deref(),
            None,
        )
        .await
    {
        Ok(pid) => {
            info!("Created session {session_id} with CLI pid {pid}");
            let resp = CreateSessionResponse {
                session_id: session_id.clone(),
                ws_url: format!("/ws/session/{session_id}"),
            };
            Ok((StatusCode::CREATED, Json(serde_json::to_value(resp).unwrap())))
        }
        Err(e) => {
            error!("Failed to launch CLI for session {session_id}: {e}");
            // Clean up the bridge session
            state.bridge.remove_session(&session_id).await;
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": format!("Failed to launch CLI: {e}"),
                })),
            ))
        }
    }
}

/// GET /v1/sessions — List all sessions
pub async fn list_sessions(
    State(state): State<WsSessionState>,
) -> Json<Value> {
    let bridge_sessions = state.bridge.list_sessions().await;
    Json(json!({
        "sessions": bridge_sessions,
    }))
}

/// GET /v1/sessions/:id — Get session info
pub async fn get_session(
    State(state): State<WsSessionState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    match state.bridge.get_session(&id).await {
        Some(info) => Ok(Json(serde_json::to_value(info).unwrap())),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": format!("Session not found: {id}"),
            })),
        )),
    }
}

/// DELETE /v1/sessions/:id — Delete a session
pub async fn delete_session(
    State(state): State<WsSessionState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // Kill the CLI process
    if let Err(e) = state.launcher.kill(&id).await {
        error!("Failed to kill CLI for session {id}: {e}");
    }

    // Remove from bridge
    if state.bridge.remove_session(&id).await {
        info!("Deleted session {id}");
        Ok(Json(json!({
            "deleted": true,
            "session_id": id,
        })))
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": format!("Session not found: {id}"),
            })),
        ))
    }
}
