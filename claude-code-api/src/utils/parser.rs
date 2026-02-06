use crate::models::{
    claude::{ClaudeCodeOutput, ClaudeStreamEvent, ContentDelta},
    openai::{ChatCompletionStreamResponse, DeltaMessage, StreamChoice},
};
use chrono::Utc;
use uuid::Uuid;

pub fn claude_to_openai_stream(
    claude_output: ClaudeCodeOutput,
    model: &str,
) -> Option<ChatCompletionStreamResponse> {
    match claude_output.r#type.as_str() {
        "assistant" => {
            // 处理助手消息
            if let Some(message) = claude_output.data.get("message")
                && let Some(content_array) = message.get("content").and_then(|c| c.as_array())
            {
                for content in content_array {
                    if let Some(text) = content.get("text").and_then(|t| t.as_str()) {
                        return Some(ChatCompletionStreamResponse {
                            id: Uuid::new_v4().to_string(),
                            object: "chat.completion.chunk".to_string(),
                            created: Utc::now().timestamp(),
                            model: model.to_string(),
                            choices: vec![StreamChoice {
                                index: 0,
                                delta: DeltaMessage {
                                    role: Some("assistant".to_string()),
                                    content: Some(text.to_string()),
                                },
                                finish_reason: None,
                            }],
                        });
                    }
                }
            }
        },
        "result" => {
            // 会话结束
            return Some(ChatCompletionStreamResponse {
                id: Uuid::new_v4().to_string(),
                object: "chat.completion.chunk".to_string(),
                created: Utc::now().timestamp(),
                model: model.to_string(),
                choices: vec![StreamChoice {
                    index: 0,
                    delta: DeltaMessage::default(),
                    finish_reason: Some("stop".to_string()),
                }],
            });
        },
        _ => {},
    }

    None
}

#[allow(dead_code)]
fn convert_claude_event_to_openai(
    event: ClaudeStreamEvent,
    model: &str,
) -> Option<ChatCompletionStreamResponse> {
    match event {
        ClaudeStreamEvent::MessageStart { .. } => Some(ChatCompletionStreamResponse {
            id: Uuid::new_v4().to_string(),
            object: "chat.completion.chunk".to_string(),
            created: Utc::now().timestamp(),
            model: model.to_string(),
            choices: vec![StreamChoice {
                index: 0,
                delta: DeltaMessage {
                    role: Some("assistant".to_string()),
                    content: None,
                },
                finish_reason: None,
            }],
        }),
        ClaudeStreamEvent::ContentBlockDelta { delta, .. } => match delta {
            ContentDelta::TextDelta { text } => Some(ChatCompletionStreamResponse {
                id: Uuid::new_v4().to_string(),
                object: "chat.completion.chunk".to_string(),
                created: Utc::now().timestamp(),
                model: model.to_string(),
                choices: vec![StreamChoice {
                    index: 0,
                    delta: DeltaMessage {
                        role: None,
                        content: Some(text),
                    },
                    finish_reason: None,
                }],
            }),
        },
        ClaudeStreamEvent::MessageStop => Some(ChatCompletionStreamResponse {
            id: Uuid::new_v4().to_string(),
            object: "chat.completion.chunk".to_string(),
            created: Utc::now().timestamp(),
            model: model.to_string(),
            choices: vec![StreamChoice {
                index: 0,
                delta: DeltaMessage::default(),
                finish_reason: Some("stop".to_string()),
            }],
        }),
        _ => None,
    }
}
