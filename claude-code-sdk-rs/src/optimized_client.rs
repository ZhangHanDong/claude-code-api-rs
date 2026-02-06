//! Optimized client implementation with performance improvements

use crate::token_tracker::{BudgetLimit, BudgetManager, BudgetWarningCallback, TokenUsageTracker};
use crate::{
    errors::{Result, SdkError},
    transport::{InputMessage, SubprocessTransport, Transport},
    types::{ClaudeCodeOptions, ControlRequest, Message},
};
use futures::stream::StreamExt;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::{RwLock, Semaphore, mpsc};
use tokio::time::{Duration, timeout};
use tracing::{debug, error, info, warn};

/// Client mode for different usage patterns
#[derive(Debug, Clone, Copy)]
pub enum ClientMode {
    /// One-shot query mode (stateless)
    OneShot,
    /// Interactive mode (stateful conversations)
    Interactive,
    /// Batch processing mode
    Batch {
        /// Maximum number of concurrent requests
        max_concurrent: usize,
    },
}

/// Connection pool for reusing subprocess transports
struct ConnectionPool {
    /// Available idle connections
    idle_connections: Arc<RwLock<VecDeque<Box<dyn Transport + Send>>>>,
    /// Maximum number of connections
    max_connections: usize,
    /// Semaphore for limiting concurrent connections
    connection_semaphore: Arc<Semaphore>,
    /// Base options for creating new connections
    base_options: ClaudeCodeOptions,
}

impl ConnectionPool {
    fn new(base_options: ClaudeCodeOptions, max_connections: usize) -> Self {
        Self {
            idle_connections: Arc::new(RwLock::new(VecDeque::new())),
            max_connections,
            connection_semaphore: Arc::new(Semaphore::new(max_connections)),
            base_options,
        }
    }

    async fn acquire(&self) -> Result<Box<dyn Transport + Send>> {
        // Try to get an idle connection first
        {
            let mut idle = self.idle_connections.write().await;
            if let Some(transport) = idle.pop_front() {
                // Verify connection is still valid
                if transport.is_connected() {
                    debug!("Reusing existing connection from pool");
                    return Ok(transport);
                }
            }
        }

        // Create new connection if under limit
        let _permit =
            self.connection_semaphore
                .acquire()
                .await
                .map_err(|_| SdkError::InvalidState {
                    message: "Failed to acquire connection permit".into(),
                })?;

        let mut transport: Box<dyn Transport + Send> =
            Box::new(SubprocessTransport::new(self.base_options.clone())?);
        transport.connect().await?;
        debug!("Created new connection");
        Ok(transport)
    }

    async fn release(&self, transport: Box<dyn Transport + Send>) {
        if transport.is_connected()
            && self.idle_connections.read().await.len() < self.max_connections
        {
            let mut idle = self.idle_connections.write().await;
            idle.push_back(transport);
            debug!("Returned connection to pool");
        } else {
            // Connection is invalid or pool is full, let it drop
            debug!("Dropping connection");
        }
    }
}

/// Optimized client with improved performance characteristics
pub struct OptimizedClient {
    /// Client mode
    mode: ClientMode,
    /// Connection pool
    pool: Arc<ConnectionPool>,
    /// Message receiver for interactive mode
    message_rx: Arc<RwLock<Option<mpsc::Receiver<Message>>>>,
    /// Current transport for interactive mode
    current_transport: Arc<RwLock<Option<Box<dyn Transport + Send>>>>,
    /// Budget manager for token/cost tracking
    budget_manager: BudgetManager,
}

impl OptimizedClient {
    /// Create a new optimized client
    pub fn new(options: ClaudeCodeOptions, mode: ClientMode) -> Result<Self> {
        unsafe {
            std::env::set_var("CLAUDE_CODE_ENTRYPOINT", "sdk-rust");
        }

        let max_connections = match mode {
            ClientMode::Batch { max_concurrent } => max_concurrent,
            _ => 1,
        };

        let pool = Arc::new(ConnectionPool::new(options, max_connections));

        Ok(Self {
            mode,
            pool,
            message_rx: Arc::new(RwLock::new(None)),
            current_transport: Arc::new(RwLock::new(None)),
            budget_manager: BudgetManager::new(),
        })
    }

    /// Execute a one-shot query with automatic retry
    pub async fn query(&self, prompt: String) -> Result<Vec<Message>> {
        self.query_with_retry(prompt, 3, Duration::from_millis(100))
            .await
    }

    /// Execute a query with custom retry configuration
    pub async fn query_with_retry(
        &self,
        prompt: String,
        max_retries: u32,
        initial_delay: Duration,
    ) -> Result<Vec<Message>> {
        let mut retries = 0;
        let mut delay = initial_delay;

        loop {
            match self.execute_query(&prompt).await {
                Ok(messages) => return Ok(messages),
                Err(e) if retries < max_retries => {
                    warn!("Query failed, retrying in {:?}: {}", delay, e);
                    tokio::time::sleep(delay).await;
                    retries += 1;
                    delay *= 2; // Exponential backoff
                },
                Err(e) => return Err(e),
            }
        }
    }

