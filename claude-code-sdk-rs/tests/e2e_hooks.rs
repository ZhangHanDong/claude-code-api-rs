use cc_sdk::{
    Query, HookCallback, HookContext, Result,
    transport::mock::MockTransport,
};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;

struct EchoHook;

#[async_trait]
impl HookCallback for EchoHook {
    async fn execute(
        &self,
        input: &serde_json::Value,
        _tool_use_id: Option<&str>,
        _context: &HookContext,
    ) -> serde_json::Value {
        serde_json::json!({"echo": input})
    }
}

#[tokio::test]
async fn e2e_hook_callback_success() -> Result<()> {
    let (transport, mut handle) = MockTransport::pair();
    let transport = Arc::new(Mutex::new(transport));

    let mut q = Query::new(transport.clone(), false, None, None, std::collections::HashMap::new());
    q.start().await?;

    // Register a known callback ID
    q.register_hook_callback_for_test("cb_test_1".to_string(), Arc::new(EchoHook)).await;

    // Send hook_callback control message from CLI -> SDK
    let req = serde_json::json!({
        "type": "control_request",
        "request_id": "req_hook_1",
        "request": {
            "subtype": "hook_callback",
            "callbackId": "cb_test_1",
            "input": {"a": 1},
            "toolUseId": "tu1"
        }
    });
    handle.sdk_control_tx.send(req).await.unwrap();

    // Expect a control_response from SDK -> CLI
    let outer = handle.outbound_control_rx.recv().await.unwrap();
    assert_eq!(outer["type"], "control_response");
    let resp = &outer["response"];
    assert_eq!(resp["subtype"], "success");
    assert_eq!(resp["request_id"], "req_hook_1");
    assert_eq!(resp["response"]["echo"]["a"], 1);

    Ok(())
}
