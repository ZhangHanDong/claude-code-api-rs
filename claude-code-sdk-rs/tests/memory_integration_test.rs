//! Integration tests for the memory system.
//!
//! These tests require a running Meilisearch instance.
//! Run with: cargo test --features memory --test memory_integration_test

#![cfg(feature = "memory")]

use nexus_claude::memory::{MemoryIntegrationBuilder, MessageDocument, SummaryGenerator};
use serde_json::json;

/// Test the full conversation flow with memory capture.
#[test]
fn test_conversation_memory_flow() {
    // Setup
    let mut manager = MemoryIntegrationBuilder::new()
        .enabled(true)
        .cwd("/projects/test-app")
        .conversation_id("test-conv-001")
        .min_relevance_score(0.3)
        .max_context_items(5)
        .build();

    // Simulate a conversation turn

    // 1. User sends a message
    manager.record_user_message("How do I implement JWT authentication?");

    // 2. Assistant uses tools during response
    manager.process_tool_call(
        "Grep",
        &json!({
            "pattern": "jwt|auth",
            "path": "/projects/test-app/src"
        }),
    );
    manager.process_tool_call(
        "Read",
        &json!({
            "file_path": "/projects/test-app/src/auth.rs"
        }),
    );
    manager.process_tool_call(
        "Edit",
        &json!({
            "file_path": "/projects/test-app/src/auth.rs",
            "old_string": "fn authenticate()",
            "new_string": "fn authenticate_jwt()"
        }),
    );

    // 3. Assistant completes response
    manager.record_assistant_message(
        "I've updated the authentication to use JWT. The changes are in src/auth.rs.",
    );

    // Verify captured context
    let messages = manager.take_pending_messages();
    assert_eq!(messages.len(), 2);

    // Check user message
    let user_msg = &messages[0];
    assert_eq!(user_msg.role, "user");
    assert_eq!(user_msg.cwd, Some("/projects/test-app".to_string()));

    // Check assistant message with captured files
    let assistant_msg = &messages[1];
    assert_eq!(assistant_msg.role, "assistant");
    assert!(
        assistant_msg
            .files_touched
            .contains(&"/projects/test-app/src/auth.rs".to_string())
    );
    assert_eq!(assistant_msg.turn_index, 0);

    // Verify turn incremented
    assert_eq!(manager.turn_index(), 1);
}

/// Test that disabled memory doesn't capture anything.
#[test]
fn test_disabled_memory_noop() {
    let mut manager = MemoryIntegrationBuilder::new()
        .enabled(false)
        .cwd("/projects/secret")
        .build();

    manager.record_user_message("Sensitive information");
    manager.process_tool_call(
        "Read",
        &json!({
            "file_path": "/projects/secret/passwords.txt"
        }),
    );
    manager.record_assistant_message("Here's the secret data...");

    // Nothing should be captured
    let messages = manager.take_pending_messages();
    assert!(messages.is_empty());
}

/// Test cwd tracking across tool calls.
#[test]
fn test_cwd_tracking() {
    let mut manager = MemoryIntegrationBuilder::new()
        .enabled(true)
        .cwd("/home/user")
        .build();

    assert_eq!(manager.cwd(), Some("/home/user"));

    // cd command should update cwd
    manager.process_tool_call(
        "Bash",
        &json!({
            "command": "cd /projects/app && cargo build"
        }),
    );

    assert_eq!(manager.cwd(), Some("/projects/app"));

    // New context should use updated cwd
    let ctx = manager.current_context("test query");
    assert_eq!(ctx.cwd, Some("/projects/app".to_string()));
}

/// Test file aggregation across multiple tool calls.
#[test]
fn test_file_aggregation() {
    let mut manager = MemoryIntegrationBuilder::new().enabled(true).build();

    manager.process_tool_call(
        "Read",
        &json!({
            "file_path": "/src/main.rs"
        }),
    );
    manager.process_tool_call(
        "Read",
        &json!({
            "file_path": "/src/lib.rs"
        }),
    );
    manager.process_tool_call(
        "Edit",
        &json!({
            "file_path": "/src/main.rs",
            "old_string": "a",
            "new_string": "b"
        }),
    );

    let ctx = manager.current_context("query");

    // Should have 2 unique files (main.rs deduplicated)
    assert_eq!(ctx.files.len(), 2);
    assert!(ctx.files.contains(&"/src/lib.rs".to_string()));
    assert!(ctx.files.contains(&"/src/main.rs".to_string()));
}

