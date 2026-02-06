//! OpenAI API compatible server for Claude Code

use axum::{
    Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::Json,
    routing::{get, post},
};
use nexus_claude::{
    ClaudeCodeOptions, ClientMode, ContentBlock, Message, OptimizedClient, PermissionMode,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;
use tracing::{Level, info, warn};
use uuid::Uuid;

// OpenAI API compatible structures

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
    #[serde(default = "default_temperature")]
    temperature: f32,
    #[serde(default = "default_max_tokens")]
    max_tokens: Option<i32>,
    #[serde(default)]
    stream: bool,
    #[serde(default)]
    n: Option<i32>,
    #[serde(default)]
    stop: Option<Vec<String>>,
    #[serde(default)]
    presence_penalty: Option<f32>,
    #[serde(default)]
    frequency_penalty: Option<f32>,
    #[serde(default)]
    user: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct ChatMessage {
    role: String,
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
}

#[derive(Debug, Serialize)]
struct ChatCompletionResponse {
    id: String,
    object: String,
    created: u64,
    model: String,
    choices: Vec<Choice>,
    usage: Usage,
}

#[derive(Debug, Serialize)]
struct Choice {
    index: i32,
    message: ChatMessage,
    finish_reason: String,
}

#[derive(Debug, Serialize)]
struct Usage {
    prompt_tokens: i32,
    completion_tokens: i32,
    total_tokens: i32,
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: ErrorDetail,
}

#[derive(Debug, Serialize)]
struct ErrorDetail {
    message: String,
    #[serde(rename = "type")]
    error_type: String,
    param: Option<String>,
    code: Option<String>,
}

#[derive(Debug, Serialize)]
struct ModelObject {
    id: String,
    object: String,
    created: u64,
    owned_by: String,
}

#[derive(Debug, Serialize)]
struct ModelsResponse {
    object: String,
    data: Vec<ModelObject>,
}

fn default_temperature() -> f32 {
    1.0
}

fn default_max_tokens() -> Option<i32> {
    Some(4096)
}

struct AppState {
    client: Arc<OptimizedClient>,
    #[allow(dead_code)]
    interactive_sessions: Arc<RwLock<std::collections::HashMap<String, Arc<OptimizedClient>>>>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    info!("Starting OpenAI API compatible server for Claude Code");

    // Create optimized client with connection pooling
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
        .route("/v1/completions", post(completions))
        .route("/v1/models", get(list_models))
        .route("/v1/models/:model", get(get_model))
        .route("/health", get(health_check))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = "127.0.0.1:8080";
    info!("OpenAI-compatible API listening on http://{}", addr);
    info!(
        "Example: curl http://localhost:8080/v1/chat/completions -H 'Content-Type: application/json' -d '{{\"model\": \"gpt-3.5-turbo\", \"messages\": [{{\"role\": \"user\", \"content\": \"Hello\"}}]}}'"
    );

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
        "Received chat completion request for model: {}",
        request.model
    );

    // Extract the last user message (Claude Code works with single prompts)
    let prompt = request
        .messages
        .iter()
        .rev()
        .find(|m| m.role == "user")
        .map(|m| m.content.clone())
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: ErrorDetail {
                        message: "No user message found".to_string(),
                        error_type: "invalid_request_error".to_string(),
                        param: Some("messages".to_string()),
                        code: None,
                    },
                }),
            )
        })?;

    // If there's a system message, prepend it to the prompt
    let system_prompt = request
        .messages
        .iter()
        .find(|m| m.role == "system")
        .map(|m| m.content.clone());

    let final_prompt = if let Some(system) = system_prompt {
        format!("{system}\n\n{prompt}")
    } else {
        prompt
    };

    // Use the optimized client
    let start = std::time::Instant::now();
    let prompt_for_tokens = final_prompt.clone();
    match state.client.query(final_prompt).await {
        Ok(messages) => {
            let response_text = extract_response_text(messages);
            let elapsed = start.elapsed();

            info!("Generated response in {:?}", elapsed);

            // Build OpenAI-compatible response
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
                    prompt_tokens: estimate_tokens(&prompt_for_tokens),
                    completion_tokens: estimate_tokens(&response_text),
                    total_tokens: estimate_tokens(&prompt_for_tokens)
                        + estimate_tokens(&response_text),
                },
            };

            Ok(Json(response))
        },
        Err(e) => {
            warn!("Claude Code error: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: ErrorDetail {
                        message: format!("Claude Code error: {e}"),
                        error_type: "server_error".to_string(),
                        param: None,
                        code: None,
                    },
                }),
            ))
        },
    }
}

