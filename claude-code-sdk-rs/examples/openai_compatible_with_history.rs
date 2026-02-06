//! OpenAI API compatible server with proper conversation history support

use axum::{
    Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::Json,
    routing::{get, post},
};
use nexus_claude::{ClaudeCodeOptions, ClientMode, OptimizedClient, PermissionMode};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;
use tracing::{Level, info, warn};
use uuid::Uuid;

#[derive(Clone)]
struct AppState {
    client: Arc<OptimizedClient>,
    #[allow(dead_code)]
    interactive_sessions: Arc<RwLock<std::collections::HashMap<String, Vec<ChatMessage>>>>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
    #[serde(default)]
    temperature: Option<f32>,
    #[serde(default)]
    max_tokens: Option<i32>,
    #[serde(default)]
    stream: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct ChatMessage {
    role: String,
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
}

#[derive(Serialize)]
struct ChatCompletionResponse {
    id: String,
    object: String,
    created: u64,
    model: String,
    choices: Vec<Choice>,
    usage: Usage,
}

#[derive(Serialize)]
struct Choice {
    index: i32,
    message: ChatMessage,
    finish_reason: String,
}

#[derive(Serialize)]
struct Usage {
    prompt_tokens: i32,
    completion_tokens: i32,
    total_tokens: i32,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: ErrorDetail,
}

#[derive(Serialize)]
struct ErrorDetail {
    message: String,
    #[serde(rename = "type")]
    error_type: String,
    param: Option<String>,
    code: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    info!("Starting OpenAI API compatible server WITH proper history support");

    let options = ClaudeCodeOptions::builder()
        .permission_mode(PermissionMode::AcceptEdits)
        .build();

    let client = Arc::new(OptimizedClient::new(options.clone(), ClientMode::OneShot)?);

    // Pre-warm connection
    info!("Pre-warming connection pool...");
    match client.query("Hi".to_string()).await {
        Ok(_) => info!("Connection pool ready"),
        Err(e) => warn!("Failed to warm up connection: {}", e),
    }

    let state = Arc::new(AppState {
        client,
        interactive_sessions: Arc::new(RwLock::new(std::collections::HashMap::new())),
    });

    let app = Router::new()
        .route("/v1/chat/completions", post(chat_completions))
        .route("/v1/models", get(list_models))
        .route("/health", get(health_check))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = "127.0.0.1:8080";
    info!("Server listening on http://{}", addr);
    info!("This version properly handles conversation history!");

    axum::Server::bind(&addr.parse()?)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}

async fn chat_completions(
    State(state): State<Arc<AppState>>,
    _headers: HeaderMap,
    Json(request): Json<ChatCompletionRequest>,
) -> Result<Json<ChatCompletionResponse>, (StatusCode, Json<ErrorResponse>)> {
    info!(
        "Received chat request with {} messages",
        request.messages.len()
    );

    // Build a complete conversation prompt
    let mut conversation = String::new();

    for message in &request.messages {
        match message.role.as_str() {
            "system" => {
                conversation.push_str(&format!("System: {}\n\n", message.content));
            },
            "user" => {
                conversation.push_str(&format!("Human: {}\n\n", message.content));
            },
            "assistant" => {
                conversation.push_str(&format!("Assistant: {}\n\n", message.content));
            },
            _ => {},
        }
    }

    // Add the final prompt for Claude to complete
    conversation.push_str("Assistant: ");

    info!(
        "Full conversation prompt ({} chars):\n{}",
        conversation.len(),
        if conversation.len() > 200 {
            format!("{}...", &conversation[..200])
        } else {
            conversation.clone()
        }
    );

    // Send to Claude
    let start = std::time::Instant::now();
    match state.client.query(conversation.clone()).await {
        Ok(messages) => {
            let response_text = extract_response_text(messages);
            let elapsed = start.elapsed();

            info!("Generated response in {:?}", elapsed);

            let response = ChatCompletionResponse {
                id: format!("chatcmpl-{}", Uuid::new_v4()),
                object: "chat.completion".to_string(),
                created: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                model: request.model,
                choices: vec![Choice {
                    index: 0,
                    message: ChatMessage {
                        role: "assistant".to_string(),
                        content: response_text.clone(),
                        name: None,
                    },
                    finish_reason: "stop".to_string(),
                }],
                usage: Usage {
                    prompt_tokens: estimate_tokens(&conversation),
                    completion_tokens: estimate_tokens(&response_text),
                    total_tokens: estimate_tokens(&conversation) + estimate_tokens(&response_text),
                },
            };

            Ok(Json(response))
        },
        Err(e) => {
            warn!("Error from Claude: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: ErrorDetail {
                        message: format!("Error processing request: {e}"),
                        error_type: "server_error".to_string(),
                        param: None,
                        code: None,
                    },
                }),
            ))
        },
    }
}

async fn list_models(_headers: HeaderMap) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "object": "list",
        "data": [
            {
                "id": "claude-3.5-sonnet",
                "object": "model",
                "created": 1677649963,
                "owned_by": "anthropic",
                "permission": [],
                "root": "claude-3.5-sonnet",
                "parent": null,
            },
            {
                "id": "gpt-3.5-turbo",
                "object": "model",
                "created": 1677649963,
                "owned_by": "anthropic-compatible",
                "permission": [],
                "root": "claude-3.5-sonnet",
                "parent": null,
            }
        ]
    }))
}

async fn health_check() -> &'static str {
    "OK"
}

fn extract_response_text(messages: Vec<nexus_claude::Message>) -> String {
    messages
        .into_iter()
        .filter_map(|msg| {
            if let nexus_claude::Message::Assistant { message } = msg {
                let text = message
                    .content
                    .into_iter()
                    .filter_map(|content| {
                        if let nexus_claude::ContentBlock::Text(text_block) = content {
                            Some(text_block.text)
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                Some(text)
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn estimate_tokens(text: &str) -> i32 {
    // Rough estimation: ~4 characters per token
    (text.len() / 4) as i32
}
