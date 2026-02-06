//! Mock tests for API functionality without real Claude connection

use nexus_claude::{
    AssistantMessage, ClaudeCodeOptions, ContentBlock, Message, PermissionMode, TextContent,
};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Mock response generator for testing
struct MockResponseGenerator {
    responses: Arc<RwLock<Vec<Message>>>,
}

impl MockResponseGenerator {
    fn new() -> Self {
        Self {
            responses: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Add a mock response
    async fn add_response(&self, content: &str) {
        let assistant_msg = AssistantMessage {
            content: vec![ContentBlock::Text(TextContent {
                text: content.to_string(),
            })],
        };

        let message = Message::Assistant {
            message: assistant_msg,
        };

        self.responses.write().await.push(message);

        // Add Result message to simulate completion
        self.responses.write().await.push(Message::Result {
            subtype: "done".to_string(),
            duration_ms: 1000,
            duration_api_ms: 800,
            is_error: false,
            num_turns: 1,
            session_id: "test-session".to_string(),
            total_cost_usd: None,
            usage: None,
            result: Some("Success".to_string()),
            structured_output: None,
        });
    }

    /// Get all responses
    async fn get_responses(&self) -> Vec<Message> {
        self.responses.read().await.clone()
    }
}

/// Test message serialization
#[test]
fn test_message_serialization() {
    let assistant_msg = AssistantMessage {
        content: vec![ContentBlock::Text(TextContent {
            text: "Hello, world!".to_string(),
        })],
    };

    let message = Message::Assistant {
        message: assistant_msg,
    };

    // Serialize to JSON
    let json = serde_json::to_string(&message).unwrap();
    assert!(json.contains("assistant"));
    assert!(json.contains("Hello, world!"));

    // Deserialize back
    let deserialized: Message = serde_json::from_str(&json).unwrap();
    match deserialized {
        Message::Assistant { message } => {
            assert_eq!(message.content.len(), 1);
            if let ContentBlock::Text(text) = &message.content[0] {
                assert_eq!(text.text, "Hello, world!");
            } else {
                panic!("Expected text content");
            }
        },
        _ => panic!("Expected assistant message"),
    }
}

/// Test options building
#[test]
#[allow(deprecated)]
fn test_options_builder() {
    let options = ClaudeCodeOptions::builder()
        .permission_mode(PermissionMode::AcceptEdits)
        .model("claude-3-opus")
        .system_prompt("You are a helpful assistant")
        .allowed_tools(vec!["Bash".to_string(), "Read".to_string()])
        .disallowed_tools(vec!["Write".to_string()])
        .build();

    assert_eq!(options.permission_mode, PermissionMode::AcceptEdits);
    assert_eq!(options.model, Some("claude-3-opus".to_string()));
    assert_eq!(
        options.system_prompt,
        Some("You are a helpful assistant".to_string())
    );
    assert_eq!(options.allowed_tools, vec!["Bash", "Read"]);
    assert_eq!(options.disallowed_tools, vec!["Write"]);
}

/// Test mock response flow
#[tokio::test]
async fn test_mock_response_flow() {
    let mock = MockResponseGenerator::new();

    // Simulate adding responses
    mock.add_response("The answer is 42").await;

    let responses = mock.get_responses().await;
    assert_eq!(responses.len(), 2); // Assistant message + Result message

    // Verify first message is assistant response
    match &responses[0] {
        Message::Assistant { message } => {
            assert_eq!(message.content.len(), 1);
            if let ContentBlock::Text(text) = &message.content[0] {
                assert_eq!(text.text, "The answer is 42");
            }
        },
        _ => panic!("Expected assistant message"),
    }

    // Verify second message is result
    match &responses[1] {
        Message::Result { .. } => (),
        _ => panic!("Expected result message"),
    }
}

/// Test error handling patterns
#[test]
fn test_error_patterns() {
    use nexus_claude::SdkError;

    // Test timeout error
    let timeout_err = SdkError::Timeout { seconds: 30 };
    assert!(timeout_err.is_recoverable());
    assert!(!timeout_err.is_config_error());

    // Test config error
    let config_err = SdkError::ConfigError("Invalid model".to_string());
    assert!(!config_err.is_recoverable());
    assert!(config_err.is_config_error());

    // Test invalid state error
    let state_err = SdkError::InvalidState {
        message: "Not connected".to_string(),
    };
    assert!(!state_err.is_recoverable());
    assert!(!state_err.is_config_error());
}

/// Test concurrent message processing simulation
#[tokio::test]
async fn test_concurrent_processing() {
    let mock = MockResponseGenerator::new();

    // Simulate concurrent responses
    let handles: Vec<_> = (0..5)
        .map(|i| {
            let mock_clone = mock.clone();
            tokio::spawn(async move {
                mock_clone.add_response(&format!("Response {i}")).await;
            })
        })
        .collect();

    // Wait for all to complete
    for handle in handles {
        handle.await.unwrap();
    }

    let responses = mock.get_responses().await;
    // Each add_response adds 2 messages (assistant + result)
    assert_eq!(responses.len(), 10);
}

// Implement Clone for MockResponseGenerator
impl Clone for MockResponseGenerator {
    fn clone(&self) -> Self {
        Self {
            responses: self.responses.clone(),
        }
    }
}