/// Test summary generation for long content.
#[test]
fn test_summary_generation() {
    let generator = SummaryGenerator::new(100);

    // Short content - no summary needed
    let short = "This is short.";
    assert!(!generator.needs_summary(short));
    assert_eq!(generator.generate_simple_summary(short), short);

    // Long content - summary generated
    let long = "First sentence with important information. \
                Second sentence with more details. \
                Third sentence continues the explanation. \
                Fourth sentence adds context. \
                Fifth sentence concludes the message.";

    assert!(generator.needs_summary(long));

    let summary = generator.generate_simple_summary(long);
    assert!(summary.len() < long.len());
    assert!(summary.contains("First sentence"));
    assert!(summary.contains("..."));
}

/// Test multiple conversation turns.
#[test]
fn test_multiple_turns() {
    let mut manager = MemoryIntegrationBuilder::new().enabled(true).build();

    // Turn 1
    manager.record_user_message("Question 1");
    manager.record_assistant_message("Answer 1");
    assert_eq!(manager.turn_index(), 1);

    // Turn 2
    manager.record_user_message("Question 2");
    manager.process_tool_call("Read", &json!({"file_path": "/file.rs"}));
    manager.record_assistant_message("Answer 2");
    assert_eq!(manager.turn_index(), 2);

    // Turn 3
    manager.record_user_message("Question 3");
    manager.record_assistant_message("Answer 3");
    assert_eq!(manager.turn_index(), 3);

    // All 6 messages should be pending
    let messages = manager.take_pending_messages();
    assert_eq!(messages.len(), 6);

    // Verify turn indices
    assert_eq!(messages[0].turn_index, 0);
    assert_eq!(messages[1].turn_index, 0);
    assert_eq!(messages[2].turn_index, 1);
    assert_eq!(messages[3].turn_index, 1);
    assert_eq!(messages[4].turn_index, 2);
    assert_eq!(messages[5].turn_index, 2);
}

/// Test MessageDocument serialization roundtrip.
#[test]
fn test_message_document_serialization() {
    let msg = MessageDocument::new(
        "msg-123",
        "conv-456",
        "assistant",
        "Full content here",
        3,
        1700000000,
    )
    .with_cwd("/projects/app")
    .with_files_touched(vec!["/src/main.rs".to_string(), "/src/lib.rs".to_string()])
    .with_summary("Brief summary");

    // Serialize
    let json = serde_json::to_string(&msg).unwrap();

    // Deserialize
    let parsed: MessageDocument = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.id, "msg-123");
    assert_eq!(parsed.conversation_id, "conv-456");
    assert_eq!(parsed.role, "assistant");
    assert_eq!(parsed.content, "Full content here");
    assert_eq!(parsed.turn_index, 3);
    assert_eq!(parsed.cwd, Some("/projects/app".to_string()));
    assert_eq!(parsed.files_touched.len(), 2);
    assert_eq!(parsed.summary, Some("Brief summary".to_string()));

    // display_content should prefer summary
    assert_eq!(parsed.display_content(), "Brief summary");
}

/// Test query context construction.
#[test]
fn test_query_context() {
    let mut manager = MemoryIntegrationBuilder::new()
        .enabled(true)
        .cwd("/projects/api")
        .build();

    manager.process_tool_call(
        "Read",
        &json!({
            "file_path": "/projects/api/src/routes.rs"
        }),
    );

    let ctx = manager.current_context("How do I add a new route?");

    assert_eq!(ctx.query, "How do I add a new route?");
    assert_eq!(ctx.cwd, Some("/projects/api".to_string()));
    assert!(
        ctx.files
            .contains(&"/projects/api/src/routes.rs".to_string())
    );
}
