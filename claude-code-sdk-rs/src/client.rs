//! Interactive client for bidirectional communication with Claude
//!
//! This module provides the `ClaudeSDKClient` for interactive, stateful
//! conversations with Claude Code CLI.

use crate::{
    errors::{Result, SdkError},
    transport::{InputMessage, SubprocessTransport, Transport},
    types::{ClaudeCodeOptions, ControlRequest, ControlResponse, Message},
};
use futures::stream::{Stream, StreamExt};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio_stream::wrappers::ReceiverStream;
use tracing::{debug, error, info};

/// Client state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientState {
    /// Not connected
    Disconnected,
    /// Connected and ready
    Connected,
    /// Error state
    Error,
}

/// Interactive client for bidirectional communication with Claude
///
/// `ClaudeSDKClient` provides a stateful, interactive interface for communicating
/// with Claude Code CLI. Unlike the simple `query` function, this client supports:
///
/// - Bidirectional communication
/// - Multiple sessions
/// - Interrupt capabilities
/// - State management
/// - Follow-up messages based on responses
///
/// # Example
///
/// ```rust,no_run
/// use cc_sdk::{ClaudeSDKClient, ClaudeCodeOptions, Message, Result};
/// use futures::StreamExt;
///
/// #[tokio::main]
/// async fn main() -> Result<()> {
///     let options = ClaudeCodeOptions::builder()
///         .system_prompt("You are a helpful assistant")
///         .model("claude-3-opus-20240229")
///         .build();
///
///     let mut client = ClaudeSDKClient::new(options);
///
///     // Connect with initial prompt
///     client.connect(Some("Hello!".to_string())).await?;
///
///     // Receive initial response
///     let mut messages = client.receive_messages().await;
///     while let Some(msg) = messages.next().await {
///         match msg? {
///             Message::Result { .. } => break,
///             msg => println!("{:?}", msg),
///         }
///     }
///
///     // Send follow-up
///     client.send_request("What's 2 + 2?".to_string(), None).await?;
///
///     // Receive response
///     let mut messages = client.receive_messages().await;
///     while let Some(msg) = messages.next().await {
///         println!("{:?}", msg?);
///     }
///
///     // Disconnect
///     client.disconnect().await?;
///
///     Ok(())
/// }
/// ```
pub struct ClaudeSDKClient {
    /// Configuration options
    #[allow(dead_code)]
    options: ClaudeCodeOptions,
    /// Transport layer
    transport: Arc<Mutex<SubprocessTransport>>,
    /// Client state
    state: Arc<RwLock<ClientState>>,
    /// Active sessions
    sessions: Arc<RwLock<HashMap<String, SessionData>>>,
    /// Message sender for current receiver
    message_tx: Arc<Mutex<Option<mpsc::Sender<Result<Message>>>>>,
    /// Message buffer for multiple receivers
    message_buffer: Arc<Mutex<Vec<Message>>>,
    /// Request counter
    request_counter: Arc<Mutex<u64>>,
}

/// Session data
#[allow(dead_code)]
struct SessionData {
    /// Session ID
    id: String,
    /// Number of messages sent
    message_count: usize,
    /// Creation time
    created_at: std::time::Instant,
}

impl ClaudeSDKClient {
    /// Create a new client with the given options
    pub fn new(options: ClaudeCodeOptions) -> Self {
        // Set environment variable to indicate SDK usage
        std::env::set_var("CLAUDE_CODE_ENTRYPOINT", "sdk-rust");

        let transport = match SubprocessTransport::new(options.clone()) {
            Ok(t) => t,
            Err(e) => {
                error!("Failed to create transport: {}", e);
                // Create with empty path, will fail on connect
                SubprocessTransport::with_cli_path(options.clone(), "")
            }
        };

        Self {
            options,
            transport: Arc::new(Mutex::new(transport)),
            state: Arc::new(RwLock::new(ClientState::Disconnected)),
            sessions: Arc::new(RwLock::new(HashMap::new())),
            message_tx: Arc::new(Mutex::new(None)),
            message_buffer: Arc::new(Mutex::new(Vec::new())),
            request_counter: Arc::new(Mutex::new(0)),
        }
    }

