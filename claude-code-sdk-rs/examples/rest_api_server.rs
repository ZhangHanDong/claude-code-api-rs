//! REST API server for testing with curl

use axum::{
    Router,
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, post},
};
use nexus_claude::{
    ClaudeCodeOptions, ClientMode, ContentBlock, Message, OptimizedClient, PermissionMode,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;
use tracing::{Level, info};

/// Request structure for queries
#[derive(Debug, Deserialize)]
struct QueryRequest {
    prompt: String,
    #[allow(dead_code)]
    mode: Option<String>,
}

/// Response structure
#[derive(Debug, Serialize)]
struct QueryResponse {
    success: bool,
    message: Option<String>,
    error: Option<String>,
    duration_ms: u64,
}

/// Batch request structure
#[derive(Debug, Deserialize)]
struct BatchRequest {
    prompts: Vec<String>,
    max_concurrent: Option<usize>,
}

/// Batch response structure
#[derive(Debug, Serialize)]
struct BatchResponse {
    success: bool,
    results: Vec<QueryResponse>,
    total_duration_ms: u64,
}

/// Health check response
#[derive(Debug, Serialize)]
struct HealthResponse {
    status: String,
    mode: String,
    mock: bool,
}

/// App state
struct AppState {
    mock_mode: bool,
    metrics: Arc<RwLock<nexus_claude::PerformanceMetrics>>,
}

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    // Check if we should run in mock mode
    let mock_mode = std::env::var("MOCK_MODE")
        .unwrap_or_else(|_| "false".to_string())
        .parse::<bool>()
        .unwrap_or(false);

    info!("Starting REST API server");
    info!(
        "Mode: {}",
        if mock_mode {
            "MOCK (for testing without claude-code)"
        } else {
            "REAL (using claude-code CLI)"
        }
    );

    // Create app state
    let state = Arc::new(AppState {
        mock_mode,
        metrics: Arc::new(RwLock::new(nexus_claude::PerformanceMetrics::default())),
    });

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

    axum::Server::bind(&addr.parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

/// Health check endpoint
async fn health_check(State(state): State<Arc<AppState>>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        mode: if state.mock_mode { "mock" } else { "real" }.to_string(),
        mock: state.mock_mode,
    })
}

/// Query endpoint
async fn query_handler(
    State(state): State<Arc<AppState>>,
    Json(request): Json<QueryRequest>,
) -> Result<Json<QueryResponse>, StatusCode> {
    let start = std::time::Instant::now();

    if state.mock_mode {
        // Mock response
        let response = generate_mock_response(&request.prompt);
        let duration_ms = start.elapsed().as_millis() as u64;

        // Update metrics
        state.metrics.write().await.record_success(duration_ms);

        Ok(Json(QueryResponse {
            success: true,
            message: Some(response),
            error: None,
            duration_ms,
        }))
    } else {
        // Real Claude API call
        match create_real_client().await {
            Ok(client) => match client.query(request.prompt.clone()).await {
                Ok(messages) => {
                    let response = extract_response_text(messages);
                    let duration_ms = start.elapsed().as_millis() as u64;
                    state.metrics.write().await.record_success(duration_ms);

                    Ok(Json(QueryResponse {
                        success: true,
                        message: Some(response),
                        error: None,
                        duration_ms,
                    }))
                },
                Err(e) => {
                    state.metrics.write().await.record_failure();
                    Ok(Json(QueryResponse {
                        success: false,
                        message: None,
                        error: Some(e.to_string()),
                        duration_ms: start.elapsed().as_millis() as u64,
                    }))
                },
            },
            Err(e) => {
                state.metrics.write().await.record_failure();
                Ok(Json(QueryResponse {
                    success: false,
                    message: None,
                    error: Some(format!("Failed to create client: {e}")),
                    duration_ms: start.elapsed().as_millis() as u64,
                }))
            },
        }
    }
}

/// Batch endpoint
async fn batch_handler(
    State(state): State<Arc<AppState>>,
    Json(request): Json<BatchRequest>,
) -> Result<Json<BatchResponse>, StatusCode> {
    let start = std::time::Instant::now();
    let max_concurrent = request.max_concurrent.unwrap_or(5);

    let mut results = Vec::new();

    if state.mock_mode {
        // Mock batch processing
        for prompt in request.prompts {
            let response = generate_mock_response(&prompt);
            results.push(QueryResponse {
                success: true,
                message: Some(response),
                error: None,
                duration_ms: 10,
            });
        }
    } else {
        // Real batch processing
        match create_batch_client(max_concurrent).await {
            Ok(client) => {
                let batch_results = client
                    .process_batch(request.prompts.clone())
                    .await
                    .unwrap_or_default();

                for result in batch_results.into_iter() {
                    match result {
                        Ok(messages) => {
                            let response = extract_response_text(messages);
                            results.push(QueryResponse {
                                success: true,
                                message: Some(response),
                                error: None,
                                duration_ms: 100,
                            });
                        },
                        Err(e) => {
                            results.push(QueryResponse {
                                success: false,
                                message: None,
                                error: Some(e.to_string()),
                                duration_ms: 0,
                            });
                        },
                    }
                }
            },
            Err(_e) => {
                return Ok(Json(BatchResponse {
                    success: false,
                    results: vec![],
                    total_duration_ms: start.elapsed().as_millis() as u64,
                }));
            },
        }
    }

    Ok(Json(BatchResponse {
        success: true,
        results,
        total_duration_ms: start.elapsed().as_millis() as u64,
    }))
}

/// Metrics endpoint
async fn metrics_handler(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let metrics = state.metrics.read().await;

    Json(serde_json::json!({
        "total_requests": metrics.total_requests,
        "successful_requests": metrics.successful_requests,
        "failed_requests": metrics.failed_requests,
        "success_rate": metrics.success_rate(),
        "average_latency_ms": metrics.average_latency_ms(),
        "min_latency_ms": metrics.min_latency_ms,
        "max_latency_ms": metrics.max_latency_ms,
    }))
}

/// Generate mock response
fn generate_mock_response(prompt: &str) -> String {
    match prompt {
        "What is 2 + 2?" => "4".to_string(),
        "What is the capital of France?" => "Paris".to_string(),
        prompt if prompt.contains("squared") => {
            if let Some(num) = prompt
                .split_whitespace()
                .find_map(|w| w.parse::<i32>().ok())
            {
                format!("{} squared is {}", num, num * num)
            } else {
                "Please provide a number to square.".to_string()
            }
        },
        _ => format!("Mock response to: {prompt}"),
    }
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

/// Create real client
async fn create_real_client() -> nexus_claude::Result<OptimizedClient> {
    let options = ClaudeCodeOptions::builder()
        .permission_mode(PermissionMode::AcceptEdits)
        .build();

    OptimizedClient::new(options, ClientMode::OneShot)
}

/// Create batch client
async fn create_batch_client(max_concurrent: usize) -> nexus_claude::Result<OptimizedClient> {
    let options = ClaudeCodeOptions::builder()
        .permission_mode(PermissionMode::AcceptEdits)
        .build();

    OptimizedClient::new(options, ClientMode::Batch { max_concurrent })
}