    /// Internal query execution
    async fn execute_query(&self, prompt: &str) -> Result<Vec<Message>> {
        let mut transport = self.pool.acquire().await?;

        // Send message
        let message = InputMessage::user(prompt.to_string(), "default".to_string());
        transport.send_message(message).await?;

        // Collect response with timeout
        let timeout_duration = Duration::from_secs(120);
        let messages = timeout(timeout_duration, self.collect_messages(&mut *transport))
            .await
            .map_err(|_| SdkError::Timeout { seconds: 120 })??;

        // Return transport to pool
        self.pool.release(transport).await;

        Ok(messages)
    }

    /// Collect messages until Result message
    async fn collect_messages<T: Transport + Send + ?Sized>(
        &self,
        transport: &mut T,
    ) -> Result<Vec<Message>> {
        let mut messages = Vec::new();
        let mut stream = transport.receive_messages();

        while let Some(result) = stream.next().await {
            match result {
                Ok(msg) => {
                    debug!("Received: {:?}", msg);
                    let is_result = matches!(msg, Message::Result { .. });

                    // Update budget/usage on result messages
                    if let Message::Result {
                        usage,
                        total_cost_usd,
                        ..
                    } = &msg
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
                        self.budget_manager
                            .update_usage(input_tokens, output_tokens, cost)
                            .await;
                    }
                    messages.push(msg);
                    if is_result {
                        break;
                    }
                },
                Err(e) => return Err(e),
            }
        }

