//! WebSocket transport for Claude Code SDK
//!
//! This module implements the `Transport` trait over WebSocket, enabling the SDK
//! to communicate with a WebSocket server using the same NDJSON protocol that
//! the subprocess transport uses over stdin/stdout.
//!
//! This is the "Plan B" transport: the SDK acts as a WebSocket **client**,
//! connecting to a server (e.g., a bridge that manages CLI processes via `--sdk-url`).

use crate::{
    errors::{Result, SdkError},
    transport::{InputMessage, Transport, TransportState},
    types::{ControlRequest, ControlResponse, Message},
};
use async_trait::async_trait;
use futures::stream::Stream;
use serde_json::Value as JsonValue;
use std::pin::Pin;
use tokio::sync::{broadcast, mpsc, watch};
use tracing::{debug, error, info, warn};

/// Configuration for the WebSocket transport
#[derive(Debug, Clone)]
pub struct WebSocketConfig {
    /// Maximum number of reconnection attempts before giving up (default: 3)
    pub max_reconnect_attempts: u32,
    /// Base delay in milliseconds for exponential backoff (default: 1000)
    pub base_reconnect_delay_ms: u64,
    /// Maximum delay in milliseconds for exponential backoff (default: 30000)
    pub max_reconnect_delay_ms: u64,
    /// Interval in seconds between keepalive pings (default: 10)
    pub ping_interval_secs: u64,
    /// Capacity of the message broadcast channel (default: 1000)
    pub message_buffer_capacity: usize,
    /// Optional Bearer token for WebSocket upgrade authentication
    pub auth_token: Option<String>,
}

impl Default for WebSocketConfig {
    fn default() -> Self {
        Self {
            max_reconnect_attempts: 3,
            base_reconnect_delay_ms: 1000,
            max_reconnect_delay_ms: 30000,
            ping_interval_secs: 10,
            message_buffer_capacity: 1000,
            auth_token: None,
        }
    }
}

/// WebSocket transport that implements the Transport trait.
///
/// Connects to a WebSocket server and communicates using NDJSON — the same
/// wire protocol used by `SubprocessTransport` over stdin/stdout.
///
/// Manual `Debug` impl because channel senders don't derive Debug.
pub struct WebSocketTransport {
    url: url::Url,
    config: WebSocketConfig,
    /// Sender for outgoing messages (NDJSON lines to WS sink)
    ws_tx: Option<mpsc::Sender<String>>,
    /// Broadcast sender for parsed incoming messages
    message_broadcast_tx: Option<broadcast::Sender<Message>>,
    /// Receiver for legacy control responses (interrupt acks)
    control_rx: Option<mpsc::Receiver<ControlResponse>>,
    /// Receiver for SDK control protocol messages (JSON)
    sdk_control_rx: Option<mpsc::Receiver<JsonValue>>,
    /// Current transport state
    state: TransportState,
    /// Counter for generating unique request IDs
    request_counter: u64,
    /// Shutdown signal sender
    shutdown_tx: Option<watch::Sender<bool>>,
}

impl std::fmt::Debug for WebSocketTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WebSocketTransport")
            .field("url", &self.url)
            .field("state", &self.state)
            .field("request_counter", &self.request_counter)
            .field("ws_tx", &self.ws_tx.is_some())
            .finish()
    }
}

impl WebSocketTransport {
    /// Create a new WebSocket transport targeting the given URL.
    ///
    /// # Arguments
    /// * `url` - WebSocket URL to connect to (e.g., `ws://localhost:8765`)
    /// * `config` - Transport configuration
    pub fn new(url: &str, config: WebSocketConfig) -> Result<Self> {
        let parsed_url = url::Url::parse(url).map_err(|e| {
            SdkError::WebSocketError(format!("Invalid WebSocket URL '{url}': {e}"))
        })?;

        match parsed_url.scheme() {
            "ws" | "wss" => {}
            scheme => {
                return Err(SdkError::WebSocketError(format!(
                    "Unsupported URL scheme '{scheme}', expected 'ws' or 'wss'"
                )));
            }
        }

        Ok(Self {
            url: parsed_url,
            config,
            ws_tx: None,
            message_broadcast_tx: None,
            control_rx: None,
            sdk_control_rx: None,
            state: TransportState::Disconnected,
            request_counter: 0,
            shutdown_tx: None,
        })
    }

