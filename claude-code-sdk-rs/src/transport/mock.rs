//! In-memory mock transport for testing and E2E simulations
use super::{InputMessage, Transport};
use crate::{
    errors::Result,
    types::{ControlRequest, ControlResponse, Message},
};
use async_trait::async_trait;
use futures::stream::{Stream, StreamExt};
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::{broadcast, mpsc};

/// Handle for interacting with the mock transport in tests
pub struct MockTransportHandle {
    /// Inject inbound messages (as if coming from CLI)
    pub inbound_message_tx: broadcast::Sender<Message>,
    /// Inject inbound SDK control JSON (as if coming from CLI)
    pub sdk_control_tx: mpsc::Sender<serde_json::Value>,
    /// Observe outbound SDK control responses sent by SDK
    pub outbound_control_rx: mpsc::Receiver<serde_json::Value>,
    /// Observe outbound SDK control requests sent by SDK
    pub outbound_control_request_rx: mpsc::Receiver<serde_json::Value>,
    /// Observe input messages sent by SDK
    pub sent_input_rx: mpsc::Receiver<InputMessage>,
    /// Observe end_input calls from SDK
    pub end_input_rx: mpsc::Receiver<bool>,
}

/// An in-memory transport implementing the `Transport` trait
pub struct MockTransport {
    connected: AtomicBool,
    // Message broadcast channel (CLI -> SDK)
    message_tx: broadcast::Sender<Message>,
    // Control response channel (legacy) (CLI -> SDK)
    control_resp_rx: Option<mpsc::Receiver<ControlResponse>>,
    // SDK control inbound channel (CLI -> SDK)
    sdk_control_rx: Option<mpsc::Receiver<serde_json::Value>>,
    // Observability channels (SDK -> CLI)
    outbound_control_tx: mpsc::Sender<serde_json::Value>,
    outbound_control_request_tx: mpsc::Sender<serde_json::Value>,
    sent_input_tx: mpsc::Sender<InputMessage>,
    end_input_tx: mpsc::Sender<bool>,
}

impl MockTransport {
    /// Create a new mock transport and a handle for tests
    pub fn pair() -> (Box<dyn Transport + Send>, MockTransportHandle) {
        let (message_tx, _rx) = broadcast::channel(100);
        let (sdk_control_tx, sdk_control_rx) = mpsc::channel(100);
        let (outbound_control_tx, outbound_control_rx) = mpsc::channel(100);
        let (outbound_control_request_tx, outbound_control_request_rx) = mpsc::channel(100);
        let (sent_input_tx, sent_input_rx) = mpsc::channel(100);
        let (end_input_tx, end_input_rx) = mpsc::channel(10);

        let transport = MockTransport {
            connected: AtomicBool::new(false),
            message_tx: message_tx.clone(),
            control_resp_rx: None,
            sdk_control_rx: Some(sdk_control_rx),
            outbound_control_tx: outbound_control_tx.clone(),
            outbound_control_request_tx: outbound_control_request_tx.clone(),
            sent_input_tx: sent_input_tx.clone(),
            end_input_tx: end_input_tx.clone(),
        };

        let handle = MockTransportHandle {
            inbound_message_tx: message_tx,
            sdk_control_tx,
            outbound_control_rx,
            outbound_control_request_rx,
            sent_input_rx,
            end_input_rx,
        };

        (Box::new(transport), handle)
    }
}

#[async_trait]
impl Transport for MockTransport {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    async fn connect(&mut self) -> Result<()> {
        self.connected.store(true, Ordering::SeqCst);
        Ok(())
    }

    async fn send_message(&mut self, message: InputMessage) -> Result<()> {
        let _ = self.sent_input_tx.send(message).await;
        Ok(())
    }

    fn receive_messages(
        &mut self,
    ) -> Pin<Box<dyn Stream<Item = Result<Message>> + Send + 'static>> {
        let rx = self.message_tx.subscribe();
        Box::pin(
            tokio_stream::wrappers::BroadcastStream::new(rx).filter_map(|r| async move {
                match r {
                    Ok(m) => Some(Ok(m)),
                    Err(_) => None,
                }
            }),
        )
    }

    async fn send_control_request(&mut self, request: ControlRequest) -> Result<()> {
        // Record as JSON for tests
        let json = match request {
            ControlRequest::Interrupt { request_id } => serde_json::json!({
                "type": "control_request",
                "request": {"type":"interrupt"},
                "request_id": request_id,
            }),
        };
        let _ = self.outbound_control_request_tx.send(json).await;
        Ok(())
    }

    async fn receive_control_response(&mut self) -> Result<Option<ControlResponse>> {
        if let Some(rx) = &mut self.control_resp_rx {
            Ok(rx.recv().await)
        } else {
            Ok(None)
        }
    }

    async fn send_sdk_control_request(&mut self, request: serde_json::Value) -> Result<()> {
        // Observe sent control requests
        let _ = self.outbound_control_request_tx.send(request).await;
        Ok(())
    }

    async fn send_sdk_control_response(&mut self, response: serde_json::Value) -> Result<()> {
        // Observe sent control responses, mimic subprocess wrapper
        let wrapped = serde_json::json!({
            "type": "control_response",
            "response": response
        });
        let _ = self.outbound_control_tx.send(wrapped).await;
        Ok(())
    }

    fn take_sdk_control_receiver(&mut self) -> Option<mpsc::Receiver<serde_json::Value>> {
        self.sdk_control_rx.take()
    }

    fn is_connected(&self) -> bool {
        self.connected.load(Ordering::SeqCst)
    }

    async fn disconnect(&mut self) -> Result<()> {
        self.connected.store(false, Ordering::SeqCst);
        Ok(())
    }

    async fn end_input(&mut self) -> Result<()> {
        let _ = self.end_input_tx.send(true).await;
        Ok(())
    }
}
