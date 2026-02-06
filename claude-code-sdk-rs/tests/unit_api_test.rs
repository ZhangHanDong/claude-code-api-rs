//! Unit tests for API components (no Claude CLI required)

use nexus_claude::{
    ClaudeCodeOptions, ClientMode, PerformanceMetrics, PermissionMode, RetryConfig,
};
use std::time::Duration;

/// Test ClientMode variants
#[test]
fn test_client_modes() {
    // Test OneShot mode
    let oneshot = ClientMode::OneShot;
    match oneshot {
        ClientMode::OneShot => (),
        _ => panic!("Expected OneShot mode"),
    }

    // Test Interactive mode
    let interactive = ClientMode::Interactive;
    match interactive {
        ClientMode::Interactive => (),
        _ => panic!("Expected Interactive mode"),
    }

    // Test Batch mode
    let batch = ClientMode::Batch { max_concurrent: 5 };
    match batch {
        ClientMode::Batch { max_concurrent } => {
            assert_eq!(max_concurrent, 5);
        },
        _ => panic!("Expected Batch mode"),
    }
}

/// Test RetryConfig
#[test]
fn test_retry_config() {
    // Test default config
    let config = RetryConfig::default();
    assert_eq!(config.max_retries, 3);
    assert_eq!(config.initial_delay, Duration::from_millis(100));
    assert_eq!(config.max_delay, Duration::from_secs(30));
    assert_eq!(config.backoff_multiplier, 2.0);
    assert_eq!(config.jitter_factor, 0.1);

    // Test custom config
    let custom = RetryConfig {
        max_retries: 5,
        initial_delay: Duration::from_millis(200),
        max_delay: Duration::from_secs(60),
        backoff_multiplier: 1.5,
        jitter_factor: 0.2,
    };
    assert_eq!(custom.max_retries, 5);
    assert_eq!(custom.initial_delay, Duration::from_millis(200));
    assert_eq!(custom.backoff_multiplier, 1.5);
}

/// Test PerformanceMetrics
#[test]
fn test_performance_metrics() {
    let mut metrics = PerformanceMetrics::default();

    // Initial state
    assert_eq!(metrics.total_requests, 0);
    assert_eq!(metrics.successful_requests, 0);
    assert_eq!(metrics.failed_requests, 0);

    // Record some operations
    metrics.record_success(100);
    metrics.record_success(200);
    metrics.record_success(150);

    assert_eq!(metrics.total_requests, 3);
    assert_eq!(metrics.successful_requests, 3);
    assert_eq!(metrics.average_latency_ms(), 150.0);
    assert_eq!(metrics.min_latency_ms, 100);
    assert_eq!(metrics.max_latency_ms, 200);

    // Record failures
    metrics.record_failure();
    metrics.record_failure();

    assert_eq!(metrics.total_requests, 5);
    assert_eq!(metrics.failed_requests, 2);
    assert_eq!(metrics.success_rate(), 0.6);
}

/// Test ClaudeCodeOptions builder
#[test]
#[allow(deprecated)]
fn test_options_builder() {
    // Test minimal options
    let minimal = ClaudeCodeOptions::builder().build();
    assert_eq!(minimal.permission_mode, PermissionMode::Default);
    assert!(minimal.model.is_none());
    assert!(minimal.system_prompt.is_none());

    // Test full options
    let full = ClaudeCodeOptions::builder()
        .permission_mode(PermissionMode::AcceptEdits)
        .model("claude-3-opus")
        .system_prompt("Test prompt")
        .append_system_prompt("Additional prompt")
        .allow_tool("Bash")
        .allow_tool("Read")
        .disallow_tool("Write")
        .permission_prompt_tool_name("custom_prompt")
        .build();

    assert_eq!(full.permission_mode, PermissionMode::AcceptEdits);
    assert_eq!(full.model, Some("claude-3-opus".to_string()));
    assert_eq!(full.system_prompt, Some("Test prompt".to_string()));
    assert_eq!(full.allowed_tools, vec!["Bash", "Read"]);
    assert_eq!(full.disallowed_tools, vec!["Write"]);
    assert_eq!(
        full.permission_prompt_tool_name,
        Some("custom_prompt".to_string())
    );
}

/// Test PermissionMode serialization
#[test]
fn test_permission_mode_serialization() {
    use serde_json;

    // Test serialization
    let mode = PermissionMode::AcceptEdits;
    let json = serde_json::to_string(&mode).unwrap();
    assert_eq!(json, "\"acceptEdits\"");

    // Test deserialization
    let deserialized: PermissionMode = serde_json::from_str("\"bypassPermissions\"").unwrap();
    assert_eq!(deserialized, PermissionMode::BypassPermissions);
}

/// Test metrics edge cases
#[test]
fn test_metrics_edge_cases() {
    let mut metrics = PerformanceMetrics::default();

    // Test with no data
    assert_eq!(metrics.average_latency_ms(), 0.0);
    assert_eq!(metrics.success_rate(), 0.0);

    // Test with only failures
    metrics.record_failure();
    metrics.record_failure();
    assert_eq!(metrics.average_latency_ms(), 0.0);
    assert_eq!(metrics.success_rate(), 0.0);

    // Test with single success
    let mut single = PerformanceMetrics::default();
    single.record_success(500);
    assert_eq!(single.average_latency_ms(), 500.0);
    assert_eq!(single.success_rate(), 1.0);
    assert_eq!(single.min_latency_ms, 500);
    assert_eq!(single.max_latency_ms, 500);
}
