//! Message parsing utilities
//!
//! This module handles parsing of JSON messages from the Claude CLI into
//! strongly typed Message enums.

use crate::{
    errors::{Result, SdkError},
    types::{
        AssistantMessage, AssistantMessageError, ContentBlock, ContentValue, Message,
        RateLimitInfo, TextContent, ThinkingContent, ToolResultContent, ToolUseContent,
        UserMessage,
    },
};
use serde_json::Value;
use tracing::{debug, trace};

/// Parse a JSON value into a Message
pub fn parse_message(json: Value) -> Result<Option<Message>> {
    // Get message type
    let msg_type = json
        .get("type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| SdkError::parse_error("Missing 'type' field", json.to_string()))?;

    match msg_type {
        "user" => parse_user_message(json),
        "assistant" => parse_assistant_message(json),
        "system" => parse_system_message(json),
        "result" => parse_result_message(json),
        "stream_event" => parse_stream_event(json),
        "rate_limit" => parse_rate_limit(json),
        _ => {
            debug!("Unknown message type: {}, wrapping as Message::Unknown", msg_type);
            Ok(Some(Message::Unknown {
                msg_type: msg_type.to_string(),
                raw: json,
            }))
        }
    }
}

/// Parse a user message
fn parse_user_message(json: Value) -> Result<Option<Message>> {
    let message = json
        .get("message")
        .ok_or_else(|| SdkError::parse_error("Missing 'message' field", json.to_string()))?;

    // Handle different content formats
    let content = if let Some(content_str) = message.get("content").and_then(|v| v.as_str()) {
        // Simple string content
        content_str.to_string()
    } else if let Some(_content_array) = message.get("content").and_then(|v| v.as_array()) {
        // Array content (e.g., tool results) - we'll skip these for now
        // as they're not standard user messages but tool responses
        debug!("Skipping user message with array content (likely tool result)");
        return Ok(None);
    } else {
        return Err(SdkError::parse_error(
            "Missing or invalid 'content' field",
            json.to_string(),
        ));
    };

    Ok(Some(Message::User {
        message: UserMessage { content },
    }))
}

/// Parse an assistant message
fn parse_assistant_message(json: Value) -> Result<Option<Message>> {
    let message = json
        .get("message")
        .ok_or_else(|| SdkError::parse_error("Missing 'message' field", json.to_string()))?;

    let content_array = message
        .get("content")
        .and_then(|v| v.as_array())
        .ok_or_else(|| {
            SdkError::parse_error("Missing or invalid 'content' array", json.to_string())
        })?;

    let mut content_blocks = Vec::new();

    for content_item in content_array {
        if let Some(block) = parse_content_block(content_item)? {
            content_blocks.push(block);
        }
    }

    // Extract optional fields from the message object
    let model = message
        .get("model")
        .and_then(|v| v.as_str())
        .map(String::from);
    let usage = message.get("usage").cloned();
    let error = message
        .get("error")
        .and_then(|v| serde_json::from_value::<AssistantMessageError>(v.clone()).ok());
    let parent_tool_use_id = json
        .get("parent_tool_use_id")
        .and_then(|v| v.as_str())
        .map(String::from);

    Ok(Some(Message::Assistant {
        message: AssistantMessage {
            content: content_blocks,
            model,
            usage,
            error,
            parent_tool_use_id,
        },
    }))
}

/// Parse a content block
fn parse_content_block(json: &Value) -> Result<Option<ContentBlock>> {
    // First check if it has a type field
    if let Some(block_type) = json.get("type").and_then(|v| v.as_str()) {
        match block_type {
            "text" => {
                let text = json.get("text").and_then(|v| v.as_str()).ok_or_else(|| {
                    SdkError::parse_error("Missing 'text' field in text block", json.to_string())
                })?;
                Ok(Some(ContentBlock::Text(TextContent {
                    text: text.to_string(),
                })))
            }
            "thinking" => {
                let thinking = json.get("thinking").and_then(|v| v.as_str()).ok_or_else(|| {
                    SdkError::parse_error("Missing 'thinking' field in thinking block", json.to_string())
                })?;
                let signature = json.get("signature").and_then(|v| v.as_str()).ok_or_else(|| {
                    SdkError::parse_error("Missing 'signature' field in thinking block", json.to_string())
                })?;
                Ok(Some(ContentBlock::Thinking(ThinkingContent {
                    thinking: thinking.to_string(),
                    signature: signature.to_string(),
                })))
            }
            "tool_use" => {
                let id = json.get("id").and_then(|v| v.as_str()).ok_or_else(|| {
                    SdkError::parse_error("Missing 'id' field in tool_use block", json.to_string())
                })?;
                let name = json.get("name").and_then(|v| v.as_str()).ok_or_else(|| {
                    SdkError::parse_error(
                        "Missing 'name' field in tool_use block",
                        json.to_string(),
                    )
                })?;
                let input = json
                    .get("input")
                    .cloned()
                    .unwrap_or_else(|| Value::Object(serde_json::Map::new()));

                Ok(Some(ContentBlock::ToolUse(ToolUseContent {
                    id: id.to_string(),
                    name: name.to_string(),
                    input,
                })))
            }
            "tool_result" => {
                let tool_use_id = json
                    .get("tool_use_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        SdkError::parse_error(
                            "Missing 'tool_use_id' field in tool_result block",
                            json.to_string(),
                        )
                    })?;

                let content = if let Some(content_val) = json.get("content") {
                    if let Some(text) = content_val.as_str() {
                        Some(ContentValue::Text(text.to_string()))
                    } else {
                        content_val.as_array().map(|array| ContentValue::Structured(array.clone()))
                    }
                } else {
                    None
                };

                let is_error = json.get("is_error").and_then(|v| v.as_bool());

                Ok(Some(ContentBlock::ToolResult(ToolResultContent {
                    tool_use_id: tool_use_id.to_string(),
                    content,
                    is_error,
                })))
            }
            _ => {
                debug!("Unknown content block type: {}", block_type);
                Ok(None)
            }
        }
    } else {
        // Try to parse as a simple text block (backward compatibility)
        if let Some(text) = json.get("text").and_then(|v| v.as_str()) {
            Ok(Some(ContentBlock::Text(TextContent {
                text: text.to_string(),
            })))
        } else {
            trace!("Skipping non-text content block without type");
            Ok(None)
        }
    }
}

