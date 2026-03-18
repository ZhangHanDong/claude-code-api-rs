use cc_sdk::{SessionInfo, SessionMessage};
use serde_json::json;

#[test]
fn test_session_info_full_deserialize() {
    let json = json!({
        "session_id": "sess-full-123",
        "summary": "A complete session",
        "last_modified": 1710000000000_i64,
        "file_size": 8192,
        "custom_title": "My Important Session",
        "first_prompt": "Write a function",
        "git_branch": "feature/new-api",
        "cwd": "/home/user/project"
    });

    let info: SessionInfo = serde_json::from_value(json).unwrap();
    assert_eq!(info.session_id, "sess-full-123");
    assert_eq!(info.summary, "A complete session");
    assert_eq!(info.last_modified, 1710000000000);
    assert_eq!(info.file_size, 8192);
    assert_eq!(info.custom_title.as_deref(), Some("My Important Session"));
    assert_eq!(info.first_prompt.as_deref(), Some("Write a function"));
    assert_eq!(info.git_branch.as_deref(), Some("feature/new-api"));
    assert_eq!(info.cwd.as_deref(), Some("/home/user/project"));
}

#[test]
fn test_session_info_minimal_deserialize() {
    let json = json!({
        "session_id": "sess-minimal"
    });

    let info: SessionInfo = serde_json::from_value(json).unwrap();
    assert_eq!(info.session_id, "sess-minimal");
    assert_eq!(info.summary, "");
    assert_eq!(info.last_modified, 0);
    assert_eq!(info.file_size, 0);
    assert!(info.custom_title.is_none());
    assert!(info.first_prompt.is_none());
    assert!(info.git_branch.is_none());
    assert!(info.cwd.is_none());
}

#[test]
fn test_session_message_deserialize() {
    let json = json!({
        "type": "assistant",
        "uuid": "msg-uuid-456",
        "session_id": "sess-789",
        "message": {
            "role": "assistant",
            "content": [{"type": "text", "text": "Hello!"}]
        }
    });

    let msg: SessionMessage = serde_json::from_value(json).unwrap();
    assert_eq!(msg.msg_type, "assistant");
    assert_eq!(msg.uuid, "msg-uuid-456");
    assert_eq!(msg.session_id, "sess-789");
    assert_eq!(msg.message["role"], "assistant");
    assert_eq!(msg.message["content"][0]["text"], "Hello!");
}

#[test]
fn test_session_info_serialize_roundtrip() {
    let info = SessionInfo {
        session_id: "sess-roundtrip".to_string(),
        summary: "Roundtrip test".to_string(),
        last_modified: 1710000000000,
        file_size: 4096,
        custom_title: Some("Titled".to_string()),
        first_prompt: Some("Hello".to_string()),
        git_branch: Some("main".to_string()),
        cwd: Some("/tmp".to_string()),
    };

    let serialized = serde_json::to_string(&info).unwrap();
    let deserialized: SessionInfo = serde_json::from_str(&serialized).unwrap();

    assert_eq!(deserialized.session_id, info.session_id);
    assert_eq!(deserialized.summary, info.summary);
    assert_eq!(deserialized.last_modified, info.last_modified);
    assert_eq!(deserialized.file_size, info.file_size);
    assert_eq!(deserialized.custom_title, info.custom_title);
    assert_eq!(deserialized.first_prompt, info.first_prompt);
    assert_eq!(deserialized.git_branch, info.git_branch);
    assert_eq!(deserialized.cwd, info.cwd);
}

#[test]
fn test_session_info_optional_fields_none() {
    let info = SessionInfo {
        session_id: "sess-none".to_string(),
        summary: String::new(),
        last_modified: 0,
        file_size: 0,
        custom_title: None,
        first_prompt: None,
        git_branch: None,
        cwd: None,
    };

    let serialized = serde_json::to_string(&info).unwrap();
    // None fields with skip_serializing_if should not appear
    assert!(!serialized.contains("custom_title"));
    assert!(!serialized.contains("first_prompt"));
    assert!(!serialized.contains("git_branch"));
    assert!(!serialized.contains("cwd"));
}

#[test]
fn test_session_message_default_uuid() {
    let json = json!({
        "type": "system",
        "message": {"content": "system message"}
    });

    let msg: SessionMessage = serde_json::from_value(json).unwrap();
    assert_eq!(msg.msg_type, "system");
    assert_eq!(msg.uuid, "");
    assert_eq!(msg.session_id, "");
}

#[test]
fn test_session_info_with_worktree_fields() {
    let json = json!({
        "session_id": "sess-wt-001",
        "summary": "Worktree session",
        "last_modified": 1710000000000_i64,
        "file_size": 2048,
        "cwd": "/home/user/project/.claude/worktrees/feature-branch",
        "git_branch": "feature-branch"
    });

    let info: SessionInfo = serde_json::from_value(json).unwrap();
    assert_eq!(info.session_id, "sess-wt-001");
    assert!(info.cwd.as_deref().unwrap().contains("worktrees"));
    assert_eq!(info.git_branch.as_deref(), Some("feature-branch"));
    assert!(info.custom_title.is_none());
}

#[test]
fn test_session_message_various_types() {
    for msg_type in &["user", "assistant", "system", "result"] {
        let json = json!({
            "type": msg_type,
            "uuid": format!("uuid-{msg_type}"),
            "session_id": "sess-types",
            "message": {"content": format!("{msg_type} message")}
        });

        let msg: SessionMessage = serde_json::from_value(json).unwrap();
        assert_eq!(msg.msg_type, *msg_type);
        assert_eq!(msg.uuid, format!("uuid-{msg_type}"));
    }
}

#[test]
fn test_session_info_large_file_size() {
    let json = json!({
        "session_id": "sess-large",
        "file_size": u64::MAX
    });

    let info: SessionInfo = serde_json::from_value(json).unwrap();
    assert_eq!(info.file_size, u64::MAX);
}
