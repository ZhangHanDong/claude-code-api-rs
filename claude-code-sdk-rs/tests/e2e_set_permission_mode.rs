use nexus_claude::{Query, transport::mock::MockTransport};
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::test]
async fn e2e_set_permission_mode_sends_control_request() {
    let (transport, mut handle) = MockTransport::pair();
    let transport = Arc::new(Mutex::new(transport));

    let mut q = Query::new(
        transport.clone(),
        true,
        None,
        None,
        std::collections::HashMap::new(),
    );
    q.start().await.unwrap();

    // Spawn a task to mock the CLI response
    let sdk_control_tx = handle.sdk_control_tx.clone();
    let responder = tokio::spawn(async move {
        // Wait for the outbound control request
        let req = handle.outbound_control_request_rx.recv().await.unwrap();

        // Extract request_id and send back a success response
        let request_id = req
            .get("request_id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        let response = serde_json::json!({
            "type": "control_response",
            "response": {
                "request_id": request_id,
                "subtype": "success",
                "response": {}
            }
        });

        sdk_control_tx.send(response).await.unwrap();
        req
    });

    // Change permission mode - it will now receive the mocked response
    let set_mode = q.set_permission_mode("acceptEdits");

    // Wait for both the responder and set_permission_mode to complete
    let (req, result) = tokio::join!(responder, set_mode);
    let req = req.unwrap();
    result.unwrap();

    // Validate outbound control_request
    assert_eq!(
        req.get("type").and_then(|v| v.as_str()),
        Some("control_request")
    );
    let inner = req.get("request").cloned().unwrap_or(serde_json::json!({}));
    assert_eq!(
        inner.get("type").and_then(|v| v.as_str()),
        Some("set_permission_mode")
    );
    assert_eq!(
        inner.get("mode").and_then(|v| v.as_str()),
        Some("acceptEdits")
    );
}