    /// Build the WebSocket connect request with optional auth headers.
    fn build_ws_request(&self) -> Result<http::Request<()>> {
        let mut request = http::Request::builder()
            .uri(self.url.as_str())
            .header("Host", self.url.host_str().unwrap_or("localhost"))
            .header("Connection", "Upgrade")
            .header("Upgrade", "websocket")
            .header("Sec-WebSocket-Version", "13")
            .header(
                "Sec-WebSocket-Key",
                tokio_tungstenite::tungstenite::handshake::client::generate_key(),
            );

        if let Some(ref token) = self.config.auth_token {
            request = request.header("Authorization", format!("Bearer {token}"));
        }

        request
            .body(())
            .map_err(|e| SdkError::WebSocketError(format!("Failed to build WS request: {e}")))
    }

    /// Establish the WebSocket connection and spawn background tasks.
    async fn establish_connection(&mut self) -> Result<()> {
        use futures::StreamExt;
        use tokio_tungstenite::tungstenite::Message as WsMessage;

        self.state = TransportState::Connecting;

        let request = self.build_ws_request()?;

        let (ws_stream, _response) =
            tokio_tungstenite::connect_async(request)
                .await
                .map_err(|e| {
                    SdkError::WebSocketError(format!("Failed to connect to {}: {e}", self.url))
                })?;

        info!("WebSocket connected to {}", self.url);

        let (ws_sink, ws_stream) = ws_stream.split();

        // Channels
        let (ws_tx, ws_rx) = mpsc::channel::<String>(256);
        let (message_broadcast_tx, _) =
            broadcast::channel::<Message>(self.config.message_buffer_capacity);
        let (control_tx, control_rx) = mpsc::channel::<ControlResponse>(32);
        let (sdk_control_tx, sdk_control_rx) = mpsc::channel::<JsonValue>(64);
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        // === Write task: mpsc::Receiver<String> → WS sink ===
        let mut shutdown_rx_write = shutdown_rx.clone();
        tokio::spawn(async move {
            use futures::SinkExt;
            let mut ws_sink = ws_sink;
            let mut ws_rx = ws_rx;

            loop {
                tokio::select! {
                    msg = ws_rx.recv() => {
                        match msg {
                            Some(line) => {
                                if let Err(e) = ws_sink.send(WsMessage::Text(line.into())).await {
                                    error!("WebSocket write error: {e}");
                                    break;
                                }
                            }
                            None => {
                                debug!("Write channel closed, shutting down write task");
                                break;
                            }
                        }
                    }
                    _ = shutdown_rx_write.changed() => {
                        debug!("Shutdown signal received in write task");
                        let _ = ws_sink.send(WsMessage::Close(None)).await;
                        break;
                    }
                }
            }
            debug!("WebSocket write task ended");
        });

        // === Read task: WS stream → parse JSON → route to channels ===
        let message_broadcast_tx_clone = message_broadcast_tx.clone();
        let control_tx_clone = control_tx;
        let sdk_control_tx_clone = sdk_control_tx;
        let mut shutdown_rx_read = shutdown_rx.clone();

        tokio::spawn(async move {
            let mut ws_stream = ws_stream;

            loop {
                tokio::select! {
                    msg = ws_stream.next() => {
                        match msg {
                            Some(Ok(WsMessage::Text(text))) => {
                                // NDJSON: split by newline, parse each line
                                let text_str: &str = &text;
                                for line in text_str.split('\n') {
                                    let line = line.trim();
                                    if line.is_empty() {
                                        continue;
                                    }

                                    match serde_json::from_str::<JsonValue>(line) {
                                        Ok(json) => {
                                            Self::route_incoming_message(
                                                json,
                                                &message_broadcast_tx_clone,
                                                &control_tx_clone,
                                                &sdk_control_tx_clone,
                                            ).await;
                                        }
                                        Err(e) => {
                                            warn!("Failed to parse WebSocket JSON: {e} — line: {line}");
                                        }
                                    }
                                }
                            }
                            Some(Ok(WsMessage::Ping(data))) => {
                                debug!("Received WS ping, pong is auto-sent by tungstenite");
                                let _ = data; // tungstenite auto-replies with pong
                            }
                            Some(Ok(WsMessage::Pong(_))) => {
                                debug!("Received WS pong");
                            }
                            Some(Ok(WsMessage::Close(frame))) => {
                                info!("WebSocket closed by server: {frame:?}");
                                break;
                            }
                            Some(Ok(_)) => {
                                // Binary or other frame types — ignore
                            }
                            Some(Err(e)) => {
                                error!("WebSocket read error: {e}");
                                break;
                            }
                            None => {
                                info!("WebSocket stream ended");
                                break;
                            }
                        }
                    }
                    _ = shutdown_rx_read.changed() => {
                        debug!("Shutdown signal received in read task");
                        break;
                    }
                }
            }
            debug!("WebSocket read task ended");
        });

        // === Keepalive task: periodic keep_alive + WS ping ===
        let keepalive_tx = ws_tx.clone();
        let ping_interval = self.config.ping_interval_secs;
        let mut shutdown_rx_keepalive = shutdown_rx.clone();

        tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(tokio::time::Duration::from_secs(ping_interval));
            interval.tick().await; // skip first immediate tick

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        let keep_alive = serde_json::json!({"type": "keep_alive"}).to_string();
                        if keepalive_tx.send(keep_alive).await.is_err() {
                            debug!("Keepalive channel closed");
                            break;
                        }
                    }
                    _ = shutdown_rx_keepalive.changed() => {
                        debug!("Shutdown signal received in keepalive task");
                        break;
                    }
                }
            }
            debug!("WebSocket keepalive task ended");
        });

        // Store handles
        self.ws_tx = Some(ws_tx);
        self.message_broadcast_tx = Some(message_broadcast_tx);
        self.control_rx = Some(control_rx);
        self.sdk_control_rx = Some(sdk_control_rx);
        self.shutdown_tx = Some(shutdown_tx);
        self.state = TransportState::Connected;

        Ok(())
    }

    /// Route an incoming JSON message to the appropriate channel.
    ///
    /// This mirrors the routing logic in `SubprocessTransport::spawn_process()`
    /// for the stdout handler.
    async fn route_incoming_message(
        json: JsonValue,
        message_broadcast_tx: &broadcast::Sender<Message>,
        control_tx: &mpsc::Sender<ControlResponse>,
        sdk_control_tx: &mpsc::Sender<JsonValue>,
    ) {
        let msg_type = match json.get("type").and_then(|v| v.as_str()) {
            Some(t) => t,
            None => {
                warn!("Received JSON without 'type' field: {json}");
                return;
            }
        };

        match msg_type {
            // Control responses — responses to OUR control requests
            "control_response" => {
                debug!("Received control response: {json:?}");
                let _ = sdk_control_tx.send(json.clone()).await;

                // Also parse for legacy control_rx (interrupt acks)
                if let Some(response_obj) = json.get("response") {
                    if let Some(request_id) = response_obj
                        .get("request_id")
                        .or_else(|| response_obj.get("requestId"))
                        .and_then(|v| v.as_str())
                    {
                        let subtype = response_obj.get("subtype").and_then(|v| v.as_str());
                        let success = subtype == Some("success");
                        let control_resp = ControlResponse::InterruptAck {
                            request_id: request_id.to_string(),
                            success,
                        };
                        let _ = control_tx.send(control_resp).await;
                    }
                }
            }

            // Control requests FROM the server (e.g., can_use_tool, hook_callback)
            "control_request" => {
                debug!("Received control request: {json:?}");
                let _ = sdk_control_tx.send(json).await;
            }

            // Legacy SDK control format
            "sdk_control_request" => {
                debug!("Received SDK control request (legacy): {json:?}");
                let _ = sdk_control_tx.send(json).await;
            }

            // Control messages (new format)
            "control" => {
                if let Some(control) = json.get("control") {
                    debug!("Received control message: {control:?}");
                    let _ = sdk_control_tx.send(control.clone()).await;
                }
            }

            // System messages with SDK control subtypes
            "system" => {
                if let Some(subtype) = json.get("subtype").and_then(|v| v.as_str()) {
                    if subtype.starts_with("sdk_control:") {
                        debug!("Received SDK control message: {subtype}");
                        let _ = sdk_control_tx.send(json.clone()).await;
                    }
                }
                // Still parse as regular message
                match crate::message_parser::parse_message(json) {
                    Ok(Some(message)) => {
                        let _ = message_broadcast_tx.send(message);
                    }
                    Ok(None) => {}
                    Err(e) => {
                        warn!("Failed to parse system message: {e}");
                    }
                }
            }

            // Keep-alive — silently consume
            "keep_alive" => {
                debug!("Received keep_alive");
            }

            // Regular messages: user, assistant, result, stream_event, etc.
            _ => {
                match crate::message_parser::parse_message(json) {
                    Ok(Some(message)) => {
                        let _ = message_broadcast_tx.send(message);
                    }
                    Ok(None) => {
                        // Non-message JSON (e.g., unknown type) — ignored
                    }
                    Err(e) => {
                        warn!("Failed to parse message: {e}");
                    }
                }
            }
        }
    }
}

