#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClaudeStreamEvent {
    MessageStart {
        message: ClaudeMessage,
    },
    ContentBlockStart {
        index: i32,
        content_block: ContentBlock,
    },
    ContentBlockDelta {
        index: i32,
        delta: ContentDelta,
    },
    ContentBlockStop {
        index: i32,
    },
    MessageDelta {
        delta: MessageDelta,
        usage: Usage,
    },
    MessageStop,
    Error {
        error: ClaudeError,
    },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ClaudeMessage {
    pub id: String,
    pub r#type: String,
    pub role: String,
    pub content: Vec<ContentBlock>,
    pub model: String,
    pub stop_reason: Option<String>,
    pub stop_sequence: Option<String>,
    pub usage: Usage,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum ContentDelta {
    #[serde(rename = "text_delta")]
    TextDelta { text: String },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MessageDelta {
    pub stop_reason: Option<String>,
    pub stop_sequence: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Usage {
    pub input_tokens: i32,
    pub output_tokens: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ClaudeError {
    pub r#type: String,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ClaudeCodeOutput {
    pub r#type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtype: Option<String>,
    #[serde(flatten)]
    pub data: Value,
}

#[derive(Debug, Clone)]
pub struct ClaudeModel {
    pub id: String,
    pub display_name: String,
    pub context_window: i32,
}

impl ClaudeModel {
    pub fn all() -> Vec<Self> {
        vec![
            Self {
                id: "claude-sonnet-4-20250514".to_string(),
                display_name: "Claude 4 Sonnet".to_string(),
                context_window: 200000,
            },
            Self {
                id: "claude-3-5-haiku-20241022".to_string(),
                display_name: "Claude 3.5 Haiku".to_string(),
                context_window: 200000,
            },
            Self {
                id: "claude-opus-4-20250514".to_string(),
                display_name: "Claude 4 Opus".to_string(),
                context_window: 200000,
            },
        ]
    }
}