    /// Connect to Claude CLI with an optional initial prompt
    pub async fn connect(&mut self, initial_prompt: Option<String>) -> Result<()> {
        // Check if already connected
        {
            let state = self.state.read().await;
            if *state == ClientState::Connected {
                return Ok(());
            }
        }

        // Connect transport
        {
            let mut transport = self.transport.lock().await;
            transport.connect().await?;
        }

        // Update state
        {
            let mut state = self.state.write().await;
            *state = ClientState::Connected;
        }

        info!("Connected to Claude CLI");

        // Start message receiver task
        self.start_message_receiver().await;

        // Send initial prompt if provided
        if let Some(prompt) = initial_prompt {
            self.send_request(prompt, None).await?;
        }

        Ok(())
    }

    /// Send a user message to Claude
    pub async fn send_user_message(
        &mut self,
        prompt: String,
    ) -> Result<()> {
        // Check connection
        {
            let state = self.state.read().await;
            if *state != ClientState::Connected {
                return Err(SdkError::InvalidState {
                    message: "Not connected".into(),
                });
            }
        }

        // Use default session ID
        let session_id = "default".to_string();

        // Update session data
        {
            let mut sessions = self.sessions.write().await;
            let session = sessions.entry(session_id.clone()).or_insert_with(|| {
                debug!("Creating new session: {}", session_id);
                SessionData {
                    id: session_id.clone(),
                    message_count: 0,
                    created_at: std::time::Instant::now(),
                }
            });
            session.message_count += 1;
        }

        // Create and send message
        let message = InputMessage::user(prompt, session_id.clone());

        {
            let mut transport = self.transport.lock().await;
            transport.send_message(message).await?;
        }

        debug!("Sent request to Claude");
        Ok(())
    }

    /// Send a request to Claude (alias for send_user_message with optional session_id)
    pub async fn send_request(
        &mut self,
        prompt: String,
        _session_id: Option<String>,
    ) -> Result<()> {
        // For now, ignore session_id and use send_user_message
        self.send_user_message(prompt).await
    }

    /// Receive messages from Claude
    ///
    /// Returns a stream of messages. The stream will end when a Result message
    /// is received or the connection is closed.
    pub async fn receive_messages(&mut self) -> impl Stream<Item = Result<Message>> {
        // Create a new channel for this receiver
        let (tx, rx) = mpsc::channel(100);

        // Get buffered messages and clear buffer
        let buffered_messages = {
            let mut buffer = self.message_buffer.lock().await;
            std::mem::take(&mut *buffer)
        };

        // Send buffered messages to the new receiver
        let tx_clone = tx.clone();
        tokio::spawn(async move {
            for msg in buffered_messages {
                if tx_clone.send(Ok(msg)).await.is_err() {
                    break;
                }
            }
        });

        // Store the sender for the message receiver task
        {
            let mut message_tx = self.message_tx.lock().await;
            *message_tx = Some(tx);
        }

        ReceiverStream::new(rx)
    }

    /// Send an interrupt request
    pub async fn interrupt(&mut self) -> Result<()> {
        // Check connection
        {
            let state = self.state.read().await;
            if *state != ClientState::Connected {
                return Err(SdkError::InvalidState {
                    message: "Not connected".into(),
                });
            }
        }

        // Generate request ID
        let request_id = {
            let mut counter = self.request_counter.lock().await;
            *counter += 1;
            format!("interrupt_{}", *counter)
        };

        // Send interrupt request
        let request = ControlRequest::Interrupt {
            request_id: request_id.clone(),
        };

        {
            let mut transport = self.transport.lock().await;
            transport.send_control_request(request).await?;
        }

        info!("Sent interrupt request: {}", request_id);

        // Wait for acknowledgment (with timeout)
        let transport = self.transport.clone();
        let ack_task = tokio::spawn(async move {
            let mut transport = transport.lock().await;
            match tokio::time::timeout(
                std::time::Duration::from_secs(5),
                transport.receive_control_response(),
            )
            .await
            {
                Ok(Ok(Some(ControlResponse::InterruptAck {
                    request_id: ack_id,
                    success,
                }))) => {
                    if ack_id == request_id && success {
                        Ok(())
                    } else {
                        Err(SdkError::ControlRequestError(
                            "Interrupt not acknowledged successfully".into(),
                        ))
                    }
                }
                Ok(Ok(None)) => Err(SdkError::ControlRequestError(
                    "No interrupt acknowledgment received".into(),
                )),
                Ok(Err(e)) => Err(e),
                Err(_) => Err(SdkError::timeout(5)),
            }
        });

        ack_task.await.map_err(|_| {
            SdkError::ControlRequestError("Interrupt task panicked".into())
        })?
    }

