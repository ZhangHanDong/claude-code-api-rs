//! Enhanced streaming handler with real chunking support

use crate::{
    models::{
        claude::ClaudeCodeOutput,
        openai::{ChatCompletionStreamResponse, DeltaMessage, StreamChoice},
    },
    utils::text_chunker::{ChunkConfig, chunk_text},
};
use chrono::Utc;
use futures::stream::{Stream, StreamExt};
use std::pin::Pin;
use tokio::sync::mpsc;
use uuid::Uuid;

/// Handle streaming response with text chunking for better UX
pub async fn handle_enhanced_streaming_response(
    model: String,
    mut rx: mpsc::Receiver<ClaudeCodeOutput>,
) -> Pin<Box<dyn Stream<Item = ChatCompletionStreamResponse> + Send>> {
    let stream = async_stream::stream! {
        let stream_id = Uuid::new_v4().to_string();

        // First, send the initial message with role
        yield ChatCompletionStreamResponse {
            id: stream_id.clone(),
            object: "chat.completion.chunk".to_string(),
            created: Utc::now().timestamp(),
            model: model.clone(),
            choices: vec![StreamChoice {
                index: 0,
                delta: DeltaMessage {
                    role: Some("assistant".to_string()),
                    content: None,
                },
                finish_reason: None,
            }],
        };

        while let Some(output) = rx.recv().await {
            match output.r#type.as_str() {
                "assistant" => {
                    // Extract the full text content
                    if let Some(message) = output.data.get("message")
                        && let Some(content_array) = message.get("content").and_then(|c| c.as_array()) {

                        for content in content_array {
                            if let Some(text) = content.get("text").and_then(|t| t.as_str()) {
                                // Chunk the text for streaming
                                let config = ChunkConfig {
                                    chunk_size: 15,  // Smaller chunks for better streaming effect
                                    chunk_delay_ms: 30,  // 30ms between chunks
                                    word_boundary: true,
                                };

                                let mut chunker = chunk_text(text.to_string(), Some(config));

                                while let Some(chunk) = chunker.next().await {
                                    yield ChatCompletionStreamResponse {
                                        id: stream_id.clone(),
                                        object: "chat.completion.chunk".to_string(),
                                        created: Utc::now().timestamp(),
                                        model: model.clone(),
                                        choices: vec![StreamChoice {
                                            index: 0,
                                            delta: DeltaMessage {
                                                role: None,
                                                content: Some(chunk),
                                            },
                                            finish_reason: None,
                                        }],
                                    };
                                }
                            }
                        }
                    }
                }
                "result" => {
                    // Send the final chunk with finish_reason
                    yield ChatCompletionStreamResponse {
                        id: stream_id.clone(),
                        object: "chat.completion.chunk".to_string(),
                        created: Utc::now().timestamp(),
                        model: model.clone(),
                        choices: vec![StreamChoice {
                            index: 0,
                            delta: DeltaMessage::default(),
                            finish_reason: Some("stop".to_string()),
                        }],
                    };
                }
                _ => {}
            }
        }
    };

    Box::pin(stream)
}

/// Configuration for streaming enhancement
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct StreamingConfig {
    /// Whether to enable text chunking
    pub enable_chunking: bool,
    /// Chunk configuration
    pub chunk_config: ChunkConfig,
}

impl Default for StreamingConfig {
    fn default() -> Self {
        Self {
            enable_chunking: true,
            chunk_config: ChunkConfig::default(),
        }
    }
}
