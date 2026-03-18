//! Phase 3 tests — MCP status types, ThinkingConfig, and integration

use cc_sdk::{
    ClaudeCodeOptions, McpConnectionStatus, McpServerInfo, McpServerStatus, McpToolAnnotations,
    McpToolInfo, ThinkingConfig,
};
use serde_json::json;

// ─── MCP Status Types ─────────────────────────────────────────────────────

#[test]
fn test_mcp_connection_status_serde() {
    let statuses = vec![
        (McpConnectionStatus::Connected, "\"connected\""),
        (McpConnectionStatus::Failed, "\"failed\""),
        (McpConnectionStatus::NeedsAuth, "\"needs-auth\""),
        (McpConnectionStatus::Pending, "\"pending\""),
        (McpConnectionStatus::Disabled, "\"disabled\""),
    ];

    for (variant, expected) in statuses {
        let serialized = serde_json::to_string(&variant).unwrap();
        assert_eq!(serialized, expected, "Failed for {:?}", variant);
        let deserialized: McpConnectionStatus = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, variant);
    }
}

#[test]
fn test_mcp_server_status_full() {
    let json = json!({
        "name": "my-server",
        "status": "connected",
        "server_info": {
            "name": "test-mcp",
            "version": "1.0.0"
        },
        "tools": [
            {
                "name": "read_file",
                "description": "Read a file",
                "annotations": {
                    "read_only": true,
                    "destructive": false
                }
            }
        ]
    });

    let status: McpServerStatus = serde_json::from_value(json).unwrap();
    assert_eq!(status.name, "my-server");
    assert_eq!(status.status, McpConnectionStatus::Connected);

    let info = status.server_info.unwrap();
    assert_eq!(info.name, "test-mcp");
    assert_eq!(info.version, "1.0.0");

    let tools = status.tools.unwrap();
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].name, "read_file");
    let annotations = tools[0].annotations.as_ref().unwrap();
    assert_eq!(annotations.read_only, Some(true));
    assert_eq!(annotations.destructive, Some(false));
    assert!(annotations.open_world.is_none());
}

#[test]
fn test_mcp_server_status_failed() {
    let json = json!({
        "name": "broken-server",
        "status": "failed",
        "error": "Connection refused"
    });

    let status: McpServerStatus = serde_json::from_value(json).unwrap();
    assert_eq!(status.status, McpConnectionStatus::Failed);
    assert_eq!(status.error.as_deref(), Some("Connection refused"));
    assert!(status.server_info.is_none());
    assert!(status.tools.is_none());
}

#[test]
fn test_mcp_tool_info_minimal() {
    let json = json!({
        "name": "simple_tool"
    });

    let tool: McpToolInfo = serde_json::from_value(json).unwrap();
    assert_eq!(tool.name, "simple_tool");
    assert!(tool.description.is_none());
    assert!(tool.annotations.is_none());
}

#[test]
fn test_mcp_server_info_roundtrip() {
    let info = McpServerInfo {
        name: "test".to_string(),
        version: "2.1.0".to_string(),
    };
    let serialized = serde_json::to_string(&info).unwrap();
    let deserialized: McpServerInfo = serde_json::from_str(&serialized).unwrap();
    assert_eq!(deserialized.name, info.name);
    assert_eq!(deserialized.version, info.version);
}

// ─── ThinkingConfig ───────────────────────────────────────────────────────

#[test]
fn test_thinking_config_adaptive() {
    let config = ThinkingConfig::Adaptive;
    let serialized = serde_json::to_string(&config).unwrap();
    assert!(serialized.contains("\"type\":\"adaptive\""));

    let deserialized: ThinkingConfig = serde_json::from_str(&serialized).unwrap();
    assert_eq!(deserialized, ThinkingConfig::Adaptive);
}

#[test]
fn test_thinking_config_enabled() {
    let config = ThinkingConfig::Enabled {
        budget_tokens: 10000,
    };
    let serialized = serde_json::to_string(&config).unwrap();
    assert!(serialized.contains("\"type\":\"enabled\""));
    assert!(serialized.contains("\"budget_tokens\":10000"));

    let deserialized: ThinkingConfig = serde_json::from_str(&serialized).unwrap();
    assert_eq!(deserialized, config);
}

#[test]
fn test_thinking_config_disabled() {
    let config = ThinkingConfig::Disabled;
    let serialized = serde_json::to_string(&config).unwrap();
    let deserialized: ThinkingConfig = serde_json::from_str(&serialized).unwrap();
    assert_eq!(deserialized, ThinkingConfig::Disabled);
}

#[test]
fn test_thinking_config_builder() {
    let options = ClaudeCodeOptions::builder()
        .thinking(ThinkingConfig::Enabled {
            budget_tokens: 5000,
        })
        .build();
    assert_eq!(
        options.thinking,
        Some(ThinkingConfig::Enabled {
            budget_tokens: 5000
        })
    );
}

#[test]
fn test_thinking_config_default_none() {
    let options = ClaudeCodeOptions::default();
    assert!(options.thinking.is_none());
}

// ─── MCP Tool Annotations ────────────────────────────────────────────────

#[test]
fn test_mcp_tool_annotations_partial() {
    let json = json!({
        "read_only": true
    });

    let annotations: McpToolAnnotations = serde_json::from_value(json).unwrap();
    assert_eq!(annotations.read_only, Some(true));
    assert!(annotations.destructive.is_none());
    assert!(annotations.open_world.is_none());
}
