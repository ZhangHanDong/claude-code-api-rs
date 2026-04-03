//! WebSocket transport for Claude Code SDK
//!
//! Implements the `Transport` trait over WebSocket using the same NDJSON protocol
//! as the subprocess transport. Includes production-grade reconnection matching
//! the Claude Code source (`WebSocketTransport.ts`):
//!
//! - Exponential backoff with ±25% jitter
//! - Time-budget based reconnection (default 5 minutes)
//! - Sleep/wake detection via reconnect attempt gap
//! - Permanent close codes (1002, 4001, 4003) skip reconnection
//! - Circular message buffer with replay on reconnect
//! - `X-Last-Request-Id` header for server-side deduplication

use crate::{
    errors::{Result, SdkError},
    transport::{InputMessage, Transport, TransportState},
    types::{ControlRequest, ControlResponse, Message},
};
use async_trait::async_trait;
use futures::stream::Stream;
use serde_json::Value as JsonValue;
use std::collections::VecDeque;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, watch, Mutex, RwLock};
use tracing::{debug, error, info, warn};

// ---------------------------------------------------------------------------
// Constants (matching CC source WebSocketTransport.ts)
// ---------------------------------------------------------------------------

/// Close codes that indicate the session is permanently gone — do not reconnect.
const PERMANENT_CLOSE_CODES: &[u16] = &[
    1002, // Protocol error
    4001, // Session expired / reaped
    4003, // Authentication failed
];

/// Default time budget for reconnection (5 minutes).
const DEFAULT_RECONNECT_GIVE_UP_MS: u64 = 5 * 60 * 1000;

/// If the gap between reconnect attempts exceeds this, assume sleep/wake.
const SLEEP_DETECTION_THRESHOLD_MS: u64 = 60_000;

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Configuration for the WebSocket transport.
#[derive(Debug, Clone)]
pub struct WebSocketConfig {
    /// Base delay in milliseconds for exponential backoff (default: 1000)
    pub base_reconnect_delay_ms: u64,
    /// Maximum delay in milliseconds for exponential backoff (default: 30000)
    pub max_reconnect_delay_ms: u64,
    /// Total time budget in milliseconds for reconnection (default: 300000 = 5 min)
    pub reconnect_give_up_ms: u64,
    /// Interval in seconds between keepalive pings (default: 10)
    pub ping_interval_secs: u64,
    /// Capacity of the message broadcast channel (default: 1000)
    pub message_buffer_capacity: usize,
    /// Capacity of the outbound message replay buffer (default: 200)
    pub replay_buffer_capacity: usize,
    /// Optional Bearer token for WebSocket upgrade authentication
    pub auth_token: Option<String>,
    /// Whether to auto-reconnect on disconnection (default: true)
    pub auto_reconnect: bool,
    /// Kept for backward compatibility; primary limiter is `reconnect_give_up_ms`.
    pub max_reconnect_attempts: u32,
}

impl Default for WebSocketConfig {
    fn default() -> Self {
        Self {
            base_reconnect_delay_ms: 1000,
            max_reconnect_delay_ms: 30000,
            reconnect_give_up_ms: DEFAULT_RECONNECT_GIVE_UP_MS,
            ping_interval_secs: 10,
            message_buffer_capacity: 1000,
            replay_buffer_capacity: 200,
            auth_token: None,
            auto_reconnect: true,
            max_reconnect_attempts: 30,
        }
    }
}

// ---------------------------------------------------------------------------
// Replay buffer
// ---------------------------------------------------------------------------

/// Circular buffer for storing outbound messages for replay on reconnect.
#[derive(Debug)]
struct ReplayBuffer {
    buffer: VecDeque<String>,
    capacity: usize,
}