#[async_trait]
impl Transport for WebSocketTransport {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    async fn connect(&mut self) -> Result<()> {
        if self.state == TransportState::Connected {
            return Ok(());
        }

        self.establish_connection().await?;
        info!("WebSocket transport connected to {}", self.url);
        Ok(())
    }

    async fn send_message(&mut self, message: InputMessage) -> Result<()> {
        if self.state != TransportState::Connected {
            return Err(SdkError::InvalidState {
                message: "Not connected".into(),
            });
        }

        let json = serde_json::to_string(&message)?;
        debug!("Sending message via WebSocket: {json}");

        if let Some(ref tx) = self.ws_tx {
            tx.send(json)
                .await
                .map_err(|_| SdkError::WebSocketError("Write channel closed".into()))?;
            Ok(())
        } else {
            Err(SdkError::InvalidState {
                message: "WebSocket write channel not available".into(),
            })
        }
    }

    fn receive_messages(
        &mut self,
    ) -> Pin<Box<dyn Stream<Item = Result<Message>> + Send + 'static>> {
        use futures::StreamExt;

        if let Some(ref tx) = self.message_broadcast_tx {
            let rx = tx.subscribe();
            Box::pin(
                tokio_stream::wrappers::BroadcastStream::new(rx).filter_map(|result| async move {
                    match result {
                        Ok(msg) => Some(Ok(msg)),
                        Err(
                            tokio_stream::wrappers::errors::BroadcastStreamRecvError::Lagged(n),
                        ) => {
                            warn!("WebSocket receiver lagged by {n} messages");
                            None
                        }
                    }
                }),
            )
        } else {
            Box::pin(futures::stream::empty())
        }
    }

    async fn send_control_request(&mut self, request: ControlRequest) -> Result<()> {
        if self.state != TransportState::Connected {
            return Err(SdkError::InvalidState {
                message: "Not connected".into(),
            });
        }

        self.request_counter += 1;
        let control_msg = match request {
            ControlRequest::Interrupt { request_id } => {
                serde_json::json!({
                    "type": "control_request",
                    "request": {
                        "type": "interrupt",
                        "request_id": request_id
                    }
                })
            }
        };

        let json = serde_json::to_string(&control_msg)?;

        if let Some(ref tx) = self.ws_tx {
            tx.send(json)
                .await
                .map_err(|_| SdkError::WebSocketError("Write channel closed".into()))?;
            Ok(())
        } else {
            Err(SdkError::InvalidState {
                message: "WebSocket write channel not available".into(),
            })
        }
    }

    async fn receive_control_response(&mut self) -> Result<Option<ControlResponse>> {
        if let Some(ref mut rx) = self.control_rx {
            Ok(rx.recv().await)
        } else {
            Ok(None)
        }
    }

    async fn send_sdk_control_request(&mut self, request: JsonValue) -> Result<()> {
        let json = serde_json::to_string(&request)?;

        if let Some(ref tx) = self.ws_tx {
            tx.send(json)
                .await
                .map_err(|_| SdkError::WebSocketError("Write channel closed".into()))?;
            Ok(())
        } else {
            Err(SdkError::InvalidState {
                message: "WebSocket write channel not available".into(),
            })
        }
    }

    async fn send_sdk_control_response(&mut self, response: JsonValue) -> Result<()> {
        let control_response = serde_json::json!({
            "type": "control_response",
            "response": response
        });

        let json = serde_json::to_string(&control_response)?;

        if let Some(ref tx) = self.ws_tx {
            tx.send(json)
                .await
                .map_err(|_| SdkError::WebSocketError("Write channel closed".into()))?;
            Ok(())
        } else {
            Err(SdkError::InvalidState {
                message: "WebSocket write channel not available".into(),
            })
        }
    }

    fn take_sdk_control_receiver(&mut self) -> Option<mpsc::Receiver<JsonValue>> {
        self.sdk_control_rx.take()
    }

    fn is_connected(&self) -> bool {
        self.state == TransportState::Connected
    }

    async fn disconnect(&mut self) -> Result<()> {
        if self.state != TransportState::Connected {
            return Ok(());
        }

        self.state = TransportState::Disconnecting;

        // Signal all background tasks to stop
        if let Some(ref tx) = self.shutdown_tx {
            let _ = tx.send(true);
        }

        // Drop the write channel to close the write task
        self.ws_tx.take();
        self.shutdown_tx.take();

        self.state = TransportState::Disconnected;
        info!("WebSocket transport disconnected");
        Ok(())
    }

    async fn end_input(&mut self) -> Result<()> {
        // For WebSocket, ending input means closing the write channel
        self.ws_tx.take();
        Ok(())
    }
}