/// Parse a system message
fn parse_system_message(json: Value) -> Result<Option<Message>> {
    let subtype = json
        .get("subtype")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    let data = json
        .get("data")
        .cloned()
        .unwrap_or_else(|| Value::Object(serde_json::Map::new()));

    Ok(Some(Message::System { subtype, data }))
}

/// Parse a result message
fn parse_result_message(json: Value) -> Result<Option<Message>> {
    // Use serde to parse the full result message
    match serde_json::from_value::<Message>(json.clone()) {
        Ok(msg) => Ok(Some(msg)),
        Err(_e) => {
            // Fallback: create a minimal result message
            let subtype = json
                .get("subtype")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();

            let duration_ms = json
                .get("duration_ms")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);

            let session_id = json
                .get("session_id")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();

            Ok(Some(Message::Result {
                subtype,
                duration_ms,
                duration_api_ms: json
                    .get("duration_api_ms")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0),
                is_error: json
                    .get("is_error")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
                num_turns: json.get("num_turns").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                session_id,
                total_cost_usd: json.get("total_cost_usd").and_then(|v| v.as_f64()),
                usage: json.get("usage").cloned(),
                result: json
                    .get("result")
                    .and_then(|v| v.as_str())
                    .map(String::from),
                structured_output: json
                    .get("structured_output")
                    .or_else(|| json.get("structuredOutput"))
                    .and_then(|v| (!v.is_null()).then(|| v.clone())),
                stop_reason: json
                    .get("stop_reason")
                    .and_then(|v| v.as_str())
                    .map(String::from),
            }))
        }
    }
}

