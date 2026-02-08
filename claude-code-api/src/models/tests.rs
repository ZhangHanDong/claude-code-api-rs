#[cfg(test)]
mod tests {
    use super::super::openai::*;
    use super::super::claude::*;
    
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
        assert_eq!(models.len(), 11); // 3 Claude 4.5/4.6 + 3 Claude 4 + 2 Claude 3.7 + 2 Claude 3.5 + 1 Claude 3

        let model_ids: Vec<String> = models.iter().map(|m| m.id.clone()).collect();
        // Check Claude 4.5/4.6 models (latest)
        assert!(model_ids.contains(&"claude-opus-4-6".to_string()));
        assert!(model_ids.contains(&"claude-sonnet-4-5-20250929".to_string()));
        assert!(model_ids.contains(&"claude-haiku-4-5-20251001".to_string()));
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
            ContentPart::Text { text: "Hello".to_string() },
            ContentPart::ImageUrl { 
                image_url: ImageUrl {
                    url: "https://example.com/image.png".to_string(),
                    detail: Some("high".to_string()),
                }
            },
        ]);
        
        let text_json = serde_json::to_string(&text_content).unwrap();
        assert_eq!(text_json, "\"Hello\"");
        
        let array_json = serde_json::to_string(&array_content).unwrap();
        assert!(array_json.contains("\"type\":\"text\""));
        assert!(array_json.contains("\"type\":\"image_url\""));
    }

    #[test]
    fn test_tool_choice_deserialize_string_auto() {
        let json = serde_json::json!("auto");
        let tc: ToolChoice = serde_json::from_value(json).unwrap();
        assert!(matches!(tc, ToolChoice::Auto));
    }

    #[test]
    fn test_tool_choice_deserialize_string_none() {
        let json = serde_json::json!("none");
        let tc: ToolChoice = serde_json::from_value(json).unwrap();
        assert!(matches!(tc, ToolChoice::None));
    }

    #[test]
    fn test_tool_choice_deserialize_string_required() {
        let json = serde_json::json!("required");
        let tc: ToolChoice = serde_json::from_value(json).unwrap();
        assert!(matches!(tc, ToolChoice::Required));
    }

    #[test]
    fn test_tool_choice_deserialize_object() {
        let json = serde_json::json!({
            "type": "function",
            "function": { "name": "get_weather" }
        });
        let tc: ToolChoice = serde_json::from_value(json).unwrap();
        match tc {
            ToolChoice::Tool { tool_type, function } => {
                assert_eq!(tool_type, "function");
                assert_eq!(function.name, "get_weather");
            }
            _ => panic!("Expected ToolChoice::Tool"),
        }
    }

    #[test]
    fn test_tool_choice_deserialize_invalid_string() {
        let json = serde_json::json!("invalid");
        let result: Result<ToolChoice, _> = serde_json::from_value(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_chat_completion_request_with_tool_choice_string() {
        let json = serde_json::json!({
            "model": "claude-3-opus",
            "messages": [{"role": "user", "content": "Hello"}],
            "tool_choice": "auto",
            "tools": [{
                "type": "function",
                "function": {
                    "name": "get_weather",
                    "description": "Get the weather",
                    "parameters": {"type": "object", "properties": {}}
                }
            }]
        });
        let req: ChatCompletionRequest = serde_json::from_value(json).unwrap();
        assert!(matches!(req.tool_choice, Some(ToolChoice::Auto)));
        assert!(req.tools.is_some());
        assert_eq!(req.tools.unwrap().len(), 1);
    }
}