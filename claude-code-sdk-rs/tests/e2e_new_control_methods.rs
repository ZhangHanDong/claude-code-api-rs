//! E2E tests for new control protocol methods added in P3 SDK parity:
//! get_context_usage, stop_task, get_mcp_status, reconnect_mcp_server, toggle_mcp_server

use cc_sdk::{Query, transport::mock::MockTransport};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Creates a Query with mock transport, starts it, and returns (Query, MockTransportHandle)
async fn setup_query() -> (Query, cc_sdk::transport::mock::MockTransportHandle) {
    let (transport, handle) = MockTransport::pair();
    let transport = Arc::new(Mutex::new(transport));
    let mut q = Query::new(transport.clone(), true, None, None, std::collections::HashMap::new());
    q.start().await.unwrap();
    (q, handle)
}

#[tokio::test]
async fn e2e_get_context_usage_sends_control_request() {
    let (mut q, mut handle) = setup_query().await;

    let sdk_tx = handle.sdk_control_tx.clone();
    let responder = tokio::spawn(async move {
        let req = handle.outbound_control_request_rx.recv().await.unwrap();
        let request_id = req.get("request_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let resp = serde_json::json!({
            "type": "control_response",
            "response": {
                "request_id": request_id,
                "subtype": "success",
                "response": {
                    "totalTokens": 5000,
                    "maxTokens": 200000,
                    "percentage": 2.5,
                    "model": "claude-sonnet-4-20250514",
                    "categories": [],
                    "isAutoCompactEnabled": false,
                    "memoryFiles": [],
                    "mcpTools": []
                }
            }
        });
        sdk_tx.send(resp).await.unwrap();
        req
    });

    let (req, result) = tokio::join!(responder, q.get_context_usage());
    let req = req.unwrap();
    let resp = result.unwrap();

    // Verify outbound request shape
    assert_eq!(req.get("type").and_then(|v| v.as_str()), Some("control_request"));
    assert_eq!(
        req.get("request").and_then(|r| r.get("type")).and_then(|v| v.as_str()),
        Some("get_context_usage")
    );

    // Verify response was returned
    assert_eq!(resp.get("totalTokens").and_then(|v| v.as_u64()), Some(5000));
    assert_eq!(resp.get("model").and_then(|v| v.as_str()), Some("claude-sonnet-4-20250514"));
}

#[tokio::test]
async fn e2e_stop_task_sends_control_request() {
    let (mut q, mut handle) = setup_query().await;

    let sdk_tx = handle.sdk_control_tx.clone();
    let responder = tokio::spawn(async move {
        let req = handle.outbound_control_request_rx.recv().await.unwrap();
        let request_id = req.get("request_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let resp = serde_json::json!({
            "type": "control_response",
            "response": {
                "request_id": request_id,
                "subtype": "success",
                "response": {}
            }
        });
        sdk_tx.send(resp).await.unwrap();
        req
    });

    let (req, result) = tokio::join!(responder, q.stop_task("task-123"));
    let req = req.unwrap();
    result.unwrap();

    assert_eq!(
        req.get("request").and_then(|r| r.get("type")).and_then(|v| v.as_str()),
        Some("stop_task")
    );
    assert_eq!(
        req.get("request").and_then(|r| r.get("task_id")).and_then(|v| v.as_str()),
        Some("task-123")
    );
}

#[tokio::test]
async fn e2e_get_mcp_status_sends_control_request() {
    let (mut q, mut handle) = setup_query().await;

    let sdk_tx = handle.sdk_control_tx.clone();
    let responder = tokio::spawn(async move {
        let req = handle.outbound_control_request_rx.recv().await.unwrap();
        let request_id = req.get("request_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let resp = serde_json::json!({
            "type": "control_response",
            "response": {
                "request_id": request_id,
                "subtype": "success",
                "response": {
                    "servers": [
                        {"name": "my-server", "status": "connected"}
                    ]
                }
            }
        });
        sdk_tx.send(resp).await.unwrap();
        req
    });

    let (req, result) = tokio::join!(responder, q.get_mcp_status());
    let req = req.unwrap();
    let resp = result.unwrap();

    assert_eq!(
        req.get("request").and_then(|r| r.get("type")).and_then(|v| v.as_str()),
        Some("mcp_status")
    );
    assert_eq!(
        resp.get("servers").and_then(|s| s.as_array()).map(|a| a.len()),
        Some(1)
    );
}

#[tokio::test]
async fn e2e_reconnect_mcp_server_sends_control_request() {
    let (mut q, mut handle) = setup_query().await;

    let sdk_tx = handle.sdk_control_tx.clone();
    let responder = tokio::spawn(async move {
        let req = handle.outbound_control_request_rx.recv().await.unwrap();
        let request_id = req.get("request_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let resp = serde_json::json!({
            "type": "control_response",
            "response": {
                "request_id": request_id,
                "subtype": "success",
                "response": {}
            }
        });
        sdk_tx.send(resp).await.unwrap();
        req
    });

    let (req, result) = tokio::join!(responder, q.reconnect_mcp_server("test-srv"));
    let req = req.unwrap();
    result.unwrap();

    assert_eq!(
        req.get("request").and_then(|r| r.get("type")).and_then(|v| v.as_str()),
        Some("mcp_reconnect")
    );
    assert_eq!(
        req.get("request").and_then(|r| r.get("server_name")).and_then(|v| v.as_str()),
        Some("test-srv")
    );
}

#[tokio::test]
async fn e2e_toggle_mcp_server_sends_control_request() {
    let (mut q, mut handle) = setup_query().await;

    let sdk_tx = handle.sdk_control_tx.clone();
    let responder = tokio::spawn(async move {
        let req = handle.outbound_control_request_rx.recv().await.unwrap();
        let request_id = req.get("request_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let resp = serde_json::json!({
            "type": "control_response",
            "response": {
                "request_id": request_id,
                "subtype": "success",
                "response": {}
            }
        });
        sdk_tx.send(resp).await.unwrap();
        req
    });

    let (req, result) = tokio::join!(responder, q.toggle_mcp_server("test-srv", false));
    let req = req.unwrap();
    result.unwrap();

    assert_eq!(
        req.get("request").and_then(|r| r.get("type")).and_then(|v| v.as_str()),
        Some("mcp_toggle")
    );
    assert_eq!(
        req.get("request").and_then(|r| r.get("server_name")).and_then(|v| v.as_str()),
        Some("test-srv")
    );
    assert_eq!(
        req.get("request").and_then(|r| r.get("enabled")).and_then(|v| v.as_bool()),
        Some(false)
    );
}
