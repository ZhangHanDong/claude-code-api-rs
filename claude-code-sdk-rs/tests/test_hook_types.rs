//! Unit tests for strongly-typed Hook types (v0.3.0)
//!
//! These tests verify that the new Hook Input and Output types
//! serialize and deserialize correctly for communication with the CLI.

use nexus_claude::{
    AsyncHookJSONOutput, HookInput, HookJSONOutput, HookSpecificOutput,
    PostToolUseHookSpecificOutput, PreToolUseHookSpecificOutput, SyncHookJSONOutput,
    UserPromptSubmitHookSpecificOutput,
};
use serde_json::json;

#[test]
fn test_pre_tool_use_hook_input_deserialization() {
    let json_str = r#"{
        "hook_event_name": "PreToolUse",
        "session_id": "test-session",
        "transcript_path": "/path/to/transcript",
        "cwd": "/current/dir",
        "permission_mode": "default",
        "tool_name": "Bash",
        "tool_input": {"command": "ls"}
    }"#;

    let result: Result<HookInput, _> = serde_json::from_str(json_str);
    assert!(
        result.is_ok(),
        "Failed to deserialize PreToolUse hook input"
    );

    if let HookInput::PreToolUse(input) = result.unwrap() {
        assert_eq!(input.tool_name, "Bash");
        assert_eq!(input.session_id, "test-session");
        assert_eq!(input.cwd, "/current/dir");
    } else {
        panic!("Expected PreToolUse variant");
    }
}

#[test]
fn test_post_tool_use_hook_input_deserialization() {
    let json_str = r#"{
        "hook_event_name": "PostToolUse",
        "session_id": "test-session",
        "transcript_path": "/path/to/transcript",
        "cwd": "/current/dir",
        "tool_name": "Bash",
        "tool_input": {"command": "ls"},
        "tool_response": {"output": "file1.txt\nfile2.txt"}
    }"#;

    let result: Result<HookInput, _> = serde_json::from_str(json_str);
    assert!(
        result.is_ok(),
        "Failed to deserialize PostToolUse hook input"
    );

    if let HookInput::PostToolUse(input) = result.unwrap() {
        assert_eq!(input.tool_name, "Bash");
        assert_eq!(input.session_id, "test-session");
    } else {
        panic!("Expected PostToolUse variant");
    }
}

#[test]
fn test_user_prompt_submit_hook_input_deserialization() {
    let json_str = r#"{
        "hook_event_name": "UserPromptSubmit",
        "session_id": "test-session",
        "transcript_path": "/path/to/transcript",
        "cwd": "/current/dir",
        "prompt": "What is 2 + 2?"
    }"#;

    let result: Result<HookInput, _> = serde_json::from_str(json_str);
    assert!(
        result.is_ok(),
        "Failed to deserialize UserPromptSubmit hook input"
    );

    if let HookInput::UserPromptSubmit(input) = result.unwrap() {
        assert_eq!(input.prompt, "What is 2 + 2?");
        assert_eq!(input.session_id, "test-session");
    } else {
        panic!("Expected UserPromptSubmit variant");
    }
}

#[test]
fn test_stop_hook_input_deserialization() {
    let json_str = r#"{
        "hook_event_name": "Stop",
        "session_id": "test-session",
        "transcript_path": "/path/to/transcript",
        "cwd": "/current/dir",
        "stop_hook_active": true
    }"#;

    let result: Result<HookInput, _> = serde_json::from_str(json_str);
    assert!(result.is_ok(), "Failed to deserialize Stop hook input");

    if let HookInput::Stop(input) = result.unwrap() {
        assert!(input.stop_hook_active);
    } else {
        panic!("Expected Stop variant");
    }
}

