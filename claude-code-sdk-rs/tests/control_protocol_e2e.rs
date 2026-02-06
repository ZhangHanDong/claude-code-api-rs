use async_trait::async_trait;
use futures::stream::{self, Stream};
use nexus_claude::Query;
use nexus_claude::Result;
use nexus_claude::transport::Transport;
use nexus_claude::{
    CanUseTool, HookMatcher, Message, PermissionResult, PermissionResultAllow,
    ToolPermissionContext,
};
use serde_json::json;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};

struct MockTransport {
    inbound_ctrl_rx: Option<mpsc::Receiver<serde_json::Value>>,
    sent_ctrl_responses: Arc<Mutex<Vec<serde_json::Value>>>,
}

impl MockTransport {
    fn new(rx: mpsc::Receiver<serde_json::Value>) -> Self {
        Self {
            inbound_ctrl_rx: Some(rx),
            sent_ctrl_responses: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

#[async_trait]
impl Transport for MockTransport {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    async fn connect(&mut self) -> Result<()> {
        Ok(())
    }

    async fn send_message(
        &mut self,
        _message: nexus_claude::transport::InputMessage,
    ) -> Result<()> {
        Ok(())
    }

    fn receive_messages(
        &mut self,
    ) -> Pin<Box<dyn Stream<Item = Result<Message>> + Send + 'static>> {
        Box::pin(stream::empty())
    }

    async fn send_control_request(&mut self, _request: nexus_claude::ControlRequest) -> Result<()> {
        Ok(())
    }

    async fn receive_control_response(&mut self) -> Result<Option<nexus_claude::ControlResponse>> {
        Ok(None)
    }

    async fn send_sdk_control_request(&mut self, _request: serde_json::Value) -> Result<()> {
        Ok(())
    }

    async fn send_sdk_control_response(&mut self, response: serde_json::Value) -> Result<()> {
        // Mimic SubprocessTransport wrapping
        let wrapped = serde_json::json!({
            "type": "control_response",
            "response": response
        });
        self.sent_ctrl_responses.lock().await.push(wrapped);
        Ok(())
    }

    fn is_connected(&self) -> bool {
        true
    }

    async fn disconnect(&mut self) -> Result<()> {
        Ok(())
    }

    fn take_sdk_control_receiver(&mut self) -> Option<mpsc::Receiver<serde_json::Value>> {
        self.inbound_ctrl_rx.take()
    }
}

struct AllowAll;

#[async_trait]
impl CanUseTool for AllowAll {
    async fn can_use_tool(
        &self,
        _tool_name: &str,
        _input: &serde_json::Value,
        _context: &ToolPermissionContext,
    ) -> PermissionResult {
        PermissionResult::Allow(PermissionResultAllow {
            updated_input: Some(json!({"safe": true})),
            updated_permissions: None,
        })
    }
}

#[tokio::test]
async fn e2e_can_use_tool_allow() -> Result<()> {
    // Prepare inbound control request from CLI → SDK
    let (tx, rx) = mpsc::channel(10);
    let request = json!({
        "type": "control_request",
        "request_id": "req_123",
        "request": {
            "subtype": "can_use_tool",
            "tool_name": "Write",
            "input": {"path":"/tmp/demo.txt"},
            "permission_suggestions": []
        }
    });
    tx.send(request).await.unwrap();

    // Build transport and query
    let mock = MockTransport::new(rx);
    let sent_responses = mock.sent_ctrl_responses.clone();
    let transport: Arc<Mutex<Box<dyn Transport + Send>>> = Arc::new(Mutex::new(Box::new(mock)));

    let can_use = Some(Arc::new(AllowAll) as Arc<dyn CanUseTool>);
    let hooks: Option<std::collections::HashMap<String, Vec<HookMatcher>>> = None;
    let sdk_mcp_servers = std::collections::HashMap::new();

    let mut query = Query::new(transport.clone(), true, can_use, hooks, sdk_mcp_servers);

    // Start and allow background control handling
    query.start().await?;

    // Wait briefly for processing
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // Verify a control_response was sent with expected shape
    let responses = sent_responses.lock().await;
    assert!(!responses.is_empty(), "No control response sent");
    let outer = responses.last().unwrap();
    assert_eq!(outer["type"], "control_response");
    let resp = &outer["response"];
    assert_eq!(resp["subtype"], "success");
    assert_eq!(resp["request_id"], "req_123");
    assert_eq!(resp["response"]["allow"], true);
    assert_eq!(resp["response"]["input"]["safe"], true);

    Ok(())
}