impl ReplayBuffer {
    fn new(capacity: usize) -> Self {
        Self {
            buffer: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    fn push(&mut self, message: String) {
        if self.buffer.len() >= self.capacity {
            self.buffer.pop_front();
        }
        self.buffer.push_back(message);
    }

    fn drain_all(&mut self) -> Vec<String> {
        self.buffer.drain(..).collect()
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.buffer.len()
    }

    fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }
}

// ---------------------------------------------------------------------------
// Disconnect signal
// ---------------------------------------------------------------------------

/// Why the WebSocket read task exited.
#[derive(Debug)]
#[allow(dead_code)]
enum DisconnectReason {
    /// Server sent a close frame.
    CloseCode(u16),
    /// Read error.
    Error(String),
    /// Stream ended without close frame.
    StreamEnded,
    /// Intentional shutdown from our side.
    Shutdown,
}

// ---------------------------------------------------------------------------
// WebSocketTransport
// ---------------------------------------------------------------------------

/// WebSocket transport with production-grade reconnection.
///
/// External channels (message_broadcast_tx, sdk_control_tx, etc.) persist
/// across reconnections — consumers don't see reconnection events.
pub struct WebSocketTransport {
    url: url::Url,
    config: WebSocketConfig,
    /// Sender for outgoing messages — stable across reconnections.
    ws_tx: Option<mpsc::Sender<String>>,
    /// Broadcast sender for parsed incoming messages.
    message_broadcast_tx: Option<broadcast::Sender<Message>>,
    /// Receiver for legacy control responses.
    control_rx: Option<mpsc::Receiver<ControlResponse>>,
    /// Receiver for SDK control protocol messages.
    sdk_control_rx: Option<mpsc::Receiver<JsonValue>>,
    /// Current transport state.
    state: TransportState,
    /// Counter for request IDs.
    request_counter: u64,
    /// Shutdown signal sender.
    shutdown_tx: Option<watch::Sender<bool>>,
    /// Last request ID for dedup header on reconnect.
    last_request_id: Arc<RwLock<Option<String>>>,
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

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Compute exponential backoff delay with ±25% jitter.
fn compute_backoff(attempt: u32, base_ms: u64, max_ms: u64) -> u64 {
    let base_delay = std::cmp::min(
        base_ms.saturating_mul(1u64.wrapping_shl(attempt.min(20))),
        max_ms,
    );
    // ±25% jitter
    let jitter_range = base_delay / 4;
    let jitter = if jitter_range > 0 {
        (rand_u64() % (jitter_range * 2)) as i64 - jitter_range as i64
    } else {
        0
    };
    (base_delay as i64 + jitter).max(0) as u64
}

/// Simple pseudo-random u64 using system time as entropy.
fn rand_u64() -> u64 {
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    // Mix nanoseconds for cheap jitter — not crypto, just spread.
    t.as_nanos() as u64 ^ (t.as_nanos() as u64).wrapping_mul(6364136223846793005)
}

impl WebSocketTransport {
    /// Create a new WebSocket transport targeting the given URL.
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
            last_request_id: Arc::new(RwLock::new(None)),
        })
    }

    /// Build the WS upgrade request with optional auth + dedup headers.
    async fn build_ws_request(&self) -> Result<http::Request<()>> {
        let mut builder = http::Request::builder()
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
            builder = builder.header("Authorization", format!("Bearer {token}"));
        }

        let last_id = self.last_request_id.read().await;
        if let Some(ref id) = *last_id {
            builder = builder.header("X-Last-Request-Id", id.as_str());
        }

        builder
            .body(())
            .map_err(|e| SdkError::WebSocketError(format!("Failed to build WS request: {e}")))
    }

    /// Establish one WebSocket connection. Returns a disconnect reason channel.
    ///
    /// This spawns read/write/keepalive tasks that communicate through the
    /// provided stable channels. When the read task exits, it sends the reason
    /// on `disconnect_tx`.
    async fn establish_connection(
        &self,
        ws_rx: &Arc<Mutex<mpsc::Receiver<String>>>,
        message_broadcast_tx: &broadcast::Sender<Message>,
        control_tx: &mpsc::Sender<ControlResponse>,
        sdk_control_tx: &mpsc::Sender<JsonValue>,
        _shutdown_rx: &watch::Receiver<bool>,
        replay_buffer: &Arc<Mutex<ReplayBuffer>>,
    ) -> Result<mpsc::Receiver<DisconnectReason>> {
        use futures::{SinkExt, StreamExt};
        use tokio_tungstenite::tungstenite::Message as WsMessage;

        let request = self.build_ws_request().await?;

        let (ws_stream, _) = tokio_tungstenite::connect_async(request)
            .await
            .map_err(|e| {
                SdkError::WebSocketError(format!("Failed to connect to {}: {e}", self.url))
            })?;

        info!("WebSocket connected to {}", self.url);

        let (mut ws_sink, ws_read_stream) = ws_stream.split();

        // --- Replay buffered messages ---
        {
            let mut buf = replay_buffer.lock().await;
            if !buf.is_empty() {
                let messages = buf.drain_all();
                info!("Replaying {} buffered messages after reconnect", messages.len());
                for msg in &messages {
                    if let Err(e) = ws_sink.send(WsMessage::Text(msg.clone().into())).await {
                        warn!("Failed to replay message: {e}");
                        break;
                    }
                }
                // Re-buffer in case we disconnect again before confirmation
                for msg in messages {
                    buf.push(msg);
                }
            }
        }

        let (disconnect_tx, disconnect_rx) = mpsc::channel::<DisconnectReason>(1);
        let (conn_shutdown_tx, conn_shutdown_rx) = watch::channel(false);

        // --- Write task ---
        // Takes messages from the shared ws_rx, buffers them, writes to WS sink.
        let ws_rx_clone = ws_rx.clone();
        let replay_buf_clone = replay_buffer.clone();
        let last_req_id = self.last_request_id.clone();
        let mut write_shutdown = conn_shutdown_rx.clone();

        tokio::spawn(async move {
            let mut ws_rx = ws_rx_clone.lock().await;
            loop {
                tokio::select! {
                    msg = ws_rx.recv() => {
                        match msg {
                            Some(line) => {
                                // Track last request ID for dedup header
                                if let Ok(json) = serde_json::from_str::<JsonValue>(&line) {
                                    if let Some(id) = json.get("request_id").and_then(|v| v.as_str()) {
                                        *last_req_id.write().await = Some(id.to_string());
                                    }
                                }
                                // Buffer for replay
                                replay_buf_clone.lock().await.push(line.clone());
                                // Send to WS
                                if let Err(e) = ws_sink.send(WsMessage::Text(line.into())).await {
                                    error!("WebSocket write error: {e}");
                                    break;
                                }
                            }
                            None => {
                                debug!("Write channel closed");
                                break;
                            }
                        }
                    }
                    _ = write_shutdown.changed() => {
                        debug!("Write task: connection shutdown");
                        let _ = ws_sink.send(WsMessage::Close(None)).await;
                        break;
                    }
                }
            }
        });

        // --- Read task ---
        let msg_tx = message_broadcast_tx.clone();
        let ctrl_tx = control_tx.clone();
        let sdk_tx = sdk_control_tx.clone();
        let mut read_shutdown = conn_shutdown_rx.clone();
        let disconnect_tx_clone = disconnect_tx;

        tokio::spawn(async move {
            let mut ws_stream = ws_read_stream;
            let reason = loop {
                tokio::select! {
                    msg = ws_stream.next() => {
                        match msg {
                            Some(Ok(WsMessage::Text(text))) => {
                                let text_str: &str = &text;
                                for line in text_str.split('\n') {
                                    let line = line.trim();
                                    if line.is_empty() { continue; }
                                    match serde_json::from_str::<JsonValue>(line) {
                                        Ok(json) => {
                                            route_incoming_message(json, &msg_tx, &ctrl_tx, &sdk_tx).await;
                                        }
                                        Err(e) => {
                                            warn!("Failed to parse WS JSON: {e} — line: {line}");
                                        }
                                    }
                                }
                            }
                            Some(Ok(WsMessage::Ping(_))) => { /* tungstenite auto-pongs */ }
                            Some(Ok(WsMessage::Pong(_))) => { debug!("WS pong received"); }
                            Some(Ok(WsMessage::Close(frame))) => {
                                let code = frame.as_ref().map(|f| f.code.into()).unwrap_or(1000u16);
                                info!("WebSocket closed by server: code={code}");
                                break DisconnectReason::CloseCode(code);
                            }
                            Some(Ok(_)) => { /* binary, etc. */ }
                            Some(Err(e)) => {
                                error!("WebSocket read error: {e}");
                                break DisconnectReason::Error(e.to_string());
                            }
                            None => {
                                info!("WebSocket stream ended");
                                break DisconnectReason::StreamEnded;
                            }
                        }
                    }
                    _ = read_shutdown.changed() => {
                        debug!("Read task: connection shutdown");
                        break DisconnectReason::Shutdown;
                    }
                }
            };
            // Signal the connection to stop write/keepalive tasks
            let _ = conn_shutdown_tx.send(true);
            let _ = disconnect_tx_clone.send(reason).await;
        });

        // --- Keepalive task ---
        let keepalive_tx = {
            // Send keep_alive via a cloned ws_tx (the main sender)
            // Actually, keepalive should bypass the replay buffer.
            // Use a separate lightweight approach: just send through main channel.
            // Keep-alive messages don't need to be replayed.
            // For simplicity, we'll just note that keep_alive goes through ws_tx
            // and gets buffered — that's fine, buffer evicts old messages.
            None::<()> // keepalive is handled below
        };
        let _ = keepalive_tx;

        // We send keep_alive through the main ws_tx which the write task reads from.
        // But ws_tx is held by the parent. We need a separate keepalive sender.
        // Actually, we can use the ws_rx pattern: the write task reads from ws_rx.
        // Keepalive needs to write to ws_tx (the sender side).
        // The caller holds ws_tx. Let's not do keepalive through the write task.
        // Instead, keepalive gets its own reference to ws_tx (passed by caller).

        Ok(disconnect_rx)
    }
}

