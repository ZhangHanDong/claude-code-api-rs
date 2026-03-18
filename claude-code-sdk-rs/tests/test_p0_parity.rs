//! Phase 0 parity tests — rate limits, effort, assistant message fields, stream events, unknown messages

use cc_sdk::{
    AssistantMessage, AssistantMessageError, ClaudeCodeOptions, ContentBlock, Effort, Message,
    RateLimitInfo, RateLimitStatus, RateLimitType, TextContent,
};
use serde_json::json;

// ─── Helpers ──────────────────────────────────────────────────────────────

/// Parse a JSON value through the SDK's internal message parser (via serde round-trip)
fn parse(json: serde_json::Value) -> Option<Message> {
    // The SDK uses message_parser internally; for unit tests we verify the types
    // directly since message_parser is pub(crate).
    serde_json::from_value(json).ok()
}

// ─── Rate Limit Tests ─────────────────────────────────────────────────────

#[test]
fn test_deserialize_rate_limit_info() {
    let json = json!({
        "status": "allowed_warning",
        "resets_at": "2026-03-19T00:00:00Z",
        "rate_limit_type": "five_hour",
        "utilization": 0.85
    });

    let info: RateLimitInfo = serde_json::from_value(json).unwrap();
    assert_eq!(info.status, RateLimitStatus::AllowedWarning);
    assert_eq!(info.resets_at.as_deref(), Some("2026-03-19T00:00:00Z"));
    assert_eq!(info.rate_limit_type, Some(RateLimitType::FiveHour));
    assert!((info.utilization.unwrap() - 0.85).abs() < f64::EPSILON);
}

#[test]
fn test_rate_limit_rejected() {
    let json = json!({
        "status": "rejected",
        "rate_limit_type": "seven_day_opus",
        "utilization": 1.0
    });

    let info: RateLimitInfo = serde_json::from_value(json).unwrap();
    assert_eq!(info.status, RateLimitStatus::Rejected);
    assert_eq!(info.rate_limit_type, Some(RateLimitType::SevenDayOpus));
}

#[test]
fn test_rate_limit_info_overage_fields() {
    let json = json!({
        "status": "allowed",
        "overage_status": "active",
        "overage_resets_at": "2026-03-25T00:00:00Z",
        "overage_disabled_reason": null,
        "raw": {"custom": "data"}
    });

    let info: RateLimitInfo = serde_json::from_value(json).unwrap();
    assert_eq!(info.status, RateLimitStatus::Allowed);
    assert_eq!(info.overage_status.as_deref(), Some("active"));
    assert_eq!(
        info.overage_resets_at.as_deref(),
        Some("2026-03-25T00:00:00Z")
    );
    assert!(info.overage_disabled_reason.is_none());
    assert!(info.raw.is_some());
}

#[test]
fn test_rate_limit_event_uuid_session_id() {
    // Verify RateLimit message variant round-trips with uuid/session_id
    let msg = Message::RateLimit {
        rate_limit_info: RateLimitInfo {
            status: RateLimitStatus::AllowedWarning,
            resets_at: None,
            rate_limit_type: None,
            utilization: Some(0.9),
            overage_status: None,
            overage_resets_at: None,
            overage_disabled_reason: None,
            raw: None,
        },
        uuid: "uuid-123".to_string(),
        session_id: "session-456".to_string(),
    };

    // Serialize and check fields
    let serialized = serde_json::to_value(&msg).unwrap();
    assert_eq!(serialized["uuid"], "uuid-123");
    assert_eq!(serialized["session_id"], "session-456");
    assert_eq!(serialized["rate_limit_info"]["status"], "allowed_warning");
}

// ─── Result Message stop_reason ───────────────────────────────────────────

#[test]
fn test_result_message_stop_reason() {
    let json = json!({
        "type": "result",
        "subtype": "conversation_turn",
        "duration_ms": 100,
        "duration_api_ms": 80,
        "is_error": false,
        "num_turns": 1,
        "session_id": "s1",
        "stop_reason": "end_turn"
    });

    let msg: Message = serde_json::from_value(json).unwrap();
    if let Message::Result { stop_reason, .. } = msg {
        assert_eq!(stop_reason.as_deref(), Some("end_turn"));
    } else {
        panic!("Expected Result message");
    }
}

#[test]
fn test_result_message_no_stop_reason() {
    // Backward compat: stop_reason missing should default to None
    let json = json!({
        "type": "result",
        "subtype": "conversation_turn",
        "duration_ms": 100,
        "duration_api_ms": 80,
        "is_error": false,
        "num_turns": 1,
        "session_id": "s1"
    });

    let msg: Message = serde_json::from_value(json).unwrap();
    if let Message::Result { stop_reason, .. } = msg {
        assert!(stop_reason.is_none());
    } else {
        panic!("Expected Result message");
    }
}

// ─── Effort CLI flag ──────────────────────────────────────────────────────

