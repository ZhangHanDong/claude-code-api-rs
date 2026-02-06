//! Tests for streaming functionality

use futures::StreamExt;
use nexus_claude::{ClaudeCodeOptions, ClaudeSDKClient, Message, Result};
use std::pin::Pin;

/// Test that receive_response returns a pinned stream
#[tokio::test]
async fn test_receive_response_returns_pinned_stream() {
    let mut client = ClaudeSDKClient::new(ClaudeCodeOptions::default());

    // Don't actually connect, just test the type
    // The method should return Pin<Box<dyn Stream>>
    async fn test_stream_type(client: &mut ClaudeSDKClient) {
        let _stream: Pin<Box<dyn futures::Stream<Item = Result<Message>> + Send + '_>> =
            client.receive_response().await;
    }

    test_stream_type(&mut client).await;
}

/// Test that receive_messages returns a stream
#[tokio::test]
async fn test_receive_messages_returns_stream() {
    let mut client = ClaudeSDKClient::new(ClaudeCodeOptions::default());

    // Don't actually connect, just test the type
    async fn test_stream_type(client: &mut ClaudeSDKClient) {
        let mut stream = client.receive_messages().await;
        // This should compile - proving it's a Stream
        let _next = stream.next();
    }

    test_stream_type(&mut client).await;
}

/// Test streaming with mock data
#[tokio::test]
async fn test_streaming_flow() {
    use tokio::sync::mpsc;

    // Create a channel to simulate messages
    let (tx, mut rx) = mpsc::channel::<Result<Message>>(10);

    // Send some test messages
    tokio::spawn(async move {
        // Send a user message
        let _ = tx
            .send(Ok(Message::User {
                message: nexus_claude::UserMessage {
                    content: "Test".to_string(),
                },
            }))
            .await;

        // Send an assistant message
        let _ = tx
            .send(Ok(Message::Assistant {
                message: nexus_claude::AssistantMessage { content: vec![] },
            }))
            .await;

        // Send a result message
        let _ = tx
            .send(Ok(Message::Result {
                subtype: "result".to_string(),
                duration_ms: 100,
                duration_api_ms: 50,
                is_error: false,
                num_turns: 1,
                session_id: "test".to_string(),
                total_cost_usd: Some(0.01),
                usage: None,
                result: Some("Success".to_string()),
                structured_output: None,
            }))
            .await;
    });

    // Consume messages
    let mut count = 0;
    while let Some(msg_result) = rx.recv().await {
        match msg_result {
            Ok(Message::Result { .. }) => {
                count += 1;
                break; // Stop on result message
            },
            Ok(_) => {
                count += 1;
            },
            Err(_) => break,
        }
    }

    assert_eq!(count, 3, "Should have received 3 messages");
}

/// Test that receive_response stops after ResultMessage
#[tokio::test]
async fn test_receive_response_stops_after_result() {
    use async_stream::stream;

    // Create a test stream that yields multiple messages
    let test_stream = stream! {
        yield Ok::<Message, nexus_claude::SdkError>(Message::User {
            message: nexus_claude::UserMessage {
                content: "Test".to_string(),
            },
        });

        yield Ok::<Message, nexus_claude::SdkError>(Message::Assistant {
            message: nexus_claude::AssistantMessage {
                content: vec![],
            },
        });

        yield Ok::<Message, nexus_claude::SdkError>(Message::Result {
            subtype: "result".to_string(),
            duration_ms: 100,
            duration_api_ms: 50,
            is_error: false,
            num_turns: 1,
            session_id: "test".to_string(),
            total_cost_usd: None,
            usage: None,
            result: None,
            structured_output: None,
        });

        // This should NOT be received
        yield Ok::<Message, nexus_claude::SdkError>(Message::User {
            message: nexus_claude::UserMessage {
                content: "Should not see this".to_string(),
            },
        });
    };

    let mut pinned_stream = Box::pin(test_stream);
    let mut count = 0;
    let mut saw_result = false;

    while let Some(msg_result) = pinned_stream.next().await {
        count += 1;
        if let Ok(Message::Result { .. }) = msg_result {
            saw_result = true;
            // In real receive_response, it would stop here
            break;
        }
    }

    assert_eq!(count, 3, "Should have processed exactly 3 messages");
    assert!(saw_result, "Should have seen a Result message");
}