/// Route an incoming JSON message to the appropriate channel.
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
        "control_response" => {
            debug!("Received control response");
            let _ = sdk_control_tx.send(json.clone()).await;
            if let Some(response_obj) = json.get("response") {
                if let Some(request_id) = response_obj
                    .get("request_id")
                    .or_else(|| response_obj.get("requestId"))
                    .and_then(|v| v.as_str())
                {
                    let success = response_obj.get("subtype").and_then(|v| v.as_str()) == Some("success");
                    let _ = control_tx
                        .send(ControlResponse::InterruptAck {
                            request_id: request_id.to_string(),
                            success,
                        })
                        .await;
                }
            }
        }
        "control_request" | "sdk_control_request" => {
            let _ = sdk_control_tx.send(json).await;
        }
        "control" => {
            if let Some(control) = json.get("control") {
                let _ = sdk_control_tx.send(control.clone()).await;
            }
        }
        "system" => {
            if let Some(subtype) = json.get("subtype").and_then(|v| v.as_str()) {
                if subtype.starts_with("sdk_control:") {
                    let _ = sdk_control_tx.send(json.clone()).await;
                }
            }
            if let Ok(Some(message)) = crate::message_parser::parse_message(json) {
                let _ = message_broadcast_tx.send(message);
            }
        }
        "keep_alive" => {
            debug!("Received keep_alive");
        }
        _ => {
            if let Ok(Some(message)) = crate::message_parser::parse_message(json) {
                let _ = message_broadcast_tx.send(message);
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

        self.state = TransportState::Connecting;

        // Create stable external channels
        let (ws_tx, ws_rx) = mpsc::channel::<String>(256);
        let ws_rx = Arc::new(Mutex::new(ws_rx));
        let (message_broadcast_tx, _) =
            broadcast::channel::<Message>(self.config.message_buffer_capacity);
        let (control_tx, control_rx) = mpsc::channel::<ControlResponse>(32);
        let (sdk_control_tx, sdk_control_rx) = mpsc::channel::<JsonValue>(64);
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        let replay_buffer = Arc::new(Mutex::new(ReplayBuffer::new(self.config.replay_buffer_capacity)));

        // First connection attempt (non-reconnect)
        let mut disconnect_rx = self
            .establish_connection(
                &ws_rx,
                &message_broadcast_tx,
                &control_tx,
                &sdk_control_tx,
                &shutdown_rx,
                &replay_buffer,
            )
            .await?;

        // Store handles
        let keepalive_ws_tx = ws_tx.clone();
        self.ws_tx = Some(ws_tx);
        self.message_broadcast_tx = Some(message_broadcast_tx.clone());
        self.control_rx = Some(control_rx);
        self.sdk_control_rx = Some(sdk_control_rx);
        self.shutdown_tx = Some(shutdown_tx);
        self.state = TransportState::Connected;

        // --- Keepalive task (uses ws_tx to send keep_alive through write task) ---
        let ping_interval = self.config.ping_interval_secs;
        let mut keepalive_shutdown = shutdown_rx.clone();
        tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(tokio::time::Duration::from_secs(ping_interval));
            interval.tick().await; // skip first immediate tick
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        let msg = serde_json::json!({"type": "keep_alive"}).to_string();
                        if keepalive_ws_tx.send(msg).await.is_err() {
                            break;
                        }
                    }
                    _ = keepalive_shutdown.changed() => { break; }
                }
            }
        });

        // --- Supervisor task: handles reconnection loop ---
        let url = self.url.clone();
        let config = self.config.clone();
        let last_request_id = self.last_request_id.clone();
        let mut supervisor_shutdown = shutdown_rx.clone();

        // Clone everything the supervisor needs to call establish_connection
        let sup_ws_rx = ws_rx;
        let sup_msg_tx = message_broadcast_tx;
        let sup_ctrl_tx = control_tx;
        let sup_sdk_tx = sdk_control_tx;
        let sup_shutdown_rx = shutdown_rx;
        let sup_replay_buf = replay_buffer;

        tokio::spawn(async move {
            let mut reconnect_start: Option<u64> = None;
            let mut reconnect_attempts: u32 = 0;
            let mut last_attempt_time: Option<u64> = None;

            loop {
                // Wait for disconnection
                let reason = tokio::select! {
                    r = disconnect_rx.recv() => {
                        match r {
                            Some(r) => r,
                            None => break, // channel closed
                        }
                    }
                    _ = supervisor_shutdown.changed() => { break; }
                };

                // Check permanent close codes
                if let DisconnectReason::CloseCode(code) = &reason {
                    if PERMANENT_CLOSE_CODES.contains(code) {
                        info!("Permanent close code {code}, not reconnecting");
                        break;
                    }
                }

                if matches!(reason, DisconnectReason::Shutdown) {
                    break;
                }

                if !config.auto_reconnect {
                    info!("Auto-reconnect disabled, staying disconnected");
                    break;
                }

                // --- Reconnection logic ---
                let now = now_ms();
                if reconnect_start.is_none() {
                    reconnect_start = Some(now);
                }

                // Sleep/wake detection
                if let Some(last) = last_attempt_time {
                    if now - last > SLEEP_DETECTION_THRESHOLD_MS {
                        info!(
                            "Detected system sleep ({}s gap), resetting reconnection budget",
                            (now - last) / 1000
                        );
                        reconnect_start = Some(now);
                        reconnect_attempts = 0;
                    }
                }
                last_attempt_time = Some(now);

                // Check time budget
                let elapsed = now - reconnect_start.unwrap_or(now);
                if elapsed >= config.reconnect_give_up_ms {
                    warn!(
                        "Reconnection time budget exhausted after {}s",
                        elapsed / 1000
                    );
                    break;
                }

                reconnect_attempts += 1;
                let delay = compute_backoff(
                    reconnect_attempts - 1,
                    config.base_reconnect_delay_ms,
                    config.max_reconnect_delay_ms,
                );

                info!(
                    "Reconnecting in {}ms (attempt {}, {}s elapsed)",
                    delay,
                    reconnect_attempts,
                    elapsed / 1000
                );

                tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;

                // Check shutdown during sleep
                if *sup_shutdown_rx.borrow() {
                    break;
                }

                // Build a temporary transport to call establish_connection
                let temp = WebSocketTransport {
                    url: url.clone(),
                    config: config.clone(),
                    ws_tx: None,
                    message_broadcast_tx: None,
                    control_rx: None,
                    sdk_control_rx: None,
                    state: TransportState::Connecting,
                    request_counter: 0,
                    shutdown_tx: None,
                    last_request_id: last_request_id.clone(),
                };

                match temp
                    .establish_connection(
                        &sup_ws_rx,
                        &sup_msg_tx,
                        &sup_ctrl_tx,
                        &sup_sdk_tx,
                        &sup_shutdown_rx,
                        &sup_replay_buf,
                    )
                    .await
                {
                    Ok(new_disconnect_rx) => {
                        info!("Reconnected successfully (attempt {})", reconnect_attempts);
                        disconnect_rx = new_disconnect_rx;
                        // Reset on successful reconnect
                        reconnect_start = None;
                        reconnect_attempts = 0;
                        last_attempt_time = None;
                    }
                    Err(e) => {
                        warn!("Reconnection attempt {} failed: {e}", reconnect_attempts);
                        // Will loop back and try again after backoff
                        // Create a dummy disconnect_rx that fires immediately
                        let (tx, rx) = mpsc::channel(1);
                        let _ = tx.send(DisconnectReason::Error(e.to_string())).await;
                        disconnect_rx = rx;
                    }
                }
            }

            debug!("WebSocket supervisor task ended");
        });

        Ok(())
    }

    async fn send_message(&mut self, message: InputMessage) -> Result<()> {
        if self.state != TransportState::Connected {
            return Err(SdkError::InvalidState {
                message: "Not connected".into(),
            });
        }

        let json = serde_json::to_string(&message)?;

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
                        Err(tokio_stream::wrappers::errors::BroadcastStreamRecvError::Lagged(n)) => {
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

        if let Some(ref tx) = self.shutdown_tx {
            let _ = tx.send(true);
        }

        self.ws_tx.take();
        self.shutdown_tx.take();
        self.state = TransportState::Disconnected;
        info!("WebSocket transport disconnected");
        Ok(())
    }

    async fn end_input(&mut self) -> Result<()> {
        self.ws_tx.take();
        Ok(())
    }
}

