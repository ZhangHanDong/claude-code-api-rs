#[cfg(test)]
mod tests {
    use super::super::openai::*;
    use super::super::claude::*;
    
    #[test]
    fn test_chat_message_serialization() {
        let message = ChatMessage {
            role: "user".to_string(),
            content: MessageContent::Text("Hello".to_string()),
            name: None,
        };
        
        let json = serde_json::to_string(&message).unwrap();
        assert!(json.contains("\"role\":\"user\""));
        assert!(json.contains("\"content\":\"Hello\""));
    }
    
    #[test]
    fn test_claude_model_list() {
        let models = ClaudeModel::all();
        assert_eq!(models.len(), 3);
        
        let model_ids: Vec<String> = models.iter().map(|m| m.id.clone()).collect();
        assert!(model_ids.contains(&"claude-3-5-sonnet-20241022".to_string()));
        assert!(model_ids.contains(&"claude-3-opus-20240229".to_string()));
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
}