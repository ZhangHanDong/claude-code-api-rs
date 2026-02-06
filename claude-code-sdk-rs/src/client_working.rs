//! A working interactive client implementation

use crate::{
    errors::{Result, SdkError},
    transport::{InputMessage, SubprocessTransport, Transport},
    types::{ClaudeCodeOptions, Message},
};
use futures::StreamExt;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock, mpsc};
use tracing::{debug, error, info};

/// Client state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientState {
    Disconnected,
    Connected,
    Error,
}

/// Working interactive client
pub struct ClaudeSDKClientWorking {
    /// Configuration options
    options: ClaudeCodeOptions,
    /// Transport wrapped in Arc<Mutex<>> for shared access
    transport: Arc<Mutex<Option<SubprocessTransport>>>,
    /// Channel to receive messages
    message_rx: Arc<Mutex<Option<mpsc::Receiver<Message>>>>,
    /// Client state
    state: Arc<RwLock<ClientState>>,
}

impl ClaudeSDKClientWorking {
    /// Create a new client
    pub fn new(options: ClaudeCodeOptions) -> Self {
        unsafe {
            std::env::set_var("CLAUDE_CODE_ENTRYPOINT", "sdk-rust");
        }

        Self {
            options,
            transport: Arc::new(Mutex::new(None)),
            message_rx: Arc::new(Mutex::new(None)),
            state: Arc::new(RwLock::new(ClientState::Disconnected)),
        }
    }

    /// Connect to Claude
    pub async fn connect(&mut self, initial_prompt: Option<String>) -> Result<()> {
        // Check if already connected
        {
            let state = self.state.read().await;
            if *state == ClientState::Connected {
                return Ok(());
            }
        }

        // Create transport
        let mut new_transport = SubprocessTransport::new(self.options.clone())?;
        new_transport.connect().await?;

        // Create message channel
        let (tx, rx) = mpsc::channel::<Message>(100);

        // Store transport
        {
            let mut transport = self.transport.lock().await;
            *transport = Some(new_transport);
        }

        // Store receiver
        {
            let mut message_rx = self.message_rx.lock().await;
            *message_rx = Some(rx);
        }

        // Update state
        {
            let mut state = self.state.write().await;
            *state = ClientState::Connected;
        }

        // Start background task to read messages
        let transport_clone = self.transport.clone();
        let state_clone = self.state.clone();
        let tx_clone = tx.clone();

        tokio::spawn(async move {
            loop {
                // Get one message at a time
                let msg_result = {
                    let mut transport_guard = transport_clone.lock().await;
                    if let Some(transport) = transport_guard.as_mut() {
                        // Get the stream and immediately poll it once
                        let mut stream = transport.receive_messages();
                        stream.next().await
                    } else {
                        break;
                    }
                };

                // Process the message if we got one
                if let Some(result) = msg_result {
                    match result {
                        Ok(msg) => {
                            debug!("Received message: {:?}", msg);
                            if tx_clone.send(msg).await.is_err() {
                                break;
                            }
                        },
                        Err(e) => {
                            error!("Error receiving message: {}", e);
                            let mut state = state_clone.write().await;
                            *state = ClientState::Error;
                            break;
                        },
                    }
                } else {
                    // No message available, wait a bit
                    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                }

                // Stream ended, check if we should reconnect
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

                let should_continue = {
                    let state = state_clone.read().await;
                    *state == ClientState::Connected
                };

                if !should_continue {
                    break;
                }
            }

            debug!("Message reader task ended");
        });

        info!("Connected to Claude CLI");

        // Send initial prompt if provided
        if let Some(prompt) = initial_prompt {
            self.send_user_message(prompt).await?;
        }

        Ok(())
    }

    /// Send a user message
    pub async fn send_user_message(&mut self, prompt: String) -> Result<()> {
        // Check connection
        {
            let state = self.state.read().await;
            if *state != ClientState::Connected {
                return Err(SdkError::InvalidState {
                    message: "Not connected".into(),
                });
            }
        }

        // Create message
        let message = InputMessage::user(prompt, "default".to_string());

        // Send message
        {
            let mut transport_guard = self.transport.lock().await;
            if let Some(transport) = transport_guard.as_mut() {
                transport.send_message(message).await?;
                debug!("User message sent");
            } else {
                return Err(SdkError::InvalidState {
                    message: "Transport not available".into(),
                });
            }
        }

        Ok(())
    }

    /// Receive next message
    pub async fn receive_message(&mut self) -> Result<Option<Message>> {
        let mut rx_guard = self.message_rx.lock().await;
        if let Some(rx) = rx_guard.as_mut() {
            Ok(rx.recv().await)
        } else {
            Err(SdkError::InvalidState {
                message: "Not connected".into(),
            })
        }
    }

    /// Receive all messages until result
    pub async fn receive_response(&mut self) -> Result<Vec<Message>> {
        let mut messages = Vec::new();

        while let Some(msg) = self.receive_message().await? {
            let is_result = matches!(msg, Message::Result { .. });
            messages.push(msg);
            if is_result {
                break;
            }
        }

        Ok(messages)
    }

    /// Disconnect
    pub async fn disconnect(&mut self) -> Result<()> {
        // Update state
        {
            let mut state = self.state.write().await;
            if *state == ClientState::Disconnected {
                return Ok(());
            }
            *state = ClientState::Disconnected;
        }

        // Disconnect transport
        {
            let mut transport_guard = self.transport.lock().await;
            if let Some(mut transport) = transport_guard.take() {
                transport.disconnect().await?;
            }
        }

        // Clear receiver
        {
            let mut rx_guard = self.message_rx.lock().await;
            rx_guard.take();
        }

        info!("Disconnected from Claude CLI");
        Ok(())
    }

    /// Check if connected
    pub async fn is_connected(&self) -> bool {
        let state = self.state.read().await;
        *state == ClientState::Connected
    }
}
