use cc_sdk::{Query, transport::mock::MockTransport};
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::test]
async fn e2e_set_permission_mode_sends_control_request() {
    let (transport, mut handle) = MockTransport::pair();
    let transport = Arc::new(Mutex::new(transport));

    let mut q = Query::new(transport.clone(), true, None, None, std::collections::HashMap::new());
    q.start().await.unwrap();

    // Change permission mode
    q.set_permission_mode("acceptEdits").await.unwrap();

    // Validate outbound control_request
    let req = handle.outbound_control_request_rx.recv().await.unwrap();
    assert_eq!(req.get("type").and_then(|v| v.as_str()), Some("control_request"));
    let inner = req.get("request").cloned().unwrap_or(serde_json::json!({}));
    assert_eq!(inner.get("type").and_then(|v| v.as_str()), Some("set_permission_mode"));
    assert_eq!(inner.get("mode").and_then(|v| v.as_str()), Some("acceptEdits"));
}

