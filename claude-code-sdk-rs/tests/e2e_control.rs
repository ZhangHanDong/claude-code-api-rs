use nexus_claude::{
    CanUseTool, PermissionResult, PermissionResultAllow, Query, ToolPermissionContext,
    transport::mock::MockTransport,
};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;

#[tokio::test]
async fn e2e_initialize_control_handshake() {
    let (transport, handle) = MockTransport::pair();
    let transport = Arc::new(Mutex::new(transport));

    let mut q = Query::new(transport.clone(), false, None, None, HashMap::new());
    q.start().await.unwrap();

    // Responder: wait for SDK control request and reply with control_response
    let mut req_rx = handle.outbound_control_request_rx;
    let sdk_tx = handle.sdk_control_tx.clone();
    let responder = tokio::spawn(async move {
        if let Some(req) = req_rx.recv().await {
            // Expect a control_request with request_id
            let req_id = req
                .get("request_id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let resp = serde_json::json!({
                "type": "control_response",
                "response": {
                    "request_id": req_id,
                    "subtype": "success",
                    "response": {"ok": true}
                }
            });
            let _ = sdk_tx.send(resp).await;
        }
    });

    // Initialize and ensure result recorded
    q.initialize().await.unwrap();
    responder.await.unwrap();

    let init = q.get_initialization_result().cloned().unwrap();
    assert_eq!(init.get("ok").and_then(|v| v.as_bool()), Some(true));
}

struct AllowAll;
#[async_trait::async_trait]
impl CanUseTool for AllowAll {
    async fn can_use_tool(
        &self,
        _tool_name: &str,
        _input: &serde_json::Value,
        _context: &ToolPermissionContext,
    ) -> PermissionResult {
        PermissionResult::Allow(PermissionResultAllow {
            updated_input: Some(serde_json::json!({"patched": true})),
            updated_permissions: None,
        })
    }
}

#[tokio::test]
async fn e2e_permission_callback_response_shape() {
    let (transport, mut handle) = MockTransport::pair();
    let transport = Arc::new(Mutex::new(transport));

    let mut q = Query::new(
        transport.clone(),
        false,
        Some(Arc::new(AllowAll)),
        None,
        HashMap::new(),
    );
    q.start().await.unwrap();

    // Simulate CLI -> SDK permission control request
    let req_id = "perm_req_123";
    let control = serde_json::json!({
        "type": "control_request",
        "request_id": req_id,
        "request": {
            "subtype": "can_use_tool",
            "tool_name": "url_preview",
            "input": {"url": "https://example.com"}
        }
    });
    handle.sdk_control_tx.send(control).await.unwrap();

    // Observe SDK -> CLI response
    let resp = handle.outbound_control_rx.recv().await.unwrap();
    let envelope = resp
        .get("response")
        .cloned()
        .unwrap_or(serde_json::json!({}));
    let payload = envelope
        .get("response")
        .cloned()
        .unwrap_or(serde_json::json!({}));
    assert_eq!(payload.get("allow").and_then(|v| v.as_bool()), Some(true));
    assert_eq!(
        payload
            .get("input")
            .and_then(|v| v.get("patched"))
            .and_then(|v| v.as_bool()),
        Some(true)
    );

    // Permission suggestions snake_case should be parsed when present
    struct CaptureSuggestions(Arc<tokio::sync::Mutex<usize>>);
    #[async_trait::async_trait]
    impl CanUseTool for CaptureSuggestions {
        async fn can_use_tool(
            &self,
            _tool_name: &str,
            _input: &serde_json::Value,
            context: &ToolPermissionContext,
        ) -> PermissionResult {
            let len = context.suggestions.len();
            *self.0.lock().await = len;
            PermissionResult::Allow(PermissionResultAllow {
                updated_input: None,
                updated_permissions: None,
            })
        }
    }

    let (transport2, mut handle2) = MockTransport::pair();
    let transport2 = Arc::new(Mutex::new(transport2));
    let count = Arc::new(tokio::sync::Mutex::new(0usize));
    let mut q2 = Query::new(
        transport2.clone(),
        false,
        Some(Arc::new(CaptureSuggestions(count.clone()))),
        None,
        HashMap::new(),
    );
    q2.start().await.unwrap();
    let control2 = serde_json::json!({
        "type": "control_request",
        "request_id": "perm_req_456",
        "request": {
            "subtype": "can_use_tool",
            "tool_name": "url_preview",
            "input": {"url": "https://example.com"},
            "permission_suggestions": [
                {"type": "setMode", "mode": "acceptEdits", "destination": "session"}
            ]
        }
    });
    handle2.sdk_control_tx.send(control2).await.unwrap();
    let _ = handle2.outbound_control_rx.recv().await.unwrap();
    let seen = *count.lock().await;
    assert_eq!(seen, 1);
}

#[tokio::test]
async fn e2e_stream_input_converts_json_variants() {
    let (transport, mut handle) = MockTransport::pair();
    let transport = Arc::new(Mutex::new(transport));

    let mut q = Query::new(transport.clone(), true, None, None, HashMap::new());
    q.start().await.unwrap();

    // Build a simple stream of JSON inputs
    let inputs = vec![
        serde_json::json!("Hello"),
        serde_json::json!({"content": "Ping", "session_id": "s1"}),
    ];
    let stream = futures::stream::iter(inputs);

    q.stream_input(stream).await.unwrap();

    // Expect two InputMessage items sent through transport
    let first = handle.sent_input_rx.recv().await.unwrap();
    assert_eq!(
        first.message.get("content").and_then(|v| v.as_str()),
        Some("Hello")
    );

    let second = handle.sent_input_rx.recv().await.unwrap();
    assert_eq!(second.session_id, "s1");
    assert_eq!(
        second.message.get("content").and_then(|v| v.as_str()),
        Some("Ping")
    );
}
