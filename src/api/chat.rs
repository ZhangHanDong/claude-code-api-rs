use axum::{
    extract::State,
    response::IntoResponse,
    Json,
};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{info, error};
use uuid::Uuid;
use chrono::Utc;

use crate::{
    core::claude_manager::ClaudeManager,
    models::{
        error::{ApiError, ApiResult},
        openai::{ChatCompletionRequest, ChatCompletionResponse, ChatChoice, ChatMessage, Usage, MessageContent},
        claude::ClaudeCodeOutput,
    },
    utils::{streaming::create_sse_stream, parser::claude_to_openai_stream},
};
use once_cell::sync::Lazy;
use parking_lot::Mutex;

type TempFileEntry = (String, std::time::Instant);
type TempFileStore = Arc<Mutex<Vec<TempFileEntry>>>;

static TEMP_FILES: Lazy<TempFileStore> =
    Lazy::new(|| {
        let tracker = Arc::new(Mutex::new(Vec::new()));
        let tracker_clone = tracker.clone();
        tokio::spawn(async move {
            cleanup_temp_files(tracker_clone).await;
        });
        tracker
    });

async fn cleanup_temp_files(tracker: Arc<Mutex<Vec<(String, std::time::Instant)>>>) {
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(300)).await; // 每5分钟检查一次

        let mut files = tracker.lock();
        let now = std::time::Instant::now();

        files.retain(|(path, created)| {
            if now.duration_since(*created).as_secs() > 900 {
                if let Err(e) = std::fs::remove_file(path) {
                    error!("Failed to remove temp file {}: {}", path, e);
                } else {
                    info!("Cleaned up temp file: {}", path);
                }
                false
            } else {
                true
            }
        });
    }
}

#[derive(Clone)]
pub struct ChatState {
    pub claude_manager: Arc<ClaudeManager>,
    pub conversation_manager: Arc<crate::core::conversation::ConversationManager>,
    pub cache: Arc<crate::core::cache::ResponseCache>,
}

impl ChatState {
    pub fn new(
        claude_manager: Arc<ClaudeManager>,
        conversation_manager: Arc<crate::core::conversation::ConversationManager>,
        cache: Arc<crate::core::cache::ResponseCache>,
    ) -> Self {
        Self {
            claude_manager,
            conversation_manager,
            cache,
        }
    }
}

pub async fn chat_completions(
    State(state): State<ChatState>,
    Json(request): Json<ChatCompletionRequest>,
) -> ApiResult<axum::response::Response> {
    use crate::core::cache::ResponseCache;

    info!("Received chat completion request for model: {}", request.model);

    if request.messages.is_empty() {
        return Err(ApiError::BadRequest("Messages cannot be empty".to_string()));
    }

    let conversation_id = if let Some(ref conv_id) = request.conversation_id {
        conv_id.clone()
    } else {
        state.conversation_manager.create_conversation(Some(request.model.clone()))
    };

    let context_messages = state.conversation_manager
        .get_context_messages(&conversation_id, &request.messages);

    if !request.stream.unwrap_or(false) {
        let cache_key = ResponseCache::generate_key(&request.model, &context_messages);
        if let Some(cached_response) = state.cache.get(&cache_key) {
            info!("Returning cached response");
            return Ok(axum::Json(cached_response).into_response());
        }
    }

    let formatted_message = format_messages_for_claude(&context_messages).await?;

    let (session_id, rx) = state.claude_manager
        .create_session_with_message(None, None, Some(request.model.clone()), &formatted_message)
        .await
        .map_err(|e| ApiError::ClaudeProcess(e.to_string()))?;

    if request.stream.unwrap_or(false) {
        Ok(handle_streaming_response(request.model, rx).await?.into_response())
    } else {
        let cache_key = ResponseCache::generate_key(&request.model, &context_messages);
        let response = handle_non_streaming_response(request.model.clone(), rx, session_id, state.claude_manager.clone()).await?;

        for msg in &request.messages {
            state.conversation_manager.add_message(&conversation_id, msg.clone())
                .map_err(|e| ApiError::Internal(e.to_string()))?;
        }

        if let Some(choice) = response.0.choices.first() {
            state.conversation_manager.add_message(&conversation_id, choice.message.clone())
                .map_err(|e| ApiError::Internal(e.to_string()))?;
        }

        let mut response_with_conv_id = response.0.clone();
        response_with_conv_id.conversation_id = Some(conversation_id.clone());

        state.cache.put(cache_key.clone(), response_with_conv_id.clone());

        Ok(Json(response_with_conv_id).into_response())
    }
}

async fn format_messages_for_claude(messages: &[ChatMessage]) -> ApiResult<String> {
    let mut conversation = String::new();
    let mut all_image_paths = Vec::new();

    for (i, message) in messages.iter().enumerate() {
        let (mut content, msg_images) = extract_content_and_images(message).await?;

        if !msg_images.is_empty() {
            content.push_str("\n\n");
            for path in &msg_images {
                content.push_str(&format!("Image: {path}\n"));
            }
            all_image_paths.extend(msg_images);
        }

        if i == messages.len() - 1 {
            conversation.push_str(&content);
        } else {
            match message.role.as_str() {
                "user" => conversation.push_str(&format!("User: {content}\n")),
                "assistant" => conversation.push_str(&format!("Assistant: {content}\n")),
                "system" => conversation.push_str(&format!("System: {content}\n")),
                _ => {}
            }
        }
    }

    Ok(conversation)
}

