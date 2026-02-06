//! Optimized REST API server using connection pooling

use axum::{
    Router,
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, post},
};
use nexus_claude::{
    ClaudeCodeOptions, ClientMode, ContentBlock, Message, OptimizedClient, PerformanceMetrics,
    PermissionMode,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;
use tracing::{Level, info, warn};

#[derive(Debug, Deserialize)]
struct QueryRequest {
    prompt: String,
}

#[derive(Debug, Serialize)]
struct QueryResponse {
    success: bool,
    message: Option<String>,
    error: Option<String>,
    duration_ms: u64,
}

#[derive(Debug, Deserialize)]
struct BatchRequest {
    prompts: Vec<String>,
    max_concurrent: Option<usize>,
}

#[derive(Debug, Serialize)]
struct BatchResponse {
    success: bool,
    results: Vec<QueryResponse>,
    total_duration_ms: u64,
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: String,
    pool_info: String,
    uptime_seconds: u64,
}

/// Shared application state
struct AppState {
    /// Optimized client for single queries (with connection pooling)
    query_client: Arc<OptimizedClient>,
    /// Optimized client for batch processing
    batch_client: Arc<OptimizedClient>,
    /// Performance metrics
    metrics: Arc<RwLock<PerformanceMetrics>>,
    /// Server start time
    start_time: Instant,
}

impl AppState {
    async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let options = ClaudeCodeOptions::builder()
            .permission_mode(PermissionMode::AcceptEdits)
            .build();

        // Create optimized clients
        let query_client = Arc::new(OptimizedClient::new(options.clone(), ClientMode::OneShot)?);

        let batch_client = Arc::new(OptimizedClient::new(
            options,
            ClientMode::Batch { max_concurrent: 5 },
        )?);

        // Pre-warm the connection pool
        info!("Pre-warming connection pool...");
        let warmup_start = Instant::now();

        // Send a simple query to establish initial connections
        match query_client.query("Hi".to_string()).await {
            Ok(_) => info!("Connection pool warmed up in {:?}", warmup_start.elapsed()),
            Err(e) => warn!("Failed to warm up connection pool: {}", e),
        }

        Ok(Self {
            query_client,
            batch_client,
            metrics: Arc::new(RwLock::new(PerformanceMetrics::default())),
            start_time: Instant::now(),
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    info!("Starting Optimized REST API Server");
    info!("Features: Connection pooling, Pre-warming, Concurrent batch processing");

    // Create application state
    let state = Arc::new(AppState::new().await?);

    // Build router
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/query", post(query_handler))
        .route("/batch", post(batch_handler))
        .route("/metrics", get(metrics_handler))
        .layer(CorsLayer::permissive())
        .with_state(state);

    // Start server
    let addr = "127.0.0.1:3000";
    info!("Server listening on http://{}", addr);
    info!("Connection pool is ready for fast responses!");

    axum::Server::bind(&addr.parse()?)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}

/// Health check endpoint
async fn health_check(State(state): State<Arc<AppState>>) -> Json<HealthResponse> {
    let uptime = state.start_time.elapsed().as_secs();

    Json(HealthResponse {
        status: "healthy".to_string(),
        pool_info: "Connection pool active with pre-warmed connections".to_string(),
        uptime_seconds: uptime,
    })
}

/// Query handler with connection pooling
async fn query_handler(
    State(state): State<Arc<AppState>>,
    Json(request): Json<QueryRequest>,
) -> Result<Json<QueryResponse>, StatusCode> {
    let start = Instant::now();

    // Use the optimized client with connection pooling
    match state.query_client.query(request.prompt.clone()).await {
        Ok(messages) => {
            let response_text = extract_response_text(messages);
            let duration_ms = start.elapsed().as_millis() as u64;

            // Update metrics
            state.metrics.write().await.record_success(duration_ms);

            info!(
                "Query completed in {}ms (with connection pooling)",
                duration_ms
            );

            Ok(Json(QueryResponse {
                success: true,
                message: Some(response_text),
                error: None,
                duration_ms,
            }))
        },
        Err(e) => {
            let duration_ms = start.elapsed().as_millis() as u64;
            state.metrics.write().await.record_failure();

            warn!("Query failed: {}", e);

            Ok(Json(QueryResponse {
                success: false,
                message: None,
                error: Some(e.to_string()),
                duration_ms,
            }))
        },
    }
}

/// Batch handler with concurrent processing
async fn batch_handler(
    State(state): State<Arc<AppState>>,
    Json(request): Json<BatchRequest>,
) -> Result<Json<BatchResponse>, StatusCode> {
    let start = Instant::now();
    let max_concurrent = request.max_concurrent.unwrap_or(5).min(10); // Cap at 10

    info!(
        "Processing batch of {} queries with max_concurrent={}",
        request.prompts.len(),
        max_concurrent
    );

    // Create a batch client with the requested concurrency
    let batch_client = if max_concurrent != 5 {
        // Create custom batch client if different concurrency is requested
        let options = ClaudeCodeOptions::builder()
            .permission_mode(PermissionMode::AcceptEdits)
            .build();

        match OptimizedClient::new(options, ClientMode::Batch { max_concurrent }) {
            Ok(client) => Arc::new(client),
            Err(_) => state.batch_client.clone(), // Fallback to default
        }
    } else {
        state.batch_client.clone()
    };

    // Process batch with optimized client
    match batch_client.process_batch(request.prompts.clone()).await {
        Ok(results) => {
            let mut responses = Vec::new();

            for (_prompt, result) in request.prompts.iter().zip(results.iter()) {
                match result {
                    Ok(messages) => {
                        let response_text = extract_response_text(messages.clone());
                        responses.push(QueryResponse {
                            success: true,
                            message: Some(response_text),
                            error: None,
                            duration_ms: 0, // Individual timing not tracked in batch
                        });
                    },
                    Err(e) => {
                        responses.push(QueryResponse {
                            success: false,
                            message: None,
                            error: Some(e.to_string()),
                            duration_ms: 0,
                        });
                    },
                }
            }

            let total_duration_ms = start.elapsed().as_millis() as u64;
            let successful = responses.iter().filter(|r| r.success).count();

            info!(
                "Batch completed: {}/{} successful in {}ms",
                successful,
                responses.len(),
                total_duration_ms
            );

            Ok(Json(BatchResponse {
                success: true,
                results: responses,
                total_duration_ms,
            }))
        },
        Err(e) => {
            warn!("Batch processing failed: {}", e);
            Ok(Json(BatchResponse {
                success: false,
                results: vec![],
                total_duration_ms: start.elapsed().as_millis() as u64,
            }))
        },
    }
}

/// Metrics endpoint
async fn metrics_handler(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let metrics = state.metrics.read().await;
    let uptime = state.start_time.elapsed().as_secs();

    Json(serde_json::json!({
        "total_requests": metrics.total_requests,
        "successful_requests": metrics.successful_requests,
        "failed_requests": metrics.failed_requests,
        "success_rate": metrics.success_rate(),
        "average_latency_ms": metrics.average_latency_ms(),
        "min_latency_ms": metrics.min_latency_ms,
        "max_latency_ms": metrics.max_latency_ms,
        "uptime_seconds": uptime,
        "optimization_features": {
            "connection_pooling": true,
            "pre_warming": true,
            "concurrent_batch": true,
            "retry_logic": true,
        }
    }))
}

/// Extract text from messages
fn extract_response_text(messages: Vec<Message>) -> String {
    messages
        .into_iter()
        .filter_map(|msg| match msg {
            Message::Assistant { message } => {
                let texts: Vec<String> = message
                    .content
                    .into_iter()
                    .filter_map(|content| match content {
                        ContentBlock::Text(text) => Some(text.text),
                        _ => None,
                    })
                    .collect();
                Some(texts.join("\n"))
            },
            _ => None,
        })
        .collect::<Vec<String>>()
        .join("\n")
}