    /// Check if the client is connected
    pub async fn is_connected(&self) -> bool {
        let state = self.state.read().await;
        *state == ClientState::Connected
    }

    /// Get active session IDs
    pub async fn get_sessions(&self) -> Vec<String> {
        let sessions = self.sessions.read().await;
        sessions.keys().cloned().collect()
    }

    /// Disconnect from Claude CLI
    pub async fn disconnect(&mut self) -> Result<()> {
        // Check if already disconnected
        {
            let state = self.state.read().await;
            if *state == ClientState::Disconnected {
                return Ok(());
            }
        }

        // Disconnect transport
        {
            let mut transport = self.transport.lock().await;
            transport.disconnect().await?;
        }

        // Update state
        {
            let mut state = self.state.write().await;
            *state = ClientState::Disconnected;
        }

        // Clear sessions
        {
            let mut sessions = self.sessions.write().await;
            sessions.clear();
        }

        info!("Disconnected from Claude CLI");
        Ok(())
    }

    /// Start the message receiver task
    async fn start_message_receiver(&mut self) {
        let transport = self.transport.clone();
        let message_tx = self.message_tx.clone();
        let message_buffer = self.message_buffer.clone();
        let state = self.state.clone();

        tokio::spawn(async move {
            let mut transport = transport.lock().await;
            let mut stream = transport.receive_messages();

            while let Some(result) = stream.next().await {
                match result {
                    Ok(message) => {
                        // Try to send to current receiver
                        let sent = {
                            let mut tx_opt = message_tx.lock().await;
                            if let Some(tx) = tx_opt.as_mut() {
                                tx.send(Ok(message.clone())).await.is_ok()
                            } else {
                                false
                            }
                        };

                        // If no receiver or send failed, buffer the message
                        if !sent {
                            let mut buffer = message_buffer.lock().await;
                            buffer.push(message);
                        }
                    }
                    Err(e) => {
                        error!("Error receiving message: {}", e);

                        // Send error to receiver if available
                        let mut tx_opt = message_tx.lock().await;
                        if let Some(tx) = tx_opt.as_mut() {
                            let _ = tx.send(Err(e)).await;
                        }

                        // Update state on error
                        let mut state = state.write().await;
                        *state = ClientState::Error;
                        break;
                    }
                }
            }

            debug!("Message receiver task ended");
        });
    }
}

impl Drop for ClaudeSDKClient {
    fn drop(&mut self) {
        // Try to disconnect gracefully
        let transport = self.transport.clone();
        let state = self.state.clone();

        tokio::spawn(async move {
            let state = state.read().await;
            if *state == ClientState::Connected {
                let mut transport = transport.lock().await;
                if let Err(e) = transport.disconnect().await {
                    debug!("Error disconnecting in drop: {}", e);
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_lifecycle() {
        let options = ClaudeCodeOptions::default();
        let mut client = ClaudeSDKClient::new(options);

        assert!(!client.is_connected().await);
        assert_eq!(client.get_sessions().await.len(), 0);
    }

    #[tokio::test]
    async fn test_client_state_transitions() {
        let options = ClaudeCodeOptions::default();
        let client = ClaudeSDKClient::new(options);

        let state = client.state.read().await;
        assert_eq!(*state, ClientState::Disconnected);
    }
}
