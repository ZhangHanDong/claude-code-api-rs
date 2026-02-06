//! Interactive client for bidirectional communication with Claude
//!
//! This module provides the `ClaudeSDKClient` for interactive, stateful
//! conversations with Claude Code CLI.

use crate::{
    errors::{Result, SdkError},
    internal_query::Query,
    token_tracker::BudgetManager,
    transport::{InputMessage, SubprocessTransport, Transport},
    types::{ClaudeCodeOptions, ContentBlock, ControlRequest, ControlResponse, Message},
};
use futures::stream::{Stream, StreamExt};
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock, mpsc};
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
/// use nexus_claude::{ClaudeSDKClient, ClaudeCodeOptions, Message, Result};
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
    transport: Arc<Mutex<Box<dyn Transport + Send>>>,
    /// Internal query handler (when control protocol is enabled)
    query_handler: Option<Arc<Mutex<Query>>>,
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
    /// Budget manager for token tracking
    budget_manager: BudgetManager,
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
        unsafe {
            std::env::set_var("CLAUDE_CODE_ENTRYPOINT", "sdk-rust");
        }

        let transport = match SubprocessTransport::new(options.clone()) {
            Ok(t) => t,
            Err(e) => {
                error!("Failed to create transport: {}", e);
                // Create with empty path, will fail on connect
                SubprocessTransport::with_cli_path(options.clone(), "")
            },
        };

        // Wrap transport in Arc for sharing
        let transport_arc: Arc<Mutex<Box<dyn Transport + Send>>> =
            Arc::new(Mutex::new(Box::new(transport)));

        Self::with_transport_internal(options, transport_arc)
    }

    /// Create a new client with a custom transport implementation
    ///
    /// This allows users to provide their own Transport implementation instead of
    /// using the default SubprocessTransport. Useful for testing, custom CLI paths,
    /// or alternative communication mechanisms.
    ///
    /// # Arguments
    ///
    /// * `options` - Configuration options for the client
    /// * `transport` - Custom transport implementation
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use nexus_claude::{ClaudeSDKClient, ClaudeCodeOptions, SubprocessTransport};
    /// # fn example() {
    /// let options = ClaudeCodeOptions::default();
    /// let transport = SubprocessTransport::with_cli_path(options.clone(), "/custom/path/claude-code");
    /// let client = ClaudeSDKClient::with_transport(options, Box::new(transport));
    /// # }
    /// ```
    pub fn with_transport(
        options: ClaudeCodeOptions,
        transport: Box<dyn Transport + Send>,
    ) -> Self {
        // Set environment variable to indicate SDK usage
        unsafe {
            std::env::set_var("CLAUDE_CODE_ENTRYPOINT", "sdk-rust");
        }

        // Wrap transport in Arc for sharing
        let transport_arc: Arc<Mutex<Box<dyn Transport + Send>>> = Arc::new(Mutex::new(transport));

        Self::with_transport_internal(options, transport_arc)
    }

    /// Internal helper to construct client with pre-wrapped transport
    fn with_transport_internal(
        options: ClaudeCodeOptions,
        transport_arc: Arc<Mutex<Box<dyn Transport + Send>>>,
    ) -> Self {
        // Create query handler if control protocol features are enabled
        let query_handler = if options.can_use_tool.is_some()
            || options.hooks.is_some()
            || !options.mcp_servers.is_empty()
            || options.enable_file_checkpointing
        {
            // Extract SDK MCP server instances
            let sdk_mcp_servers: HashMap<String, Arc<dyn std::any::Any + Send + Sync>> = options
                .mcp_servers
                .iter()
                .filter_map(|(k, v)| {
                    // Only extract SDK type MCP servers
                    if let crate::types::McpServerConfig::Sdk { name: _, instance } = v {
                        Some((k.clone(), instance.clone()))
                    } else {
                        None
                    }
                })
                .collect();

            // Enable streaming mode when control protocol is active
            let is_streaming = options.can_use_tool.is_some()
                || options.hooks.is_some()
                || !sdk_mcp_servers.is_empty();

            let query = Query::new(
                transport_arc.clone(), // Share the same transport
                is_streaming,          // Enable streaming for control protocol
                options.can_use_tool.clone(),
                options.hooks.clone(),
                sdk_mcp_servers,
            );
            Some(Arc::new(Mutex::new(query)))
        } else {
            None
        };

        Self {
            options,
            transport: transport_arc,
            query_handler,
            state: Arc::new(RwLock::new(ClientState::Disconnected)),
            sessions: Arc::new(RwLock::new(HashMap::new())),
            message_tx: Arc::new(Mutex::new(None)),
            message_buffer: Arc::new(Mutex::new(Vec::new())),
            request_counter: Arc::new(Mutex::new(0)),
            budget_manager: BudgetManager::new(),
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

        // Initialize query handler if present
        if let Some(ref query_handler) = self.query_handler {
            let mut handler = query_handler.lock().await;
            handler.start().await?;
            handler.initialize().await?;
            info!("Initialized SDK control protocol");
        }

        // Update state
        {
            let mut state = self.state.write().await;
            *state = ClientState::Connected;
        }

        info!("Connected to Claude CLI");

        // Start message receiver task (always needed for regular messages)
        self.start_message_receiver().await;

        // Send initial prompt if provided
        if let Some(prompt) = initial_prompt {
            self.send_request(prompt, None).await?;
        }

        Ok(())
    }

    /// Send a user message to Claude
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
    pub async fn receive_messages(&mut self) -> impl Stream<Item = Result<Message>> + use<> {
        // Always use the regular message receiver
        // (Query handler shares the same transport and receives control messages separately)
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

        // If we have a query handler, use it
        if let Some(ref query_handler) = self.query_handler {
            let mut handler = query_handler.lock().await;
            return handler.interrupt().await;
        }

        // Otherwise use regular interrupt
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
                },
                Ok(Ok(None)) => Err(SdkError::ControlRequestError(
                    "No interrupt acknowledgment received".into(),
                )),
                Ok(Err(e)) => Err(e),
                Err(_) => Err(SdkError::timeout(5)),
            }
        });

        ack_task
            .await
            .map_err(|_| SdkError::ControlRequestError("Interrupt task panicked".into()))?
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

    /// Receive messages until and including a ResultMessage
    ///
    /// This is a convenience method that collects all messages from a single response.
    /// It will automatically stop after receiving a ResultMessage.
    pub async fn receive_response(
        &mut self,
    ) -> Pin<Box<dyn Stream<Item = Result<Message>> + Send + '_>> {
        let mut messages = self.receive_messages().await;

        // Create a stream that stops after ResultMessage
        Box::pin(async_stream::stream! {
            while let Some(msg_result) = messages.next().await {
                match &msg_result {
                    Ok(Message::Result { .. }) => {
                        yield msg_result;
                        return;
                    }
                    _ => {
                        yield msg_result;
                    }
                }
            }
        })
    }

    /// Get server information
    ///
    /// Returns initialization information from the Claude Code server including:
    /// - Available commands
    /// - Current and available output styles
    /// - Server capabilities
    pub async fn get_server_info(&self) -> Option<serde_json::Value> {
        // If we have a query handler with control protocol, get from there
        if let Some(ref query_handler) = self.query_handler {
            let handler = query_handler.lock().await;
            if let Some(init_result) = handler.get_initialization_result() {
                return Some(init_result.clone());
            }
        }

        // Otherwise check message buffer for init message
        let buffer = self.message_buffer.lock().await;
        for msg in buffer.iter() {
            if let Message::System { subtype, data } = msg
                && subtype == "init"
            {
                return Some(data.clone());
            }
        }
        None
    }

    /// Get account information
    ///
    /// This method attempts to retrieve Claude account information through multiple methods:
    /// 1. From environment variable `ANTHROPIC_USER_EMAIL`
    /// 2. From Claude CLI config file (if accessible)
    /// 3. By querying the CLI with `/status` command (interactive mode)
    ///
    /// # Returns
    ///
    /// A string containing the account information, or an error if unavailable.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use nexus_claude::{ClaudeSDKClient, ClaudeCodeOptions};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut client = ClaudeSDKClient::new(ClaudeCodeOptions::default());
    /// client.connect(None).await?;
    ///
    /// match client.get_account_info().await {
    ///     Ok(info) => println!("Account: {}", info),
    ///     Err(_) => println!("Account info not available"),
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Note
    ///
    /// Account information may not always be available in SDK mode.
    /// Consider setting the `ANTHROPIC_USER_EMAIL` environment variable
    /// for reliable account identification.
    pub async fn get_account_info(&mut self) -> Result<String> {
        // Check connection
        {
            let state = self.state.read().await;
            if *state != ClientState::Connected {
                return Err(SdkError::InvalidState {
                    message: "Not connected. Call connect() first.".into(),
                });
            }
        }

        // Method 1: Check environment variable
        if let Ok(email) = std::env::var("ANTHROPIC_USER_EMAIL") {
            return Ok(format!("Email: {}", email));
        }

        // Method 2: Try reading from Claude config
        if let Some(config_info) = Self::read_claude_config().await {
            return Ok(config_info);
        }

        // Method 3: Try /status command (may not work in SDK mode)
        self.send_user_message("/status".to_string()).await?;

        let mut messages = self.receive_messages().await;
        let mut account_info = String::new();

        while let Some(msg_result) = messages.next().await {
            match msg_result? {
                Message::Assistant { message } => {
                    for block in message.content {
                        if let ContentBlock::Text(text) = block {
                            account_info.push_str(&text.text);
                            account_info.push('\n');
                        }
                    }
                },
                Message::Result { .. } => break,
                _ => {},
            }
        }

        let trimmed = account_info.trim();

        // Check if we got actual status info or just a chat response
        if !trimmed.is_empty()
            && (trimmed.contains("account")
                || trimmed.contains("email")
                || trimmed.contains("subscription")
                || trimmed.contains("authenticated"))
        {
            return Ok(trimmed.to_string());
        }

        Err(SdkError::InvalidState {
            message: "Account information not available. Try setting ANTHROPIC_USER_EMAIL environment variable.".into(),
        })
    }

    /// Read Claude config file
    async fn read_claude_config() -> Option<String> {
        // Try common config locations
        let config_paths = vec![
            dirs::home_dir()?
                .join(".config")
                .join("claude")
                .join("config.json"),
            dirs::home_dir()?.join(".claude").join("config.json"),
        ];

        for path in config_paths {
            if let Ok(content) = tokio::fs::read_to_string(&path).await
                && let Ok(json) = serde_json::from_str::<serde_json::Value>(&content)
            {
                if let Some(email) = json.get("email").and_then(|v| v.as_str()) {
                    return Some(format!("Email: {}", email));
                }
                if let Some(user) = json.get("user").and_then(|v| v.as_str()) {
                    return Some(format!("User: {}", user));
                }
            }
        }

        None
    }

    /// Set permission mode dynamically
    ///
    /// Changes the permission mode during an active session.
    /// Requires control protocol to be enabled (via can_use_tool, hooks, mcp_servers, or file checkpointing).
    ///
    /// # Arguments
    ///
    /// * `mode` - Permission mode: "default", "acceptEdits", "plan", or "bypassPermissions"
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use nexus_claude::{ClaudeSDKClient, ClaudeCodeOptions};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut client = ClaudeSDKClient::new(ClaudeCodeOptions::default());
    /// client.connect(None).await?;
    ///
    /// // Switch to accept edits mode
    /// client.set_permission_mode("acceptEdits").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn set_permission_mode(&mut self, mode: &str) -> Result<()> {
        if let Some(ref query_handler) = self.query_handler {
            let mut handler = query_handler.lock().await;
            handler.set_permission_mode(mode).await
        } else {
            Err(SdkError::InvalidState {
                message: "Query handler not initialized. Enable control protocol features (can_use_tool, hooks, mcp_servers, or enable_file_checkpointing).".to_string(),
            })
        }
    }

    /// Set model dynamically
    ///
    /// Changes the active model during an active session.
    /// Requires control protocol to be enabled (via can_use_tool, hooks, mcp_servers, or file checkpointing).
    ///
    /// # Arguments
    ///
    /// * `model` - Model identifier (e.g., "claude-3-5-sonnet-20241022") or None to use default
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use nexus_claude::{ClaudeSDKClient, ClaudeCodeOptions};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut client = ClaudeSDKClient::new(ClaudeCodeOptions::default());
    /// client.connect(None).await?;
    ///
    /// // Switch to a different model
    /// client.set_model(Some("claude-3-5-sonnet-20241022".to_string())).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn set_model(&mut self, model: Option<String>) -> Result<()> {
        if let Some(ref query_handler) = self.query_handler {
            let mut handler = query_handler.lock().await;
            handler.set_model(model).await
        } else {
            Err(SdkError::InvalidState {
                message: "Query handler not initialized. Enable control protocol features (can_use_tool, hooks, mcp_servers, or enable_file_checkpointing).".to_string(),
            })
        }
    }

    /// Send a query with optional session ID
    ///
    /// This method is similar to Python SDK's query method in ClaudeSDKClient
    pub async fn query(&mut self, prompt: String, session_id: Option<String>) -> Result<()> {
        let session_id = session_id.unwrap_or_else(|| "default".to_string());

        // Send the message
        let message = InputMessage::user(prompt, session_id);

        {
            let mut transport = self.transport.lock().await;
            transport.send_message(message).await?;
        }

        Ok(())
    }

    /// Rewind tracked files to their state at a specific user message
    ///
    /// Requires `enable_file_checkpointing` to be enabled in `ClaudeCodeOptions`.
    /// This method allows you to undo file changes made during the session by
    /// reverting them to their state at any previous user message checkpoint.
    ///
    /// # Arguments
    ///
    /// * `user_message_id` - UUID of the user message to rewind to. This should be
    ///   the `uuid` field from a message received during the conversation.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use nexus_claude::{ClaudeSDKClient, ClaudeCodeOptions};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let options = ClaudeCodeOptions::builder()
    ///     .enable_file_checkpointing(true)
    ///     .build();
    /// let mut client = ClaudeSDKClient::new(options);
    /// client.connect(None).await?;
    ///
    /// // Ask Claude to make some changes
    /// client.send_request("Make some changes to my files".to_string(), None).await?;
    ///
    /// // ... later, rewind to a checkpoint
    /// // client.rewind_files("user-message-uuid-here").await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The client is not connected
    /// - The query handler is not initialized (control protocol required)
    /// - File checkpointing is not enabled
    /// - The specified user_message_id is invalid
    pub async fn rewind_files(&mut self, user_message_id: &str) -> Result<()> {
        // Check connection
        {
            let state = self.state.read().await;
            if *state != ClientState::Connected {
                return Err(SdkError::InvalidState {
                    message: "Not connected. Call connect() first.".into(),
                });
            }
        }

        if !self.options.enable_file_checkpointing {
            return Err(SdkError::InvalidState {
                message: "File checkpointing is not enabled. Set ClaudeCodeOptions::builder().enable_file_checkpointing(true).".to_string(),
            });
        }

        // Require query handler for control protocol
        if let Some(ref query_handler) = self.query_handler {
            let mut handler = query_handler.lock().await;
            handler.rewind_files(user_message_id).await
        } else {
            Err(SdkError::InvalidState {
                message: "Query handler not initialized. Enable control protocol features (can_use_tool, hooks, mcp_servers, or enable_file_checkpointing).".to_string(),
            })
        }
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
        let budget_manager = self.budget_manager.clone();

        tokio::spawn(async move {
            // Subscribe to messages without holding the lock
            let mut stream = {
                let mut transport = transport.lock().await;
                transport.receive_messages()
            }; // Lock is released here immediately

            while let Some(result) = stream.next().await {
                match result {
                    Ok(message) => {
                        // Update token usage for Result messages
                        if let Message::Result { .. } = &message
                            && let Message::Result {
                                usage,
                                total_cost_usd,
                                ..
                            } = &message
                        {
                            let (input_tokens, output_tokens) = if let Some(usage_json) = usage {
                                let input = usage_json
                                    .get("input_tokens")
                                    .and_then(|v| v.as_u64())
                                    .unwrap_or(0);
                                let output = usage_json
                                    .get("output_tokens")
                                    .and_then(|v| v.as_u64())
                                    .unwrap_or(0);
                                (input, output)
                            } else {
                                (0, 0)
                            };
                            let cost = total_cost_usd.unwrap_or(0.0);
                            budget_manager
                                .update_usage(input_tokens, output_tokens, cost)
                                .await;
                        }

                        // Buffer init messages for get_server_info()
                        if let Message::System { subtype, .. } = &message
                            && subtype == "init"
                        {
                            let mut buffer = message_buffer.lock().await;
                            buffer.push(message.clone());
                        }

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
                    },
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
                    },
                }
            }

            debug!("Message receiver task ended");
        });
    }

    /// Get token usage statistics
    ///
    /// Returns the current token usage tracker with cumulative statistics
    /// for all queries executed by this client.
    pub async fn get_usage_stats(&self) -> crate::token_tracker::TokenUsageTracker {
        self.budget_manager.get_usage().await
    }

    /// Set budget limit with optional warning callback
    ///
    /// # Arguments
    ///
    /// * `limit` - Budget limit configuration (cost and/or token caps)
    /// * `on_warning` - Optional callback function triggered when usage exceeds warning threshold
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use nexus_claude::{ClaudeSDKClient, ClaudeCodeOptions};
    /// use nexus_claude::token_tracker::{BudgetLimit, BudgetWarningCallback};
    /// use std::sync::Arc;
    ///
    /// # async fn example() {
    /// let mut client = ClaudeSDKClient::new(ClaudeCodeOptions::default());
    ///
    /// // Set budget with callback
    /// let cb: BudgetWarningCallback = Arc::new(|msg: &str| println!("Budget warning: {}", msg));
    /// client.set_budget_limit(BudgetLimit::with_cost(5.0), Some(cb)).await;
    /// # }
    /// ```
    pub async fn set_budget_limit(
        &self,
        limit: crate::token_tracker::BudgetLimit,
        on_warning: Option<crate::token_tracker::BudgetWarningCallback>,
    ) {
        self.budget_manager.set_limit(limit).await;
        if let Some(callback) = on_warning {
            self.budget_manager.set_warning_callback(callback).await;
        }
    }

    /// Clear budget limit and reset warning state
    pub async fn clear_budget_limit(&self) {
        self.budget_manager.clear_limit().await;
    }

    /// Reset token usage statistics to zero
    ///
    /// Clears all accumulated token and cost statistics.
    /// Budget limits remain in effect.
    pub async fn reset_usage_stats(&self) {
        self.budget_manager.reset_usage().await;
    }

    /// Check if budget has been exceeded
    ///
    /// Returns true if current usage exceeds any configured limits
    pub async fn is_budget_exceeded(&self) -> bool {
        self.budget_manager.is_exceeded().await
    }

    // Removed unused helper; usage is updated inline in message receiver
}

impl Drop for ClaudeSDKClient {
    fn drop(&mut self) {
        // Try to disconnect gracefully
        let transport = self.transport.clone();
        let state = self.state.clone();

        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_lifecycle() {
        let options = ClaudeCodeOptions::default();
        let client = ClaudeSDKClient::new(options);

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

    #[test]
    fn test_file_checkpointing_enables_query_handler() {
        let options = ClaudeCodeOptions::builder()
            .enable_file_checkpointing(true)
            .build();
        let client = ClaudeSDKClient::new(options);

        assert!(
            client.query_handler.is_some(),
            "enable_file_checkpointing should initialize the query handler for control protocol requests"
        );
    }
}