/// Parse a stream event message
fn parse_stream_event(json: Value) -> Result<Option<Message>> {
    let uuid = json
        .get("uuid")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let session_id = json
        .get("session_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let event = json
        .get("event")
        .cloned()
        .unwrap_or_else(|| Value::Object(serde_json::Map::new()));
    let parent_tool_use_id = json
        .get("parent_tool_use_id")
        .and_then(|v| v.as_str())
        .map(String::from);

    Ok(Some(Message::StreamEvent {
        uuid,
        session_id,
        event,
        parent_tool_use_id,
    }))
}

/// Parse a rate limit message
fn parse_rate_limit(json: Value) -> Result<Option<Message>> {
    let uuid = json
        .get("uuid")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let session_id = json
        .get("session_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let rate_limit_info = json
        .get("rate_limit_info")
        .cloned()
        .ok_or_else(|| {
            SdkError::parse_error("Missing 'rate_limit_info' field", json.to_string())
        })
        .and_then(|v| {
            serde_json::from_value::<RateLimitInfo>(v)
                .map_err(|e| SdkError::parse_error(format!("Invalid rate_limit_info: {e}"), json.to_string()))
        })?;

    Ok(Some(Message::RateLimit {
        rate_limit_info,
        uuid,
        session_id,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_user_message() {
        let json = json!({
            "type": "user",
            "message": {
                "role": "user",
                "content": "Hello, Claude!"
            }
        });

        let result = parse_message(json).unwrap();
        assert!(result.is_some());

        if let Some(Message::User { message }) = result {
            assert_eq!(message.content, "Hello, Claude!");
        } else {
            panic!("Expected User message");
        }
    }

    #[test]
    fn test_parse_assistant_message_with_text() {
        let json = json!({
            "type": "assistant",
            "message": {
                "role": "assistant",
                "content": [
                    {
                        "type": "text",
                        "text": "Hello! How can I help you?"
                    }
                ]
            }
        });

        let result = parse_message(json).unwrap();
        assert!(result.is_some());

        if let Some(Message::Assistant { message }) = result {
            assert_eq!(message.content.len(), 1);
            if let ContentBlock::Text(text) = &message.content[0] {
                assert_eq!(text.text, "Hello! How can I help you?");
            } else {
                panic!("Expected Text content block");
            }
        } else {
            panic!("Expected Assistant message");
        }
    }

    #[test]
    fn test_parse_thinking_block() {
        let json = json!({
            "type": "thinking",
            "thinking": "Let me analyze this problem...",
            "signature": "thinking_sig_123"
        });

        let result = parse_content_block(&json).unwrap();
        assert!(result.is_some());

        if let Some(ContentBlock::Thinking(thinking)) = result {
            assert_eq!(thinking.thinking, "Let me analyze this problem...");
            assert_eq!(thinking.signature, "thinking_sig_123");
        } else {
            panic!("Expected Thinking content block");
        }
    }

    #[test]
    fn test_parse_tool_use_block() {
        let json = json!({
            "type": "tool_use",
            "id": "tool_123",
            "name": "read_file",
            "input": {
                "path": "/tmp/test.txt"
            }
        });

        let result = parse_content_block(&json).unwrap();
        assert!(result.is_some());

        if let Some(ContentBlock::ToolUse(tool_use)) = result {
            assert_eq!(tool_use.id, "tool_123");
            assert_eq!(tool_use.name, "read_file");
            assert_eq!(tool_use.input["path"], "/tmp/test.txt");
        } else {
            panic!("Expected ToolUse content block");
        }
    }

    #[test]
    fn test_parse_system_message() {
        let json = json!({
            "type": "system",
            "subtype": "status",
            "data": {
                "status": "ready"
            }
        });

        let result = parse_message(json).unwrap();
        assert!(result.is_some());

        if let Some(Message::System { subtype, data }) = result {
            assert_eq!(subtype, "status");
            assert_eq!(data["status"], "ready");
        } else {
            panic!("Expected System message");
        }
    }

    #[test]
    fn test_parse_result_message() {
        let json = json!({
            "type": "result",
            "subtype": "conversation_turn",
            "duration_ms": 1234,
            "duration_api_ms": 1000,
            "is_error": false,
            "num_turns": 1,
            "session_id": "test_session",
            "total_cost_usd": 0.001
        });

        let result = parse_message(json).unwrap();
        assert!(result.is_some());

        if let Some(Message::Result {
            subtype,
            duration_ms,
            session_id,
            total_cost_usd,
            ..
        }) = result
        {
            assert_eq!(subtype, "conversation_turn");
            assert_eq!(duration_ms, 1234);
            assert_eq!(session_id, "test_session");
            assert_eq!(total_cost_usd, Some(0.001));
        } else {
            panic!("Expected Result message");
        }
    }

    #[test]
    fn test_parse_result_message_structured_output_alias() {
        let json = json!({
            "type": "result",
            "subtype": "conversation_turn",
            "duration_ms": 1,
            "duration_api_ms": 1,
            "is_error": false,
            "num_turns": 1,
            "session_id": "test_session",
            "structuredOutput": {"answer": 42}
        });

        let result = parse_message(json).unwrap();
        assert!(result.is_some());

        if let Some(Message::Result { structured_output, .. }) = result {
            assert_eq!(structured_output, Some(json!({"answer": 42})));
        } else {
            panic!("Expected Result message");
        }
    }

    #[test]
    fn test_parse_unknown_message_type() {
        let json = json!({
            "type": "unknown_type",
            "data": "some data"
        });

        let result = parse_message(json).unwrap();
        assert!(result.is_some());
        if let Some(Message::Unknown { msg_type, raw }) = result {
            assert_eq!(msg_type, "unknown_type");
            assert_eq!(raw["data"], "some data");
        } else {
            panic!("Expected Unknown message");
        }
    }
}
