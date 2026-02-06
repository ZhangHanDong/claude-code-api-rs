//! Example showing how to integrate the optimized client with existing API patterns

use nexus_claude::{
    ClaudeCodeOptions, ClientMode, OptimizedClient, PerformanceMetrics, PermissionMode, Result,
};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tracing::{Level, info};

/// API wrapper that provides a high-level interface using the optimized client
struct ClaudeAPI {
    client: OptimizedClient,
    metrics: Arc<RwLock<PerformanceMetrics>>,
}

impl ClaudeAPI {
    /// Create new API instance
    pub fn new(options: ClaudeCodeOptions, mode: ClientMode) -> Result<Self> {
        let client = OptimizedClient::new(options, mode)?;
        let metrics = Arc::new(RwLock::new(PerformanceMetrics::default()));

        Ok(Self { client, metrics })
    }

    /// Execute a query with metrics tracking
    pub async fn query_with_metrics(&self, prompt: String) -> Result<String> {
        let start = Instant::now();

        match self.client.query(prompt).await {
            Ok(messages) => {
                let latency_ms = start.elapsed().as_millis() as u64;
                self.metrics.write().await.record_success(latency_ms);

                // Extract text content from assistant messages
                let mut response = String::new();
                for msg in messages {
                    if let nexus_claude::Message::Assistant {
                        message: assistant_msg,
                    } = msg
                    {
                        for content in &assistant_msg.content {
                            if let nexus_claude::ContentBlock::Text(text) = content {
                                response.push_str(&text.text);
                                response.push('\n');
                            }
                        }
                    }
                }

                Ok(response.trim().to_string())
            },
            Err(e) => {
                self.metrics.write().await.record_failure();
                Err(e)
            },
        }
    }

    /// Get current metrics
    pub async fn get_metrics(&self) -> PerformanceMetrics {
        self.metrics.read().await.clone()
    }

    /// Health check endpoint
    pub async fn health_check(&self) -> Result<bool> {
        match self.client.query("Say 'OK'".to_string()).await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}

/// Example REST API handler (conceptual)
async fn handle_completion_request(api: Arc<ClaudeAPI>, prompt: String) -> Result<String> {
    api.query_with_metrics(prompt).await
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    // Configure Claude options
    let options = ClaudeCodeOptions::builder()
        .permission_mode(PermissionMode::AcceptEdits)
        .model("claude-3.5-sonnet")
        .build();

    // Create API instance with batch mode for handling multiple requests
    let api = Arc::new(ClaudeAPI::new(
        options,
        ClientMode::Batch { max_concurrent: 5 },
    )?);

    info!("=== API Integration Example ===");

    // Simulate API requests
    let requests = [
        "What is the weather like today?",
        "Explain quantum computing in simple terms",
        "Write a Python function to calculate fibonacci numbers",
        "What are the benefits of Rust programming?",
        "How does async/await work?",
    ];

    // Process requests concurrently
    let mut handles = Vec::new();
    for (i, request) in requests.iter().enumerate() {
        let api_clone = api.clone();
        let prompt = request.to_string();

        let handle = tokio::spawn(async move {
            info!("Processing request {}", i + 1);
            let result = handle_completion_request(api_clone, prompt).await;
            (i, result)
        });

        handles.push(handle);
    }

    // Collect results
    for handle in handles {
        match handle.await {
            Ok((i, result)) => match result {
                Ok(response) => {
                    info!("Request {} completed:", i + 1);
                    info!(
                        "  Response preview: {}...",
                        response.chars().take(100).collect::<String>()
                    );
                },
                Err(e) => {
                    info!("Request {} failed: {}", i + 1, e);
                },
            },
            Err(e) => {
                info!("Task failed: {}", e);
            },
        }
    }

    // Display metrics
    let metrics = api.get_metrics().await;
    info!("\n=== Performance Metrics ===");
    info!("Total requests: {}", metrics.total_requests);
    info!("Successful: {}", metrics.successful_requests);
    info!("Failed: {}", metrics.failed_requests);
    info!("Success rate: {:.2}%", metrics.success_rate() * 100.0);
    info!("Average latency: {:.2}ms", metrics.average_latency_ms());
    info!("Min latency: {}ms", metrics.min_latency_ms);
    info!("Max latency: {}ms", metrics.max_latency_ms);

    // Health check
    info!("\n=== Health Check ===");
    match api.health_check().await {
        Ok(healthy) => info!(
            "Service health: {}",
            if healthy { "OK" } else { "UNHEALTHY" }
        ),
        Err(e) => info!("Health check failed: {}", e),
    }

    Ok(())
}