#[test]
fn test_subagent_stop_hook_input_deserialization() {
    let json_str = r#"{
        "hook_event_name": "SubagentStop",
        "session_id": "test-session",
        "transcript_path": "/path/to/transcript",
        "cwd": "/current/dir",
        "stop_hook_active": false
    }"#;

    let result: Result<HookInput, _> = serde_json::from_str(json_str);
    assert!(
        result.is_ok(),
        "Failed to deserialize SubagentStop hook input"
    );

    if let HookInput::SubagentStop(input) = result.unwrap() {
        assert!(!input.stop_hook_active);
    } else {
        panic!("Expected SubagentStop variant");
    }
}

#[test]
fn test_pre_compact_hook_input_deserialization() {
    let json_str = r#"{
        "hook_event_name": "PreCompact",
        "session_id": "test-session",
        "transcript_path": "/path/to/transcript",
        "cwd": "/current/dir",
        "trigger": "manual",
        "custom_instructions": "Keep important context"
    }"#;

    let result: Result<HookInput, _> = serde_json::from_str(json_str);
    assert!(
        result.is_ok(),
        "Failed to deserialize PreCompact hook input"
    );

    if let HookInput::PreCompact(input) = result.unwrap() {
        assert_eq!(input.trigger, "manual");
        assert_eq!(
            input.custom_instructions,
            Some("Keep important context".to_string())
        );
    } else {
        panic!("Expected PreCompact variant");
    }
}

#[test]
fn test_sync_hook_output_serialization() {
    let output = SyncHookJSONOutput {
        continue_: Some(true),
        suppress_output: Some(false),
        stop_reason: Some("Completed successfully".to_string()),
        decision: Some("block".to_string()),
        system_message: Some("Test message".to_string()),
        reason: Some("Test reason".to_string()),
        hook_specific_output: None,
    };

    let json = serde_json::to_value(&output).expect("Failed to serialize SyncHookJSONOutput");

    // Verify field name conversion (continue_ -> continue)
    assert!(json.get("continue").is_some(), "Missing 'continue' field");
    assert_eq!(json["continue"], true);
    assert_eq!(json["suppressOutput"], false);
    assert_eq!(json["stopReason"], "Completed successfully");
}

#[test]
fn test_async_hook_output_serialization() {
    let output = AsyncHookJSONOutput {
        async_: true,
        async_timeout: Some(5000),
    };

    let json = serde_json::to_value(&output).expect("Failed to serialize AsyncHookJSONOutput");

    // Verify field name conversion (async_ -> async)
    assert!(json.get("async").is_some(), "Missing 'async' field");
    assert_eq!(json["async"], true);
    assert_eq!(json["asyncTimeout"], 5000);
}

#[test]
fn test_hook_json_output_enum_serialization() {
    // Test Sync variant
    let sync_output = HookJSONOutput::Sync(SyncHookJSONOutput {
        continue_: Some(false),
        ..Default::default()
    });

    let json = serde_json::to_value(&sync_output).expect("Failed to serialize HookJSONOutput");
    assert_eq!(json["continue"], false);

    // Test Async variant
    let async_output = HookJSONOutput::Async(AsyncHookJSONOutput {
        async_: true,
        async_timeout: None,
    });

    let json = serde_json::to_value(&async_output).expect("Failed to serialize HookJSONOutput");
    assert_eq!(json["async"], true);
}

#[test]
fn test_pre_tool_use_hook_specific_output() {
    let specific_output = HookSpecificOutput::PreToolUse(PreToolUseHookSpecificOutput {
        permission_decision: Some("deny".to_string()),
        permission_decision_reason: Some("Tool not allowed".to_string()),
        updated_input: Some(json!({"modified": true})),
    });

    let json = serde_json::to_value(&specific_output)
        .expect("Failed to serialize PreToolUseHookSpecificOutput");

    assert_eq!(json["hookEventName"], "PreToolUse");
    assert_eq!(json["permissionDecision"], "deny");
    assert_eq!(json["permissionDecisionReason"], "Tool not allowed");
}