async fn completions(
    State(state): State<Arc<AppState>>,
    Json(request): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    // Extract prompt from completion request
    let prompt = request["prompt"].as_str().ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: ErrorDetail {
                    message: "Missing prompt".to_string(),
                    error_type: "invalid_request_error".to_string(),
                    param: Some("prompt".to_string()),
                    code: None,
                },
            }),
        )
    })?;

    match state.client.query(prompt.to_string()).await {
        Ok(messages) => {
            let response_text = extract_response_text(messages);

            let response = json!({
                "id": format!("cmpl-{}", Uuid::new_v4()),
                "object": "text_completion",
                "created": SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                "model": request["model"].as_str().unwrap_or("claude-code"),
                "choices": [{
                    "text": response_text,
                    "index": 0,
                    "logprobs": null,
                    "finish_reason": "stop"
                }],
                "usage": {
                    "prompt_tokens": estimate_tokens(prompt),
                    "completion_tokens": estimate_tokens(&response_text),
                    "total_tokens": estimate_tokens(prompt) + estimate_tokens(&response_text)
                }
            });

            Ok(Json(response))
        },
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: ErrorDetail {
                    message: format!("Claude Code error: {e}"),
                    error_type: "server_error".to_string(),
                    param: None,
                    code: None,
                },
            }),
        )),
    }
}

async fn list_models() -> Json<ModelsResponse> {
    Json(ModelsResponse {
        object: "list".to_string(),
        data: vec![
            ModelObject {
                id: "gpt-3.5-turbo".to_string(),
                object: "model".to_string(),
                created: 1677610602,
                owned_by: "claude-code".to_string(),
            },
            ModelObject {
                id: "gpt-4".to_string(),
                object: "model".to_string(),
                created: 1687882410,
                owned_by: "claude-code".to_string(),
            },
            ModelObject {
                id: "claude-3-sonnet".to_string(),
                object: "model".to_string(),
                created: 1709856000,
                owned_by: "claude-code".to_string(),
            },
            ModelObject {
                id: "claude-3.5-sonnet".to_string(),
                object: "model".to_string(),
                created: 1718841600,
                owned_by: "claude-code".to_string(),
            },
        ],
    })
}

async fn get_model(
    axum::extract::Path(model): axum::extract::Path<String>,
) -> Result<Json<ModelObject>, (StatusCode, Json<ErrorResponse>)> {
    let models = [
        ("gpt-3.5-turbo", 1677610602),
        ("gpt-4", 1687882410),
        ("claude-3-sonnet", 1709856000),
        ("claude-3.5-sonnet", 1718841600),
    ];

    if let Some((_, created)) = models.iter().find(|(m, _)| *m == model) {
        Ok(Json(ModelObject {
            id: model,
            object: "model".to_string(),
            created: *created,
            owned_by: "claude-code".to_string(),
        }))
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: ErrorDetail {
                    message: format!("Model '{model}' not found"),
                    error_type: "invalid_request_error".to_string(),
                    param: Some("model".to_string()),
                    code: None,
                },
            }),
        ))
    }
}

async fn health_check() -> Json<serde_json::Value> {
    Json(json!({
        "status": "healthy",
        "service": "claude-code-openai-api",
        "version": "1.0.0"
    }))
}

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

fn estimate_tokens(text: &str) -> i32 {
    // Simple estimation: ~4 characters per token
    (text.len() / 4) as i32
}