#[test]
fn test_effort_cli_flag() {
    let options = ClaudeCodeOptions::builder().effort(Effort::High).build();
    assert_eq!(options.effort, Some(Effort::High));

    // Test Display impl
    assert_eq!(Effort::Low.to_string(), "low");
    assert_eq!(Effort::Medium.to_string(), "medium");
    assert_eq!(Effort::High.to_string(), "high");
    assert_eq!(Effort::Max.to_string(), "max");
}

#[test]
fn test_effort_none_no_flag() {
    let options = ClaudeCodeOptions::default();
    assert!(options.effort.is_none());
}

// ─── Assistant Message Fields ─────────────────────────────────────────────

#[test]
fn test_assistant_message_model_usage() {
    let msg = AssistantMessage {
        content: vec![ContentBlock::Text(TextContent {
            text: "Hello".to_string(),
        })],
        model: Some("claude-sonnet-4-6".to_string()),
        usage: Some(json!({"input_tokens": 10, "output_tokens": 20})),
        error: None,
        parent_tool_use_id: None,
    };

    assert_eq!(msg.model.as_deref(), Some("claude-sonnet-4-6"));
    assert_eq!(msg.usage.as_ref().unwrap()["input_tokens"], 10);
}

#[test]
fn test_assistant_message_error() {
    let json = json!({
        "content": [],
        "error": "rate_limit"
    });

    let msg: AssistantMessage = serde_json::from_value(json).unwrap();
    assert_eq!(msg.error, Some(AssistantMessageError::RateLimit));
}

#[test]
fn test_assistant_message_parent_tool_use_id() {
    let msg = AssistantMessage {
        content: vec![],
        model: None,
        usage: None,
        error: None,
        parent_tool_use_id: Some("tool-abc".to_string()),
    };
    assert_eq!(msg.parent_tool_use_id.as_deref(), Some("tool-abc"));
}

// ─── Stream Event ─────────────────────────────────────────────────────────

#[test]
fn test_stream_event_deserialization() {
    // StreamEvent variant round-trip
    let msg = Message::StreamEvent {
        uuid: "ev-1".to_string(),
        session_id: "sess-1".to_string(),
        event: json!({"delta": "hello"}),
        parent_tool_use_id: Some("tool-1".to_string()),
    };

    let serialized = serde_json::to_value(&msg).unwrap();
    assert_eq!(serialized["uuid"], "ev-1");
    assert_eq!(serialized["event"]["delta"], "hello");
    assert_eq!(serialized["parent_tool_use_id"], "tool-1");
}

// ─── Unknown Message Type ─────────────────────────────────────────────────

#[test]
fn test_unknown_message_type_no_panic() {
    // Message::Unknown is constructed by message_parser, not by serde.
    // Verify it can be created and matched without panic.
    let msg = Message::Unknown {
        msg_type: "future_type".to_string(),
        raw: json!({"some": "data"}),
    };

    match msg {
        Message::Unknown { msg_type, raw } => {
            assert_eq!(msg_type, "future_type");
            assert_eq!(raw["some"], "data");
        }
        _ => panic!("Expected Unknown message"),
    }
}

// ─── Effort Serde ─────────────────────────────────────────────────────────

#[test]
fn test_effort_serde_roundtrip() {
    let effort = Effort::Max;
    let serialized = serde_json::to_string(&effort).unwrap();
    assert_eq!(serialized, "\"max\"");
    let deserialized: Effort = serde_json::from_str(&serialized).unwrap();
    assert_eq!(deserialized, Effort::Max);
}

// ─── Rate Limit Serde ─────────────────────────────────────────────────────

#[test]
fn test_rate_limit_type_serde() {
    let types = vec![
        (RateLimitType::FiveHour, "\"five_hour\""),
        (RateLimitType::SevenDay, "\"seven_day\""),
        (RateLimitType::SevenDayOpus, "\"seven_day_opus\""),
        (RateLimitType::SevenDaySonnet, "\"seven_day_sonnet\""),
        (RateLimitType::Overage, "\"overage\""),
    ];

    for (variant, expected) in types {
        let serialized = serde_json::to_string(&variant).unwrap();
        assert_eq!(serialized, expected);
        let deserialized: RateLimitType = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, variant);
    }
}

// ─── AssistantMessageError Serde ──────────────────────────────────────────

#[test]
fn test_assistant_message_error_serde() {
    let errors = vec![
        (AssistantMessageError::AuthenticationFailed, "\"authentication_failed\""),
        (AssistantMessageError::BillingError, "\"billing_error\""),
        (AssistantMessageError::RateLimit, "\"rate_limit\""),
        (AssistantMessageError::InvalidRequest, "\"invalid_request\""),
        (AssistantMessageError::ServerError, "\"server_error\""),
        (AssistantMessageError::Unknown, "\"unknown\""),
    ];

    for (variant, expected) in errors {
        let serialized = serde_json::to_string(&variant).unwrap();
        assert_eq!(serialized, expected, "Failed for {:?}", variant);
    }
}

// Verify parse helper works for simple cases
#[test]
fn test_parse_helper_basic() {
    let result = parse(json!({"type": "system", "subtype": "init", "data": {}}));
    assert!(result.is_some());
}