        Ok(messages)
    }

    /// Get token/cost usage statistics
    pub async fn get_usage_stats(&self) -> TokenUsageTracker {
        self.budget_manager.get_usage().await
    }

    /// Set budget limit with optional warning callback
    ///
    /// Example:
    /// ```rust,no_run
    /// use nexus_claude::{OptimizedClient, ClaudeCodeOptions, ClientMode};
    /// use nexus_claude::token_tracker::{BudgetLimit, BudgetWarningCallback};
    /// use std::sync::Arc;
    /// # async fn demo() -> nexus_claude::Result<()> {
    /// let client = OptimizedClient::new(ClaudeCodeOptions::default(), ClientMode::OneShot)?;
    /// let cb: BudgetWarningCallback = Arc::new(|msg: &str| println!("Warn: {}", msg));
    /// client.set_budget_limit(BudgetLimit::with_cost(1.0), Some(cb)).await;
    /// # Ok(()) }
    /// ```
    pub async fn set_budget_limit(
        &self,
        limit: BudgetLimit,
        on_warning: Option<BudgetWarningCallback>,
    ) {
        self.budget_manager.set_limit(limit).await;
        if let Some(cb) = on_warning {
            self.budget_manager.set_warning_callback(cb).await;
        }
    }

    /// Clear budget limit and reset warning state
    pub async fn clear_budget_limit(&self) {
        self.budget_manager.clear_limit().await;
    }

    /// Reset usage statistics to zero
    pub async fn reset_usage_stats(&self) {
        self.budget_manager.reset_usage().await;
    }

    /// Check whether budget is exceeded
    pub async fn is_budget_exceeded(&self) -> bool {
        self.budget_manager.is_exceeded().await
    }

    /// Start an interactive session
    pub async fn start_interactive_session(&self) -> Result<()> {
        if !matches!(self.mode, ClientMode::Interactive) {
            return Err(SdkError::InvalidState {
                message: "Client not in interactive mode".into(),
            });
        }

        // Acquire a transport for the session
        let transport = self.pool.acquire().await?;

        // Create message channel
        let (tx, rx) = mpsc::channel::<Message>(100);

        // Store transport and receiver
        *self.current_transport.write().await = Some(transport);
        *self.message_rx.write().await = Some(rx);

        // Start background message processor
        self.start_message_processor(tx).await;

        info!("Interactive session started");
        Ok(())
    }

    /// Start background task to process messages
    async fn start_message_processor(&self, tx: mpsc::Sender<Message>) {
        let transport_ref = self.current_transport.clone();

        tokio::spawn(async move {
            loop {
                // Get message from transport
                let msg_result = {
                    let mut transport_guard = transport_ref.write().await;
                    if let Some(transport) = transport_guard.as_mut() {
                        let mut stream = transport.receive_messages();
                        stream.next().await
                    } else {
                        break;
                    }
                };

                // Process message
                if let Some(result) = msg_result {
                    match result {
                        Ok(msg) => {
                            if tx.send(msg).await.is_err() {
                                error!("Failed to send message to channel");
                                break;
                            }
                        },
                        Err(e) => {
                            error!("Error receiving message: {}", e);
                            break;
                        },
                    }
                }
            }
        });
    }

    /// Send a message in interactive mode
    pub async fn send_interactive(&self, prompt: String) -> Result<()> {
        let transport_guard = self.current_transport.read().await;
        if let Some(_transport) = transport_guard.as_ref() {
            // Need to handle transport mutability properly
            drop(transport_guard);

            let mut transport_guard = self.current_transport.write().await;
            if let Some(transport) = transport_guard.as_mut() {
                let message = InputMessage::user(prompt, "default".to_string());
                transport.send_message(message).await?;
            } else {
                return Err(SdkError::InvalidState {
                    message: "Transport lost during operation".into(),
                });
            }
            Ok(())
        } else {
            Err(SdkError::InvalidState {
                message: "No active interactive session".into(),
            })
        }
    }

    /// Receive messages in interactive mode
    pub async fn receive_interactive(&self) -> Result<Vec<Message>> {
        let mut rx_guard = self.message_rx.write().await;
        if let Some(rx) = rx_guard.as_mut() {
            let mut messages = Vec::new();

            // Collect messages until Result
            while let Some(msg) = rx.recv().await {
                let is_result = matches!(msg, Message::Result { .. });
                messages.push(msg);
                if is_result {
                    break;
                }
            }

            Ok(messages)
        } else {
            Err(SdkError::InvalidState {
                message: "No active interactive session".into(),
            })
        }
    }

    /// Process a batch of queries concurrently
    pub async fn process_batch(&self, prompts: Vec<String>) -> Result<Vec<Result<Vec<Message>>>> {
        let max_concurrent = match self.mode {
            ClientMode::Batch { max_concurrent } => max_concurrent,
            _ => {
                return Err(SdkError::InvalidState {
                    message: "Client not in batch mode".into(),
                });
            },
        };

        let semaphore = Arc::new(Semaphore::new(max_concurrent));
        let mut handles = Vec::new();

        for prompt in prompts {
            let permit = semaphore.clone().acquire_owned().await.unwrap();
            let client = self.clone(); // Assume client is cloneable

            let handle = tokio::spawn(async move {
                let result = client.query(prompt).await;
                drop(permit);
                result
            });

            handles.push(handle);
        }

        // Collect results
        let mut results = Vec::new();
        for handle in handles {
            match handle.await {
                Ok(result) => results.push(result),
                Err(e) => results.push(Err(SdkError::TransportError(format!("Task failed: {e}")))),
            }
        }

        Ok(results)
    }

    /// Send interrupt signal
    pub async fn interrupt(&self) -> Result<()> {
        let transport_guard = self.current_transport.read().await;
        if let Some(_transport) = transport_guard.as_ref() {
            drop(transport_guard);

            let mut transport_guard = self.current_transport.write().await;
            if let Some(transport) = transport_guard.as_mut() {
                let request = ControlRequest::Interrupt {
                    request_id: uuid::Uuid::new_v4().to_string(),
                };
                transport.send_control_request(request).await?;
            } else {
                return Err(SdkError::InvalidState {
                    message: "Transport lost during operation".into(),
                });
            }
            info!("Interrupt sent");
            Ok(())
        } else {
            Err(SdkError::InvalidState {
                message: "No active session".into(),
            })
        }
    }

    /// End interactive session
    pub async fn end_interactive_session(&self) -> Result<()> {
        // Clear current transport
        if let Some(transport) = self.current_transport.write().await.take() {
            self.pool.release(transport).await;
        }

        // Clear message receiver
        *self.message_rx.write().await = None;

        info!("Interactive session ended");
        Ok(())
    }
}

// Implement Clone if needed (this is a simplified version)
impl Clone for OptimizedClient {
    fn clone(&self) -> Self {
        Self {
            mode: self.mode,
            pool: self.pool.clone(),
            message_rx: Arc::new(RwLock::new(None)),
            current_transport: Arc::new(RwLock::new(None)),
            budget_manager: self.budget_manager.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_mode_creation() {
        let options = ClaudeCodeOptions::builder().build();

        // Test OneShot mode
        let client = OptimizedClient::new(options.clone(), ClientMode::OneShot);
        assert!(client.is_ok());

        // Test Interactive mode
        let client = OptimizedClient::new(options.clone(), ClientMode::Interactive);
        assert!(client.is_ok());

        // Test Batch mode
        let client = OptimizedClient::new(options, ClientMode::Batch { max_concurrent: 5 });
        assert!(client.is_ok());
    }

    #[test]
    fn test_connection_pool_creation() {
        let options = ClaudeCodeOptions::builder().build();
        let pool = ConnectionPool::new(options, 10);

        assert_eq!(pool.max_connections, 10);
    }

    #[tokio::test]
    async fn test_client_cloning() {
        let options = ClaudeCodeOptions::builder().build();
        let client = OptimizedClient::new(options, ClientMode::OneShot).unwrap();

        let cloned = client.clone();

        // Verify mode is preserved
        match (client.mode, cloned.mode) {
            (ClientMode::OneShot, ClientMode::OneShot) => (),
            _ => panic!("Mode not preserved during cloning"),
        }
    }
}
