//! Working interactive client implementation

use crate::{
    errors::{Result, SdkError},
    transport::{InputMessage, SubprocessTransport, Transport},
    types::{ClaudeCodeOptions, ControlRequest, Message},
};
use futures::{Stream, StreamExt};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_stream::wrappers::ReceiverStream;
use tracing::{debug, info};

/// Interactive client for stateful conversations with Claude
///
/// This is the recommended client for interactive use. It provides a clean API
/// that matches the Python SDK's functionality.
pub struct InteractiveClient {
    transport: Arc<Mutex<Box<dyn Transport + Send>>>,
    connected: bool,
}

impl InteractiveClient {
    /// Create a new client
    pub fn new(options: ClaudeCodeOptions) -> Result<Self> {
        unsafe {
            std::env::set_var("CLAUDE_CODE_ENTRYPOINT", "sdk-rust");
        }
        let transport: Box<dyn Transport + Send> = Box::new(SubprocessTransport::new(options)?);
        Ok(Self {
            transport: Arc::new(Mutex::new(transport)),
            connected: false,
        })
    }

    /// Connect to Claude
    pub async fn connect(&mut self) -> Result<()> {
        if self.connected {
            return Ok(());
        }

        let mut transport = self.transport.lock().await;
        transport.connect().await?;
        drop(transport); // Release lock immediately

        self.connected = true;
        info!("Connected to Claude CLI");
        Ok(())
    }

    /// Send a message and receive all messages until Result message
    pub async fn send_and_receive(&mut self, prompt: String) -> Result<Vec<Message>> {
        if !self.connected {
            return Err(SdkError::InvalidState {
                message: "Not connected".into(),
            });
        }

        // Send message
        {
            let mut transport = self.transport.lock().await;
            let message = InputMessage::user(prompt, "default".to_string());
            transport.send_message(message).await?;
        } // Lock released here

        debug!("Message sent, waiting for response");

        // Receive messages
        let mut messages = Vec::new();
        loop {
            // Try to get a message
            let msg_result = {
                let mut transport = self.transport.lock().await;
                let mut stream = transport.receive_messages();
                stream.next().await
            }; // Lock released here

            // Process the message
            if let Some(result) = msg_result {
                match result {
                    Ok(msg) => {
                        debug!("Received: {:?}", msg);
                        let is_result = matches!(msg, Message::Result { .. });
                        messages.push(msg);
                        if is_result {
                            break;
                        }
                    },
                    Err(e) => return Err(e),
                }
            } else {
                // No more messages, wait a bit
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            }
        }

        Ok(messages)
    }

    /// Send a message without waiting for response
    pub async fn send_message(&mut self, prompt: String) -> Result<()> {
        if !self.connected {
            return Err(SdkError::InvalidState {
                message: "Not connected".into(),
            });
        }

        let mut transport = self.transport.lock().await;
        let message = InputMessage::user(prompt, "default".to_string());
        transport.send_message(message).await?;
        drop(transport);

        debug!("Message sent");
        Ok(())
    }

    /// Receive messages until Result message (convenience method like Python SDK)
    pub async fn receive_response(&mut self) -> Result<Vec<Message>> {
        if !self.connected {
            return Err(SdkError::InvalidState {
                message: "Not connected".into(),
            });
        }

        let mut messages = Vec::new();
        loop {
            // Try to get a message
            let msg_result = {
                let mut transport = self.transport.lock().await;
                let mut stream = transport.receive_messages();
                stream.next().await
            }; // Lock released here

            // Process the message
            if let Some(result) = msg_result {
                match result {
                    Ok(msg) => {
                        debug!("Received: {:?}", msg);
                        let is_result = matches!(msg, Message::Result { .. });
                        messages.push(msg);
                        if is_result {
                            break;
                        }
                    },
                    Err(e) => return Err(e),
                }
            } else {
                // No more messages, wait a bit
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            }
        }

        Ok(messages)
    }

    /// Receive messages as a stream (streaming output support)
    ///
    /// Returns a stream of messages that can be iterated over asynchronously.
    /// This is similar to Python SDK's `receive_messages()` method.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use nexus_claude::{InteractiveClient, ClaudeCodeOptions};
    /// use futures::StreamExt;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let mut client = InteractiveClient::new(ClaudeCodeOptions::default())?;
    ///     client.connect().await?;
    ///     
    ///     // Send a message
    ///     client.send_message("Hello!".to_string()).await?;
    ///     
    ///     // Receive messages as a stream
    ///     let mut stream = client.receive_messages_stream().await;
    ///     while let Some(msg) = stream.next().await {
    ///         match msg {
    ///             Ok(message) => println!("Received: {:?}", message),
    ///             Err(e) => eprintln!("Error: {}", e),
    ///         }
    ///     }
    ///     
    ///     Ok(())
    /// }
    /// ```
    pub async fn receive_messages_stream(&mut self) -> impl Stream<Item = Result<Message>> + '_ {
        // Create a channel for messages
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        let transport = self.transport.clone();

        // Spawn a task to receive messages from transport
        tokio::spawn(async move {
            let mut transport = transport.lock().await;
            let mut stream = transport.receive_messages();

            while let Some(result) = stream.next().await {
                // Send each message through the channel
                if tx.send(result).await.is_err() {
                    // Receiver dropped, stop sending
                    break;
                }
            }
        });

        // Return the receiver as a stream
        ReceiverStream::new(rx)
    }

    /// Receive messages as an async iterator until a Result message
    ///
    /// This is a convenience method that collects messages until a Result message
    /// is received, similar to Python SDK's `receive_response()`.
    pub async fn receive_response_stream(&mut self) -> impl Stream<Item = Result<Message>> + '_ {
        // Create a stream that stops after Result message
        async_stream::stream! {
            let mut stream = self.receive_messages_stream().await;

            while let Some(result) = stream.next().await {
                match &result {
                    Ok(msg) => {
                        let is_result = matches!(msg, Message::Result { .. });
                        yield result;
                        if is_result {
                            break;
                        }
                    }
                    Err(_) => {
                        yield result;
                        break;
                    }
                }
            }
        }
    }

    /// Send interrupt signal to cancel current operation
    pub async fn interrupt(&mut self) -> Result<()> {
        if !self.connected {
            return Err(SdkError::InvalidState {
                message: "Not connected".into(),
            });
        }

        let mut transport = self.transport.lock().await;
        let request = ControlRequest::Interrupt {
            request_id: uuid::Uuid::new_v4().to_string(),
        };
        transport.send_control_request(request).await?;
        drop(transport);

        info!("Interrupt sent");
        Ok(())
    }

    /// Disconnect
    pub async fn disconnect(&mut self) -> Result<()> {
        if !self.connected {
            return Ok(());
        }

        let mut transport = self.transport.lock().await;
        transport.disconnect().await?;
        drop(transport);

        self.connected = false;
        info!("Disconnected from Claude CLI");
        Ok(())
    }
}
