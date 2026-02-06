#[cfg(test)]
#[allow(clippy::module_inception)]
mod tests {
    use super::super::claude::*;
    use super::super::openai::*;

    #[test]
    fn test_chat_message_serialization() {
        let message = ChatMessage {
            role: "user".to_string(),
            content: Some(MessageContent::Text("Hello".to_string())),
            name: None,
            tool_calls: None,
        };

        let json = serde_json::to_string(&message).unwrap();
        assert!(json.contains("\"role\":\"user\""));
        assert!(json.contains("\"content\":\"Hello\""));
    }

    #[test]
    fn test_claude_model_list() {
        let models = ClaudeModel::all();
        assert_eq!(models.len(), 8); // 3 Claude 4 + 2 Claude 3.7 + 2 Claude 3.5 + 1 Claude 3

        let model_ids: Vec<String> = models.iter().map(|m| m.id.clone()).collect();
        // Check Claude 4 models
        assert!(model_ids.contains(&"claude-opus-4-1-20250805".to_string()));
        assert!(model_ids.contains(&"claude-opus-4-20250514".to_string()));
        assert!(model_ids.contains(&"claude-sonnet-4-20250514".to_string()));
        // Check Claude 3.7 models
        assert!(model_ids.contains(&"claude-3-7-sonnet-20250219".to_string()));
        assert!(model_ids.contains(&"claude-3-7-sonnet-latest".to_string()));
        // Check Claude 3.5 models
        assert!(model_ids.contains(&"claude-3-5-haiku-20241022".to_string()));
        assert!(model_ids.contains(&"claude-3-5-haiku-latest".to_string()));
        // Check Claude 3 models
        assert!(model_ids.contains(&"claude-3-haiku-20240307".to_string()));
    }

    #[test]
    fn test_message_content_variants() {
        let text_content = MessageContent::Text("Hello".to_string());
        let array_content = MessageContent::Array(vec![
            ContentPart::Text {
                text: "Hello".to_string(),
            },
            ContentPart::ImageUrl {
                image_url: ImageUrl {
                    url: "https://example.com/image.png".to_string(),
                    detail: Some("high".to_string()),
                },
            },
        ]);

        let text_json = serde_json::to_string(&text_content).unwrap();
        assert_eq!(text_json, "\"Hello\"");

        let array_json = serde_json::to_string(&array_content).unwrap();
        assert!(array_json.contains("\"type\":\"text\""));
        assert!(array_json.contains("\"type\":\"image_url\""));
    }
}
