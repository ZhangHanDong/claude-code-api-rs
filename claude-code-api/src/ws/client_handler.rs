//! Client WebSocket handler
//!
//! Endpoint: `/ws/session/:session_id`
//!
//! External clients (browsers, tools) connect here to interact with a
//! CLI session. On connect, they receive session_init + message history
//! + any pending permission requests.

use super::bridge::WsBridge;
use super::ndjson::{parse_ndjson, to_ndjson};
use axum::{
    extract::{Path, State, WebSocketUpgrade, ws::Message as AxumWsMessage},
    response::IntoResponse,
};
use futures::{SinkExt, StreamExt};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

/// WebSocket handler for external client connections.
///
/// Clients connect to `/ws/session/:session_id` to interact with
/// a running Claude Code CLI session.
pub async fn ws_session_handler(
    ws: WebSocketUpgrade,
    Path(session_id): Path<String>,
    State(bridge): State<Arc<WsBridge>>,
) -> impl IntoResponse {
    info!("Client WebSocket upgrade request for session {session_id}");

    ws.on_upgrade(move |socket| handle_client_socket(socket, session_id, bridge))
}

async fn handle_client_socket(
    socket: axum::extract::ws::WebSocket,
    session_id: String,
    bridge: Arc<WsBridge>,
) {
    let (mut ws_sink, mut ws_stream) = socket.split();

    // Create channel for writing to this client's WebSocket
    let (client_tx, mut client_rx) = mpsc::channel::<String>(256);

    // Register with bridge and get session state + history
    let registration = bridge.register_client(&session_id, client_tx.clone()).await;

    let (state, history, pending_perms) = match registration {
        Some(data) => data,
        None => {
            warn!("Client tried to connect to unknown session {session_id}");
            let err_msg = json!({
                "type": "error",
                "error": "Session not found",
                "session_id": session_id,
            });
            let _ = ws_sink
                .send(AxumWsMessage::Text(to_ndjson(&err_msg).into()))
                .await;
            return;
        }
    };

    info!(
        "Client connected to session {session_id} (history: {} msgs, pending perms: {})",
        history.len(),
        pending_perms.len()
    );

    // Send session_init to the newly connected client
    let init_msg = json!({
        "type": "session_init",
        "session_id": session_id,
        "state": serde_json::to_value(&state).unwrap_or(json!({})),
    });
    if ws_sink
        .send(AxumWsMessage::Text(to_ndjson(&init_msg).into()))
        .await
        .is_err()
    {
        warn!("Failed to send session_init to client");
        return;
    }

    // Replay message history
    for msg in &history {
        if ws_sink
            .send(AxumWsMessage::Text(to_ndjson(msg).into()))
            .await
            .is_err()
        {
            warn!("Failed to replay history to client");
            return;
        }
    }

    // Send pending permission requests
    for perm in &pending_perms {
        let perm_msg = json!({
            "type": "permission_request",
            "request_id": perm.request_id,
            "request": {
                "subtype": "can_use_tool",
                "tool_name": perm.tool_name,
                "input": perm.input,
                "description": perm.description,
            },
        });
        if ws_sink
            .send(AxumWsMessage::Text(to_ndjson(&perm_msg).into()))
            .await
            .is_err()
        {
            warn!("Failed to send pending permission to client");
            return;
        }
    }

    // Write task: drain channel → WS sink
    let session_id_write = session_id.clone();
    let write_task = tokio::spawn(async move {
        while let Some(msg) = client_rx.recv().await {
            if ws_sink
                .send(AxumWsMessage::Text(msg.into()))
                .await
                .is_err()
            {
                debug!("Client WebSocket write failed for session {session_id_write}");
                break;
            }
        }
        debug!("Client write task ended for session {session_id_write}");
    });

    // Read loop: WS stream → parse client messages → route via bridge
    let session_id_read = session_id.clone();
    let bridge_read = bridge.clone();
    while let Some(msg) = ws_stream.next().await {
        match msg {
            Ok(AxumWsMessage::Text(text)) => {
                let values = parse_ndjson(&text);
                for value in values {
                    bridge_read
                        .route_client_message(&session_id_read, value)
                        .await;
                }
            }
            Ok(AxumWsMessage::Close(_)) => {
                info!("Client disconnected from session {session_id_read}");
                break;
            }
            Ok(AxumWsMessage::Ping(_)) | Ok(AxumWsMessage::Pong(_)) => {}
            Ok(_) => {}
            Err(e) => {
                error!("Client WebSocket error for session {session_id_read}: {e}");
                break;
            }
        }
    }

    // Cleanup
    bridge.unregister_client(&session_id, &client_tx).await;
    write_task.abort();
    info!("Client disconnected from session {session_id}");
}