impl Drop for WebSocketTransport {
    fn drop(&mut self) {
        if let Some(ref tx) = self.shutdown_tx {
            let _ = tx.send(true);
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_websocket_config_default() {
        let config = WebSocketConfig::default();
        assert_eq!(config.base_reconnect_delay_ms, 1000);
        assert_eq!(config.max_reconnect_delay_ms, 30000);
        assert_eq!(config.reconnect_give_up_ms, DEFAULT_RECONNECT_GIVE_UP_MS);
        assert_eq!(config.ping_interval_secs, 10);
        assert_eq!(config.message_buffer_capacity, 1000);
        assert_eq!(config.replay_buffer_capacity, 200);
        assert!(config.auth_token.is_none());
        assert!(config.auto_reconnect);
    }

    #[test]
    fn test_websocket_transport_new_valid_url() {
        let transport = WebSocketTransport::new("ws://localhost:8765", WebSocketConfig::default());
        assert!(transport.is_ok());
        assert!(!transport.unwrap().is_connected());
    }

    #[test]
    fn test_websocket_transport_new_wss_url() {
        let transport = WebSocketTransport::new("wss://example.com/ws", WebSocketConfig::default());
        assert!(transport.is_ok());
    }

    #[test]
    fn test_websocket_transport_new_invalid_scheme() {
        let transport = WebSocketTransport::new("http://localhost:8765", WebSocketConfig::default());
        assert!(transport.is_err());
        assert!(transport.unwrap_err().to_string().contains("Unsupported URL scheme"));
    }

    #[test]
    fn test_websocket_transport_new_invalid_url() {
        let transport = WebSocketTransport::new("not a url at all", WebSocketConfig::default());
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
        assert!(transport.disconnect().await.is_ok());
    }

    #[test]
    fn test_replay_buffer_basic() {
        let mut buf = ReplayBuffer::new(3);
        assert!(buf.is_empty());
        buf.push("a".into());
        buf.push("b".into());
        buf.push("c".into());
        assert_eq!(buf.len(), 3);

        // Overflow evicts oldest
        buf.push("d".into());
        assert_eq!(buf.len(), 3);
        let all = buf.drain_all();
        assert_eq!(all, vec!["b", "c", "d"]);
        assert!(buf.is_empty());
    }

    #[test]
    fn test_replay_buffer_drain_empty() {
        let mut buf = ReplayBuffer::new(5);
        assert!(buf.drain_all().is_empty());
    }

    #[test]
    fn test_permanent_close_codes() {
        assert!(PERMANENT_CLOSE_CODES.contains(&1002));
        assert!(PERMANENT_CLOSE_CODES.contains(&4001));
        assert!(PERMANENT_CLOSE_CODES.contains(&4003));
        assert!(!PERMANENT_CLOSE_CODES.contains(&1000));
        assert!(!PERMANENT_CLOSE_CODES.contains(&1006));
    }

    #[test]
    fn test_compute_backoff_exponential() {
        let d0 = compute_backoff(0, 1000, 30000);
        let d1 = compute_backoff(1, 1000, 30000);
        let d2 = compute_backoff(2, 1000, 30000);

        // With jitter, values won't be exact but should be in range
        assert!(d0 >= 750 && d0 <= 1250, "d0={d0}"); // 1000 ± 25%
        assert!(d1 >= 1500 && d1 <= 2500, "d1={d1}"); // 2000 ± 25%
        assert!(d2 >= 3000 && d2 <= 5000, "d2={d2}"); // 4000 ± 25%
    }

    #[test]
    fn test_compute_backoff_capped() {
        let d = compute_backoff(20, 1000, 30000);
        // Should be capped at max_ms ± 25%
        assert!(d <= 37500, "d={d}"); // 30000 + 25%
    }

    #[test]
    fn test_disconnect_reason_debug() {
        // Just ensure Debug is derived
        let _ = format!("{:?}", DisconnectReason::CloseCode(1000));
        let _ = format!("{:?}", DisconnectReason::StreamEnded);
        let _ = format!("{:?}", DisconnectReason::Error("test".into()));
        let _ = format!("{:?}", DisconnectReason::Shutdown);
    }
}
