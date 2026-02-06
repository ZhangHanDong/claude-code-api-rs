use async_trait::async_trait;
use nexus_claude::{
    HookCallback, HookContext, HookInput, HookJSONOutput, Query, Result, SdkError,
    SyncHookJSONOutput, transport::mock::MockTransport,
};
use std::sync::Arc;
use tokio::sync::Mutex;

struct EchoHook;

#[async_trait]
impl HookCallback for EchoHook {
    async fn execute(
        &self,
        input: &HookInput,
        _tool_use_id: Option<&str>,
        _context: &HookContext,
    ) -> std::result::Result<HookJSONOutput, SdkError> {
        // Echo the input back as additional context
        let input_json = serde_json::to_value(input).unwrap_or_else(|_| serde_json::json!({}));

        Ok(HookJSONOutput::Sync(SyncHookJSONOutput {
            reason: Some(format!("Echoed input: {input_json}")),
            ..Default::default()
        }))
    }
}

#[tokio::test]
async fn e2e_hook_callback_success() -> Result<()> {
    let (transport, mut handle) = MockTransport::pair();
    let transport = Arc::new(Mutex::new(transport));

    let mut q = Query::new(
        transport.clone(),
        false,
        None,
        None,
        std::collections::HashMap::new(),
    );
    q.start().await?;

    // Register a known callback ID
    q.register_hook_callback_for_test("cb_test_1".to_string(), Arc::new(EchoHook))
        .await;

    // Send hook_callback control message from CLI -> SDK
    // Must use strongly-typed format with hook_event_name
    let req = serde_json::json!({
        "type": "control_request",
        "request_id": "req_hook_1",
        "request": {
            "subtype": "hook_callback",
            "callbackId": "cb_test_1",
            "input": {
                "hook_event_name": "PreToolUse",
                "session_id": "test-session",
                "transcript_path": "/tmp/transcript",
                "cwd": "/test/dir",
                "tool_name": "TestTool",
                "tool_input": {"command": "test"}
            },
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

    // Verify the hook returned a reason with the echoed input
    let response = &resp["response"];
    assert!(response.get("reason").is_some());
    let reason = response["reason"].as_str().unwrap();
    assert!(reason.contains("Echoed input"));

    Ok(())
}