impl Drop for WebSocketTransport {
    fn drop(&mut self) {
        // Signal shutdown on drop
        if let Some(ref tx) = self.shutdown_tx {
            let _ = tx.send(true);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_websocket_config_default() {
        let config = WebSocketConfig::default();
        assert_eq!(config.max_reconnect_attempts, 3);
        assert_eq!(config.base_reconnect_delay_ms, 1000);
        assert_eq!(config.max_reconnect_delay_ms, 30000);
        assert_eq!(config.ping_interval_secs, 10);
        assert_eq!(config.message_buffer_capacity, 1000);
        assert!(config.auth_token.is_none());
    }

    #[test]
    fn test_websocket_transport_new_valid_url() {
        let transport =
            WebSocketTransport::new("ws://localhost:8765", WebSocketConfig::default());
        assert!(transport.is_ok());
        let transport = transport.unwrap();
        assert!(!transport.is_connected());
    }

    #[test]
    fn test_websocket_transport_new_wss_url() {
        let transport =
            WebSocketTransport::new("wss://example.com/ws", WebSocketConfig::default());
        assert!(transport.is_ok());
    }

    #[test]
    fn test_websocket_transport_new_invalid_scheme() {
        let transport =
            WebSocketTransport::new("http://localhost:8765", WebSocketConfig::default());
        assert!(transport.is_err());
        let err = transport.unwrap_err().to_string();
        assert!(err.contains("Unsupported URL scheme"));
    }

    #[test]
    fn test_websocket_transport_new_invalid_url() {
        let transport =
            WebSocketTransport::new("not a url at all", WebSocketConfig::default());
        assert!(transport.is_err());
    }

    #[tokio::test]
    async fn test_websocket_transport_send_before_connect() {
        let mut transport =
            WebSocketTransport::new("ws://localhost:9999", WebSocketConfig::default()).unwrap();

        let result = transport
            .send_message(InputMessage::user("hello".into(), "".into()))
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Not connected"));
    }

    #[tokio::test]
    async fn test_websocket_transport_disconnect_when_not_connected() {
        let mut transport =
            WebSocketTransport::new("ws://localhost:9999", WebSocketConfig::default()).unwrap();

        // Should be a no-op, not an error
        let result = transport.disconnect().await;
        assert!(result.is_ok());
    }
}
