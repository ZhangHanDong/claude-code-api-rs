use cc_sdk::{Query, transport::mock::MockTransport};
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::test]
async fn e2e_set_model_sends_control_request() {
    let (transport, mut handle) = MockTransport::pair();
    let transport = Arc::new(Mutex::new(transport));

    let mut q = Query::new(transport.clone(), true, None, None, std::collections::HashMap::new());
    q.start().await.unwrap();

    // Call set_model and assert an outbound control_request seen
    q.set_model(Some("sonnet".to_string())).await.unwrap();

    // The first outbound control request should be our set_model
    let req = handle.outbound_control_request_rx.recv().await.unwrap();
    assert_eq!(req.get("type").and_then(|v| v.as_str()), Some("control_request"));
    assert_eq!(
        req.get("request").and_then(|r| r.get("type")).and_then(|v| v.as_str()),
        Some("set_model")
    );
    assert_eq!(
        req.get("request").and_then(|r| r.get("model")).and_then(|v| v.as_str()),
        Some("sonnet")
    );
}

#[tokio::test]
async fn e2e_stream_input_calls_end_input() {
    let (transport, mut handle) = MockTransport::pair();
    let transport = Arc::new(Mutex::new(transport));

    let mut q = Query::new(transport.clone(), true, None, None, std::collections::HashMap::new());
    q.start().await.unwrap();

    // Prepare a short stream of input JSON values
    let inputs = vec![serde_json::json!("Hello"), serde_json::json!("World")];
    let stream = futures::stream::iter(inputs);

    q.stream_input(stream).await.unwrap();

    // Consume two input messages
    let _ = handle.sent_input_rx.recv().await.unwrap();
    let _ = handle.sent_input_rx.recv().await.unwrap();

    // Ensure end_input was called
    let ended = handle.end_input_rx.recv().await.unwrap();
    assert!(ended);
}