async fn extract_content_and_images(message: &ChatMessage) -> ApiResult<(String, Vec<String>)> {
    let mut text_parts = Vec::new();
    let mut image_paths = Vec::new();

    match &message.content {
        MessageContent::Text(text) => {
            text_parts.push(text.clone());
        }
        MessageContent::Array(parts) => {
            for part in parts {
                match part {
                    crate::models::openai::ContentPart::Text { text } => {
                        text_parts.push(text.clone());
                    }
                    crate::models::openai::ContentPart::ImageUrl { image_url } => {
                        let path = process_image_url(&image_url.url).await?;
                        image_paths.push(path);
                    }
                }
            }
        }
    }

    Ok((text_parts.join(" "), image_paths))
}

async fn process_image_url(url: &str) -> ApiResult<String> {
    use std::io::Write;
    use base64::{Engine as _, engine::general_purpose};

    if url.starts_with("data:image/") {
        let parts: Vec<&str> = url.split(',').collect();
        if parts.len() != 2 {
            return Err(ApiError::BadRequest("Invalid data URL format".to_string()));
        }

        let base64_data = parts[1];
        let image_data = general_purpose::STANDARD
            .decode(base64_data)
            .map_err(|e| ApiError::BadRequest(format!("Invalid base64 data: {e}")))?;

        let temp_dir = std::env::temp_dir();
        let file_name = format!("claude_image_{}.png", Uuid::new_v4());
        let file_path = temp_dir.join(&file_name);

        let mut file = std::fs::File::create(&file_path)
            .map_err(|e| ApiError::Internal(format!("Failed to create temp file: {e}")))?;

        file.write_all(&image_data)
            .map_err(|e| ApiError::Internal(format!("Failed to write image data: {e}")))?;

        let path_string = file_path.to_string_lossy().to_string();

        TEMP_FILES.lock().push((path_string.clone(), std::time::Instant::now()));

        Ok(path_string)
    } else if url.starts_with("http://") || url.starts_with("https://") {
        download_image(url).await
    } else {
        Ok(url.to_string())
    }
}

async fn download_image(url: &str) -> ApiResult<String> {
    use reqwest;
    use std::io::Write;

    let response = reqwest::get(url)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to download image: {e}")))?;

    if !response.status().is_success() {
        return Err(ApiError::BadRequest(format!("Failed to download image: HTTP {}", response.status())));
    }

    let bytes = response.bytes()
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to read image data: {e}")))?;

    let temp_dir = std::env::temp_dir();
    let file_name = format!("claude_image_{}.png", Uuid::new_v4());
    let file_path = temp_dir.join(&file_name);

    let mut file = std::fs::File::create(&file_path)
        .map_err(|e| ApiError::Internal(format!("Failed to create temp file: {e}")))?;

    file.write_all(&bytes)
        .map_err(|e| ApiError::Internal(format!("Failed to write image data: {e}")))?;

    let path_string = file_path.to_string_lossy().to_string();

    TEMP_FILES.lock().push((path_string.clone(), std::time::Instant::now()));

    Ok(path_string)
}

async fn handle_streaming_response(
    model: String,
    mut rx: mpsc::Receiver<ClaudeCodeOutput>,
) -> ApiResult<impl IntoResponse> {
    let stream = async_stream::stream! {
        while let Some(output) = rx.recv().await {
            if let Some(openai_response) = claude_to_openai_stream(output, &model) {
                yield openai_response;
            }
        }
    };

    Ok(create_sse_stream(stream))
}

async fn handle_non_streaming_response(
    model: String,
    mut rx: mpsc::Receiver<ClaudeCodeOutput>,
    session_id: String,
    claude_manager: Arc<ClaudeManager>,
) -> ApiResult<Json<ChatCompletionResponse>> {
    use tokio::time::{timeout, Duration};

    let mut full_content = String::new();
    let mut token_count = 0;

    info!("Waiting for Claude response...");

    let timeout_duration = Duration::from_secs(30);
    let start = std::time::Instant::now();

    loop {
        match timeout(Duration::from_secs(5), rx.recv()).await {
            Ok(Some(output)) => {
                info!("Received output from Claude");
                if let Some(response) = claude_to_openai_stream(output, &model)
                    && let Some(content) = response.choices.first()
                        .and_then(|c| c.delta.content.as_ref()) {
                        full_content.push_str(content);
                        token_count += content.split_whitespace().count() as i32;
                    }
            }
            Ok(None) => {
                info!("Claude stream ended, total content length: {}", full_content.len());
                break;
            }
            Err(_) => {
                if start.elapsed() > timeout_duration {
                    error!("Timeout waiting for Claude response");
                    return Err(ApiError::ClaudeProcess("Timeout waiting for response".to_string()));
                }
                info!("No data received in 5s, but still waiting... (elapsed: {:?})", start.elapsed());
            }
        }
    }

    let _ = claude_manager.close_session(&session_id).await;

    let response = ChatCompletionResponse {
        id: Uuid::new_v4().to_string(),
        object: "chat.completion".to_string(),
        created: Utc::now().timestamp(),
        model,
        choices: vec![ChatChoice {
            index: 0,
            message: ChatMessage {
                role: "assistant".to_string(),
                content: MessageContent::Text(full_content),
                name: None,
            },
            finish_reason: Some("stop".to_string()),
        }],
        usage: Usage {
            prompt_tokens: 0,
            completion_tokens: token_count,
            total_tokens: token_count,
        },
        conversation_id: None,
    };

    Ok(Json(response))
}
