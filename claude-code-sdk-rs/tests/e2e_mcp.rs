use nexus_claude::{Query, Result, transport::mock::MockTransport};
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::test]
async fn e2e_mcp_server_not_found_sends_error() -> Result<()> {
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

    // Send MCP message for a non-existent server
    let req = serde_json::json!({
        "type": "control_request",
        "request_id": "req_mcp_1",
        "request": {
            "subtype": "mcp_message",
            "server_name": "no_such_server",
            "message": {"jsonrpc": "2.0", "id": 1, "method": "ping"}
        }
    });
    handle.sdk_control_tx.send(req).await.unwrap();

    let outer = handle.outbound_control_rx.recv().await.unwrap();
    assert_eq!(outer["type"], "control_response");
    let resp = &outer["response"];
    assert_eq!(resp["subtype"], "error");
    assert_eq!(resp["request_id"], "req_mcp_1");
    assert!(
        resp["error"]
            .as_str()
            .unwrap_or("")
            .contains("no_such_server")
    );

    Ok(())
}