#[test]
fn test_post_tool_use_hook_specific_output() {
    let specific_output = HookSpecificOutput::PostToolUse(PostToolUseHookSpecificOutput {
        additional_context: Some("Tool execution was successful".to_string()),
    });

    let json = serde_json::to_value(&specific_output)
        .expect("Failed to serialize PostToolUseHookSpecificOutput");

    assert_eq!(json["hookEventName"], "PostToolUse");
    assert_eq!(json["additionalContext"], "Tool execution was successful");
}

#[test]
fn test_user_prompt_submit_hook_specific_output() {
    let specific_output =
        HookSpecificOutput::UserPromptSubmit(UserPromptSubmitHookSpecificOutput {
            additional_context: Some("Remember to be concise".to_string()),
        });

    let json = serde_json::to_value(&specific_output)
        .expect("Failed to serialize UserPromptSubmitHookSpecificOutput");

    assert_eq!(json["hookEventName"], "UserPromptSubmit");
    assert_eq!(json["additionalContext"], "Remember to be concise");
}

#[test]
fn test_hook_specific_output_discriminated_union() {
    // Test PreToolUse variant
    let pre_tool_use = HookSpecificOutput::PreToolUse(PreToolUseHookSpecificOutput {
        permission_decision: Some("allow".to_string()),
        permission_decision_reason: None,
        updated_input: None,
    });

    let json = serde_json::to_value(&pre_tool_use).expect("Failed to serialize HookSpecificOutput");
    assert_eq!(json["hookEventName"], "PreToolUse");
    assert_eq!(json["permissionDecision"], "allow");

    // Test PostToolUse variant
    let post_tool_use = HookSpecificOutput::PostToolUse(PostToolUseHookSpecificOutput {
        additional_context: Some("Context added".to_string()),
    });

    let json =
        serde_json::to_value(&post_tool_use).expect("Failed to serialize HookSpecificOutput");
    assert_eq!(json["hookEventName"], "PostToolUse");
    assert_eq!(json["additionalContext"], "Context added");
}

#[test]
fn test_sync_hook_output_with_hook_specific() {
    use nexus_claude::HookSpecificOutput;

    let output = SyncHookJSONOutput {
        continue_: Some(true),
        hook_specific_output: Some(HookSpecificOutput::PreToolUse(
            PreToolUseHookSpecificOutput {
                permission_decision: Some("ask".to_string()),
                permission_decision_reason: Some("Requires confirmation".to_string()),
                updated_input: None,
            },
        )),
        ..Default::default()
    };

    let json = serde_json::to_value(&output).expect("Failed to serialize complete hook output");

    assert_eq!(json["continue"], true);
    assert_eq!(json["hookSpecificOutput"]["hookEventName"], "PreToolUse");
    assert_eq!(json["hookSpecificOutput"]["permissionDecision"], "ask");
}

#[test]
fn test_round_trip_serialization() {
    // Create a complex hook output
    let original = SyncHookJSONOutput {
        continue_: Some(false),
        suppress_output: Some(true),
        stop_reason: Some("Blocked".to_string()),
        decision: Some("block".to_string()),
        system_message: Some("Operation blocked".to_string()),
        reason: Some("Security policy".to_string()),
        hook_specific_output: Some(HookSpecificOutput::UserPromptSubmit(
            UserPromptSubmitHookSpecificOutput {
                additional_context: Some("Extra context".to_string()),
            },
        )),
    };

    // Serialize to JSON
    let json = serde_json::to_value(&original).expect("Failed to serialize");

    // Deserialize back
    let deserialized: SyncHookJSONOutput =
        serde_json::from_value(json).expect("Failed to deserialize");

    // Verify round-trip
    assert_eq!(deserialized.continue_, Some(false));
    assert_eq!(deserialized.suppress_output, Some(true));
    assert_eq!(deserialized.stop_reason, Some("Blocked".to_string()));
    assert_eq!(deserialized.decision, Some("block".to_string()));
    assert!(deserialized.hook_specific_output.is_some());
}
