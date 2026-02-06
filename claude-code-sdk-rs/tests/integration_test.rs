//! Integration tests for the Claude Code SDK

use nexus_claude::{ClaudeCodeOptions, PermissionMode};

#[test]
#[allow(deprecated)]
fn test_options_builder() {
    let options = ClaudeCodeOptions::builder()
        .system_prompt("Test prompt")
        .model("claude-3-opus")
        .permission_mode(PermissionMode::AcceptEdits)
        .allow_tool("read")
        .allow_tool("write")
        .max_turns(10)
        .max_thinking_tokens(5000)
        .build();

    assert_eq!(options.system_prompt, Some("Test prompt".to_string()));
    assert_eq!(options.model, Some("claude-3-opus".to_string()));
    assert_eq!(options.permission_mode, PermissionMode::AcceptEdits);
    assert_eq!(options.allowed_tools, vec!["read", "write"]);
    assert_eq!(options.max_turns, Some(10));
    assert_eq!(options.max_thinking_tokens, 5000);
}

#[test]
fn test_message_types() {
    use nexus_claude::{ContentBlock, Message, TextContent, UserMessage};

    let user_msg = Message::User {
        message: UserMessage {
            content: "Hello".to_string(),
        },
    };

    match user_msg {
        Message::User { message } => {
            assert_eq!(message.content, "Hello");
        },
        _ => panic!("Expected User message"),
    }

    let text_block = ContentBlock::Text(TextContent {
        text: "Response text".to_string(),
    });

    match text_block {
        ContentBlock::Text(text) => {
            assert_eq!(text.text, "Response text");
        },
        _ => panic!("Expected Text block"),
    }
}

#[test]
fn test_error_types() {
    use nexus_claude::SdkError;

    let err = SdkError::timeout(30);
    assert!(err.is_recoverable());
    assert!(!err.is_config_error());

    let err = SdkError::ConfigError("Invalid config".into());
    assert!(!err.is_recoverable());
    assert!(err.is_config_error());
}
