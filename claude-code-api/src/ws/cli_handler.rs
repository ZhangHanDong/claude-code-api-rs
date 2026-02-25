//! CLI WebSocket handler
//!
//! Endpoint: `/ws/cli/:session_id`
//!
//! The Claude Code CLI connects here when launched with `--sdk-url`.
//! This handler receives NDJSON messages from the CLI and routes them
//! to connected external clients via the WsBridge.

use super::bridge::WsBridge;
use super::ndjson::parse_ndjson;
use axum::{
    extract::{Path, State, WebSocketUpgrade, ws::Message as AxumWsMessage},
    response::IntoResponse,
};
use futures::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

/// WebSocket handler for CLI connections.
///
/// The CLI connects to `/ws/cli/:session_id` after being launched with
/// `--sdk-url ws://host:port/ws/cli/:session_id`.
pub async fn ws_cli_handler(
    ws: WebSocketUpgrade,
    Path(session_id): Path<String>,
    State(bridge): State<Arc<WsBridge>>,
) -> impl IntoResponse {
    info!("CLI WebSocket upgrade request for session {session_id}");

    ws.on_upgrade(move |socket| handle_cli_socket(socket, session_id, bridge))
}

async fn handle_cli_socket(
    socket: axum::extract::ws::WebSocket,
    session_id: String,
    bridge: Arc<WsBridge>,
) {
    let (mut ws_sink, mut ws_stream) = socket.split();

    // Create channel for writing to the CLI's WebSocket
    let (cli_tx, mut cli_rx) = mpsc::channel::<String>(256);

    // Register CLI sender with the bridge
    bridge.register_cli(&session_id, cli_tx).await;
    info!("CLI connected for session {session_id}");

    // Write task: drain channel → WS sink
    let session_id_write = session_id.clone();
    let write_task = tokio::spawn(async move {
        while let Some(msg) = cli_rx.recv().await {
            if ws_sink
                .send(AxumWsMessage::Text(msg.into()))
                .await
                .is_err()
            {
                warn!("Failed to write to CLI WebSocket for session {session_id_write}");
                break;
            }
        }
        debug!("CLI write task ended for session {session_id_write}");
    });

    // Read loop: WS stream → parse NDJSON → route via bridge
    let session_id_read = session_id.clone();
    let bridge_read = bridge.clone();
    while let Some(msg) = ws_stream.next().await {
        match msg {
            Ok(AxumWsMessage::Text(text)) => {
                let values = parse_ndjson(&text);
                for value in values {
                    bridge_read
                        .route_cli_message(&session_id_read, value)
                        .await;
                }
            }
            Ok(AxumWsMessage::Close(_)) => {
                info!("CLI WebSocket closed for session {session_id_read}");
                break;
            }
            Ok(AxumWsMessage::Ping(_)) | Ok(AxumWsMessage::Pong(_)) => {
                // Handled automatically by axum
            }
            Ok(_) => {
                // Binary or other — ignore
            }
            Err(e) => {
                error!("CLI WebSocket error for session {session_id_read}: {e}");
                break;
            }
        }
    }

    // Cleanup
    bridge.unregister_cli(&session_id).await;
    write_task.abort();
    info!("CLI disconnected from session {session_id}");
}
