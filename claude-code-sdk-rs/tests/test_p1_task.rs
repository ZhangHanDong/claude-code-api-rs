use cc_sdk::{
    Message, TaskStatus, TaskUsage, AgentDefinition,
};
use serde_json::json;

#[test]
fn test_task_started_from_system_message() {
    let msg = Message::System {
        subtype: "task_started".to_string(),
        data: json!({
            "task_id": "task-001",
            "description": "Running code analysis",
            "uuid": "uuid-123",
            "session_id": "sess-456",
            "tool_use_id": "tool-789"
        }),
    };
    let started = msg.as_task_started().expect("should parse task_started");
    assert_eq!(started.task_id, "task-001");
    assert_eq!(started.description, "Running code analysis");
    assert_eq!(started.uuid, "uuid-123");
    assert_eq!(started.session_id, "sess-456");
    assert_eq!(started.tool_use_id, Some("tool-789".to_string()));
    assert_eq!(started.task_type, None);
}

#[test]
fn test_task_progress_from_system_message() {
    let msg = Message::System {
        subtype: "task_progress".to_string(),
        data: json!({
            "task_id": "task-001",
            "description": "Analyzing files...",
            "usage": {
                "total_tokens": 1500,
                "tool_uses": 3,
                "duration_ms": 5000
            },
            "uuid": "uuid-234",
            "session_id": "sess-456"
        }),
    };
    let progress = msg.as_task_progress().expect("should parse task_progress");
    assert_eq!(progress.task_id, "task-001");
    assert_eq!(progress.description, "Analyzing files...");
    assert_eq!(progress.usage.total_tokens, 1500);
    assert_eq!(progress.usage.tool_uses, 3);
    assert_eq!(progress.usage.duration_ms, 5000);
    assert_eq!(progress.uuid, "uuid-234");
    assert_eq!(progress.last_tool_name, None);
}

#[test]
fn test_task_notification_from_system_message() {
    let msg = Message::System {
        subtype: "task_notification".to_string(),
        data: json!({
            "task_id": "task-001",
            "status": "completed",
            "summary": "Analysis complete",
            "uuid": "uuid-345",
            "session_id": "sess-456"
        }),
    };
    let notif = msg.as_task_notification().expect("should parse task_notification");
    assert_eq!(notif.task_id, "task-001");
    assert_eq!(notif.status, TaskStatus::Completed);
    assert_eq!(notif.summary, Some("Analysis complete".to_string()));
    assert_eq!(notif.output_file, None);
}

#[test]
fn test_task_started_wrong_subtype() {
    let msg = Message::System {
        subtype: "task_progress".to_string(),
        data: json!({
            "task_id": "task-001",
            "description": "test",
            "uuid": "uuid-123",
            "session_id": "sess-456"
        }),
    };
    assert!(msg.as_task_started().is_none());

    // Also test non-System variant
    let user_msg = Message::User {
        message: cc_sdk::UserMessage { content: "hello".to_string() },
    };
    assert!(user_msg.as_task_started().is_none());
}

#[test]
fn test_task_status_serde() {
    let completed: TaskStatus = serde_json::from_str(r#""completed""#).unwrap();
    assert_eq!(completed, TaskStatus::Completed);

    let failed: TaskStatus = serde_json::from_str(r#""failed""#).unwrap();
    assert_eq!(failed, TaskStatus::Failed);

    let stopped: TaskStatus = serde_json::from_str(r#""stopped""#).unwrap();
    assert_eq!(stopped, TaskStatus::Stopped);

    // Round-trip
    let serialized = serde_json::to_string(&TaskStatus::Completed).unwrap();
    assert_eq!(serialized, r#""completed""#);
}

#[test]
fn test_task_usage_default() {
    let usage = TaskUsage::default();
    assert_eq!(usage.total_tokens, 0);
    assert_eq!(usage.tool_uses, 0);
    assert_eq!(usage.duration_ms, 0);
}

#[test]
fn test_task_notification_with_usage() {
    let msg = Message::System {
        subtype: "task_notification".to_string(),
        data: json!({
            "task_id": "task-002",
            "status": "failed",
            "output_file": "/tmp/output.txt",
            "summary": "Task failed due to timeout",
            "uuid": "uuid-567",
            "session_id": "sess-789",
            "tool_use_id": "tool-111",
            "usage": {
                "total_tokens": 5000,
                "tool_uses": 10,
                "duration_ms": 30000
            }
        }),
    };
    let notif = msg.as_task_notification().expect("should parse");
    assert_eq!(notif.status, TaskStatus::Failed);
    assert_eq!(notif.output_file, Some("/tmp/output.txt".to_string()));
    assert_eq!(notif.tool_use_id, Some("tool-111".to_string()));
    let usage = notif.usage.expect("should have usage");
    assert_eq!(usage.total_tokens, 5000);
    assert_eq!(usage.tool_uses, 10);
    assert_eq!(usage.duration_ms, 30000);
}

#[test]
fn test_agent_definition_new_fields() {
    let agent = AgentDefinition {
        description: "Test agent".to_string(),
        prompt: "You are a test agent".to_string(),
        tools: Some(vec!["Read".to_string()]),
        model: Some("claude-sonnet-4-20250514".to_string()),
        skills: Some(vec!["coding".to_string(), "analysis".to_string()]),
        memory: Some("persistent".to_string()),
        mcp_servers: Some(vec![json!({"name": "test-server", "url": "http://localhost:3000"})]),
    };

    let json_str = serde_json::to_string(&agent).unwrap();
    let parsed: AgentDefinition = serde_json::from_str(&json_str).unwrap();

    assert_eq!(parsed.skills, Some(vec!["coding".to_string(), "analysis".to_string()]));
    assert_eq!(parsed.memory, Some("persistent".to_string()));
    assert_eq!(parsed.mcp_servers.as_ref().unwrap().len(), 1);

    // Verify mcpServers rename
    let value: serde_json::Value = serde_json::to_value(&agent).unwrap();
    assert!(value.get("mcpServers").is_some());
    assert!(value.get("mcp_servers").is_none());
}

#[test]
fn test_agent_definition_backward_compat() {
    let old_json = json!({
        "description": "Old agent",
        "prompt": "You are an old agent"
    });
    let agent: AgentDefinition = serde_json::from_value(old_json).unwrap();
    assert_eq!(agent.description, "Old agent");
    assert_eq!(agent.prompt, "You are an old agent");
    assert!(agent.tools.is_none());
    assert!(agent.model.is_none());
    assert!(agent.skills.is_none());
    assert!(agent.memory.is_none());
    assert!(agent.mcp_servers.is_none());
}

#[test]
fn test_task_progress_last_tool_name() {
    let msg = Message::System {
        subtype: "task_progress".to_string(),
        data: json!({
            "task_id": "task-003",
            "description": "Using Bash tool",
            "usage": {
                "total_tokens": 800,
                "tool_uses": 1,
                "duration_ms": 2000
            },
            "uuid": "uuid-890",
            "session_id": "sess-012",
            "last_tool_name": "Bash"
        }),
    };
    let progress = msg.as_task_progress().expect("should parse");
    assert_eq!(progress.last_tool_name, Some("Bash".to_string()));
}
