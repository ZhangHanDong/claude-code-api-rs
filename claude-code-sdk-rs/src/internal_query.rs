//! Internal query implementation with control protocol support
//!
//! This module provides the internal Query struct that handles control protocol,
//! permissions, hooks, and MCP server integration.

use crate::{
    errors::{Result, SdkError},
    transport::{InputMessage, Transport},
    types::{
        CanUseTool, HookCallback, HookContext, HookMatcher, Message, PermissionResult,
        PermissionUpdate, SDKControlInitializeRequest, SDKControlInterruptRequest,
        SDKControlPermissionRequest, SDKControlRequest, SDKControlSetPermissionModeRequest,
        SDKHookCallbackRequest, ToolPermissionContext,
    },
};
use futures::StreamExt;
use futures::stream::Stream;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock, mpsc};
use tokio::time::{Duration, timeout};
use tracing::{debug, error, warn};

/// Internal query handler with control protocol support
pub struct Query {
    /// Transport layer (shared with client)
    transport: Arc<Mutex<Box<dyn Transport + Send>>>,
    /// Whether in streaming mode
    #[allow(dead_code)]
    is_streaming_mode: bool,
    /// Tool permission callback
    can_use_tool: Option<Arc<dyn CanUseTool>>,
    /// Hook configurations
    hooks: Option<HashMap<String, Vec<HookMatcher>>>,
    /// SDK MCP servers
    sdk_mcp_servers: HashMap<String, Arc<dyn std::any::Any + Send + Sync>>,
    /// Message channel sender (reserved for future streaming receive support)
    #[allow(dead_code)]
    message_tx: mpsc::Sender<Result<Message>>,
    /// Message channel receiver (reserved for future streaming receive support)
    #[allow(dead_code)]
    message_rx: Option<mpsc::Receiver<Result<Message>>>,
    /// Initialization result
    initialization_result: Option<JsonValue>,
    /// Active hook callbacks
    hook_callbacks: Arc<RwLock<HashMap<String, Arc<dyn HookCallback>>>>,
    /// Hook callback counter
    callback_counter: Arc<Mutex<u64>>,
    /// Request counter for generating unique IDs
    request_counter: Arc<Mutex<u64>>,
    /// Pending control request responses
    pending_responses: Arc<RwLock<HashMap<String, tokio::sync::oneshot::Sender<JsonValue>>>>,
}

impl Query {
    /// Create a new Query handler
    pub fn new(
        transport: Arc<Mutex<Box<dyn Transport + Send>>>,
        is_streaming_mode: bool,
        can_use_tool: Option<Arc<dyn CanUseTool>>,
        hooks: Option<HashMap<String, Vec<HookMatcher>>>,
        sdk_mcp_servers: HashMap<String, Arc<dyn std::any::Any + Send + Sync>>,
    ) -> Self {
        let (tx, rx) = mpsc::channel(100);

        Self {
            transport,
            is_streaming_mode,
            can_use_tool,
            hooks,
            sdk_mcp_servers,
            message_tx: tx,
            message_rx: Some(rx),
            initialization_result: None,
            hook_callbacks: Arc::new(RwLock::new(HashMap::new())),
            callback_counter: Arc::new(Mutex::new(0)),
            request_counter: Arc::new(Mutex::new(0)),
            pending_responses: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Test helper to register a hook callback with a known ID
    ///
    /// This is intended for E2E tests to inject a callback ID that can be
    /// referenced by inbound `hook_callback` control messages.
    pub async fn register_hook_callback_for_test(
        &self,
        callback_id: String,
        callback: Arc<dyn HookCallback>,
    ) {
        let mut map = self.hook_callbacks.write().await;
        map.insert(callback_id, callback);
    }

    /// Start the query handler
    pub async fn start(&mut self) -> Result<()> {
        // Start control request handler task
        self.start_control_handler().await;

        // Start SDK message forwarder task (route non-control messages to message_tx)
        let transport = self.transport.clone();
        let tx = self.message_tx.clone();
        tokio::spawn(async move {
            // Get message stream once and consume it continuously
            let mut stream = {
                let mut guard = transport.lock().await;
                guard.receive_messages()
            }; // Lock released immediately after getting stream

            // Continuously consume from the same stream
            while let Some(result) = stream.next().await {
                match result {
                    Ok(msg) => {
                        if tx.send(Ok(msg)).await.is_err() {
                            break;
                        }
                    },
                    Err(e) => {
                        let _ = tx.send(Err(e)).await;
                        break;
                    },
                }
            }
        });
        Ok(())
    }

    /// Initialize the control protocol
    pub async fn initialize(&mut self) -> Result<()> {
        // Build hooks with callback IDs (Python SDK style)
        let hooks_with_ids = if let Some(ref hooks) = self.hooks {
            let mut counter = self.callback_counter.lock().await;
            let mut callbacks_map = self.hook_callbacks.write().await;

            let hooks_json: HashMap<String, JsonValue> = hooks
                .iter()
                .map(|(event_name, matchers)| {
                    let matchers_with_ids: Vec<JsonValue> = matchers
                        .iter()
                        .map(|matcher| {
                            // Generate callback IDs for each hook in this matcher
                            let callback_ids: Vec<String> = matcher
                                .hooks
                                .iter()
                                .map(|hook_callback| {
                                    *counter += 1;
                                    let callback_id = format!(
                                        "hook_{}_{}",
                                        *counter,
                                        uuid::Uuid::new_v4().simple()
                                    );

                                    // Store the callback for later use
                                    callbacks_map
                                        .insert(callback_id.clone(), hook_callback.clone());

                                    callback_id
                                })
                                .collect();

                            serde_json::json!({
                                "matcher": matcher.matcher.clone(),
                                "hookCallbackIds": callback_ids
                            })
                        })
                        .collect();

                    (event_name.clone(), serde_json::json!(matchers_with_ids))
                })
                .collect();

            Some(hooks_json)
        } else {
            None
        };

        // Send initialize request
        let init_request = SDKControlRequest::Initialize(SDKControlInitializeRequest {
            subtype: "initialize".to_string(),
            hooks: hooks_with_ids,
        });

        // Send control request and save result
        let result = self.send_control_request(init_request).await?;
        self.initialization_result = Some(result);

        debug!("Initialization request sent with hook callback IDs");
        Ok(())
    }

    /// Send a control request and wait for response
    async fn send_control_request(&mut self, request: SDKControlRequest) -> Result<JsonValue> {
        // Generate unique request ID
        let request_id = {
            let mut counter = self.request_counter.lock().await;
            *counter += 1;
            format!("req_{}_{}", *counter, uuid::Uuid::new_v4().simple())
        };

        // Create oneshot channel for response
        let (tx, rx) = tokio::sync::oneshot::channel();

        // Register pending response
        {
            let mut pending = self.pending_responses.write().await;
            pending.insert(request_id.clone(), tx);
        }

        // Build control request with request_id (snake_case for CLI compatibility)
        let control_request = serde_json::json!({
            "type": "control_request",
            "request_id": request_id,
            "request": request
        });

        debug!("Sending control request: {:?}", control_request);

        // Send via transport
        {
            let mut transport = self.transport.lock().await;
            transport.send_sdk_control_request(control_request).await?;
        }

        // Wait for response with timeout
        match timeout(Duration::from_secs(60), rx).await {
            Ok(Ok(response)) => {
                debug!("Received control response for {}", request_id);

                // Python parity: treat subtype=error as an error, and return only
                // the payload from `response` (or legacy `data`) on success.
                if response.get("subtype").and_then(|v| v.as_str()) == Some("error") {
                    let msg = response
                        .get("error")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown control request error");
                    return Err(SdkError::ControlRequestError(msg.to_string()));
                }

                Ok(response
                    .get("response")
                    .or_else(|| response.get("data"))
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!({})))
            },
            Ok(Err(_)) => Err(SdkError::ControlRequestError(
                "Response channel closed".to_string(),
            )),
            Err(_) => {
                // Clean up pending response
                let mut pending = self.pending_responses.write().await;
                pending.remove(&request_id);
                Err(SdkError::Timeout { seconds: 60 })
            },
        }
    }

    /// Handle permission request
    #[allow(dead_code)]
    async fn handle_permission_request(
        &mut self,
        request: SDKControlPermissionRequest,
    ) -> Result<()> {
        if let Some(ref can_use_tool) = self.can_use_tool {
            let context = ToolPermissionContext {
                signal: None,
                suggestions: request.permission_suggestions.unwrap_or_default(),
            };

            let result = can_use_tool
                .can_use_tool(&request.tool_name, &request.input, &context)
                .await;

            // Send response back (CLI expects: { allow: bool, input?, reason? })
            let response = match result {
                PermissionResult::Allow(allow) => {
                    let mut obj = serde_json::json!({ "allow": true });
                    if let Some(updated) = allow.updated_input {
                        obj["input"] = updated;
                    }
                    obj
                },
                PermissionResult::Deny(deny) => {
                    let mut obj = serde_json::json!({ "allow": false });
                    if !deny.message.is_empty() {
                        obj["reason"] = serde_json::json!(deny.message);
                    }
                    obj
                },
            };

            // Send response back through transport
            let mut transport = self.transport.lock().await;
            transport.send_sdk_control_response(response).await?;
            debug!("Permission response sent");
        }

        Ok(())
    }

    /// Extract requestId from CLI message (supports both camelCase and snake_case)
    fn extract_request_id(msg: &JsonValue) -> Option<JsonValue> {
        msg.get("requestId")
            .or_else(|| msg.get("request_id"))
            .cloned()
    }

    /// Start control request handler task
    async fn start_control_handler(&mut self) {
        let transport = self.transport.clone();
        let can_use_tool = self.can_use_tool.clone();
        let hook_callbacks = self.hook_callbacks.clone();
        let sdk_mcp_servers = self.sdk_mcp_servers.clone();
        let pending_responses = self.pending_responses.clone();

        // Take ownership of the SDK control receiver to avoid holding locks
        let sdk_control_rx = {
            let mut transport_lock = transport.lock().await;
            transport_lock.take_sdk_control_receiver()
        }; // Lock released here

        if let Some(mut control_rx) = sdk_control_rx {
            tokio::spawn(async move {
                // Now we can receive control requests without holding any locks
                let transport_for_control = transport;
                let can_use_tool_clone = can_use_tool;
                let hook_callbacks_clone = hook_callbacks;
                let sdk_mcp_servers_clone = sdk_mcp_servers;
                let pending_responses_clone = pending_responses;

                loop {
                    // Receive control request without holding lock
                    let control_message = control_rx.recv().await;

                    if let Some(control_message) = control_message {
                        debug!("Received control message: {:?}", control_message);

                        // Check if this is a control response (from CLI to SDK)
                        if control_message.get("type").and_then(|v| v.as_str())
                            == Some("control_response")
                        {
                            // Expected shape: {"type":"control_response", "response": {"request_id": "...", ...}}
                            if let Some(resp_obj) = control_message.get("response") {
                                let request_id = resp_obj
                                    .get("request_id")
                                    .or_else(|| resp_obj.get("requestId"))
                                    .and_then(|v| v.as_str());

                                if let Some(request_id) = request_id {
                                    let mut pending = pending_responses_clone.write().await;
                                    if let Some(tx) = pending.remove(request_id) {
                                        // Deliver the nested control response object; send_control_request will
                                        // extract the `response` (or legacy `data`) payload for callers.
                                        let _ = tx.send(resp_obj.clone());
                                        debug!(
                                            "Control response delivered for request_id: {}",
                                            request_id
                                        );
                                    } else {
                                        warn!(
                                            "No pending request found for request_id: {}",
                                            request_id
                                        );
                                    }
                                } else {
                                    warn!(
                                        "Control response missing request_id: {:?}",
                                        control_message
                                    );
                                }
                            } else {
                                warn!(
                                    "Control response missing 'response' payload: {:?}",
                                    control_message
                                );
                            }
                            continue;
                        }

                        // Parse and handle control requests (from CLI to SDK)
                        // Check if this is a control_request with a nested request field
                        let request_data = if control_message.get("type").and_then(|v| v.as_str())
                            == Some("control_request")
                        {
                            control_message
                                .get("request")
                                .cloned()
                                .unwrap_or(control_message.clone())
                        } else {
                            control_message.clone()
                        };

                        if let Some(subtype) = request_data.get("subtype").and_then(|v| v.as_str())
                        {
                            match subtype {
                                "can_use_tool" => {
                                    // Handle permission request
                                    if let Ok(request) =
                                        serde_json::from_value::<SDKControlPermissionRequest>(
                                            request_data.clone(),
                                        )
                                    {
                                        // Handle with can_use_tool callback
                                        if let Some(ref can_use_tool) = can_use_tool_clone {
                                            let context = ToolPermissionContext {
                                                signal: None,
                                                suggestions: request
                                                    .permission_suggestions
                                                    .unwrap_or_default(),
                                            };

                                            let result = can_use_tool
                                                .can_use_tool(
                                                    &request.tool_name,
                                                    &request.input,
                                                    &context,
                                                )
                                                .await;

                                            // CLI expects: {"allow": true, "input": ...} or {"allow": false, "reason": ...}
                                            let permission_response = match result {
                                                PermissionResult::Allow(allow) => {
                                                    let mut resp = serde_json::json!({
                                                        "allow": true,
                                                    });
                                                    if let Some(input) = allow.updated_input {
                                                        resp["input"] = input;
                                                    }
                                                    if let Some(perms) = allow.updated_permissions {
                                                        resp["updatedPermissions"] =
                                                            serde_json::to_value(perms)
                                                                .unwrap_or_default();
                                                    }
                                                    resp
                                                },
                                                PermissionResult::Deny(deny) => {
                                                    let mut resp = serde_json::json!({
                                                        "allow": false,
                                                    });
                                                    if !deny.message.is_empty() {
                                                        resp["reason"] =
                                                            serde_json::json!(deny.message);
                                                    }
                                                    if deny.interrupt {
                                                        resp["interrupt"] = serde_json::json!(true);
                                                    }
                                                    resp
                                                },
                                            };

                                            // Wrap response with proper structure
                                            // CLI expects "subtype": "success" for all successful responses
                                            let response = serde_json::json!({
                                                "subtype": "success",
                                                "request_id": Self::extract_request_id(&control_message),
                                                "response": permission_response
                                            });

                                            // Send response
                                            let mut transport = transport_for_control.lock().await;
                                            if let Err(e) =
                                                transport.send_sdk_control_response(response).await
                                            {
                                                error!("Failed to send permission response: {}", e);
                                            }
                                        }
                                    } else {
                                        // Fallback for snake_case fields (tool_name, permission_suggestions)
                                        if let Some(tool_name) =
                                            request_data.get("tool_name").and_then(|v| v.as_str())
                                            && let Some(input_val) =
                                                request_data.get("input").cloned()
                                            && let Some(ref can_use_tool) = can_use_tool_clone
                                        {
                                            // Try to parse permission suggestions (snake_case)
                                            let suggestions: Vec<PermissionUpdate> = request_data
                                                .get("permission_suggestions")
                                                .cloned()
                                                .and_then(|v| {
                                                    serde_json::from_value::<Vec<PermissionUpdate>>(
                                                        v,
                                                    )
                                                    .ok()
                                                })
                                                .unwrap_or_default();

                                            let context = ToolPermissionContext {
                                                signal: None,
                                                suggestions,
                                            };
                                            let result = can_use_tool
                                                .can_use_tool(tool_name, &input_val, &context)
                                                .await;

                                            let permission_response = match result {
                                                PermissionResult::Allow(allow) => {
                                                    let mut resp =
                                                        serde_json::json!({ "allow": true });
                                                    if let Some(input) = allow.updated_input {
                                                        resp["input"] = input;
                                                    }
                                                    if let Some(perms) = allow.updated_permissions {
                                                        resp["updatedPermissions"] =
                                                            serde_json::to_value(perms)
                                                                .unwrap_or_default();
                                                    }
                                                    resp
                                                },
                                                PermissionResult::Deny(deny) => {
                                                    let mut resp =
                                                        serde_json::json!({ "allow": false });
                                                    if !deny.message.is_empty() {
                                                        resp["reason"] =
                                                            serde_json::json!(deny.message);
                                                    }
                                                    if deny.interrupt {
                                                        resp["interrupt"] = serde_json::json!(true);
                                                    }
                                                    resp
                                                },
                                            };

                                            let response = serde_json::json!({
                                                "subtype": "success",
                                                "request_id": Self::extract_request_id(&control_message),
                                                "response": permission_response
                                            });
                                            let mut transport = transport_for_control.lock().await;
                                            if let Err(e) =
                                                transport.send_sdk_control_response(response).await
                                            {
                                                error!(
                                                    "Failed to send permission response (fallback): {}",
                                                    e
                                                );
                                            }
                                        }
                                    }
                                },
                                "hook_callback" => {
                                    // Handle hook callback with strongly-typed inputs/outputs
                                    if let Ok(request) =
                                        serde_json::from_value::<SDKHookCallbackRequest>(
                                            request_data.clone(),
                                        )
                                    {
                                        let callbacks = hook_callbacks_clone.read().await;

                                        if let Some(callback) = callbacks.get(&request.callback_id)
                                        {
                                            let context = HookContext { signal: None };

                                            // Try to deserialize input as HookInput
                                            let hook_result = match serde_json::from_value::<
                                                crate::types::HookInput,
                                            >(
                                                request.input.clone()
                                            ) {
                                                Ok(hook_input) => {
                                                    // Call the hook with strongly-typed input
                                                    callback
                                                        .execute(
                                                            &hook_input,
                                                            request.tool_use_id.as_deref(),
                                                            &context,
                                                        )
                                                        .await
                                                },
                                                Err(parse_err) => {
                                                    error!(
                                                        "Failed to parse hook input: {}",
                                                        parse_err
                                                    );
                                                    // Return error using MessageParseError
                                                    Err(crate::errors::SdkError::MessageParseError {
                                                        error: format!("Invalid hook input: {parse_err}"),
                                                        raw: request.input.to_string(),
                                                    })
                                                },
                                            };

                                            // Handle hook result
                                            let response_json = match hook_result {
                                                Ok(hook_output) => {
                                                    // Serialize HookJSONOutput to JSON
                                                    let output_value = serde_json::to_value(
                                                        &hook_output,
                                                    )
                                                    .unwrap_or_else(|e| {
                                                        error!(
                                                            "Failed to serialize hook output: {}",
                                                            e
                                                        );
                                                        serde_json::json!({})
                                                    });

                                                    serde_json::json!({
                                                        "subtype": "success",
                                                        "request_id": Self::extract_request_id(&control_message),
                                                        "response": output_value
                                                    })
                                                },
                                                Err(e) => {
                                                    error!("Hook callback failed: {}", e);
                                                    serde_json::json!({
                                                        "subtype": "error",
                                                        "request_id": Self::extract_request_id(&control_message),
                                                        "error": e.to_string()
                                                    })
                                                },
                                            };

                                            let mut transport = transport_for_control.lock().await;
                                            if let Err(e) = transport
                                                .send_sdk_control_response(response_json)
                                                .await
                                            {
                                                error!(
                                                    "Failed to send hook callback response: {}",
                                                    e
                                                );
                                            }
                                        } else {
                                            warn!(
                                                "No hook callback found for ID: {}",
                                                request.callback_id
                                            );
                                            // Send error response
                                            let error_response = serde_json::json!({
                                                "subtype": "error",
                                                "request_id": Self::extract_request_id(&control_message),
                                                "error": format!("No hook callback found for ID: {}", request.callback_id)
                                            });
                                            let mut transport = transport_for_control.lock().await;
                                            if let Err(e) = transport
                                                .send_sdk_control_response(error_response)
                                                .await
                                            {
                                                error!("Failed to send error response: {}", e);
                                            }
                                        }
                                    } else {
                                        // Fallback for snake_case fields (callback_id, tool_use_id)
                                        let callback_id = request_data
                                            .get("callback_id")
                                            .and_then(|v| v.as_str());
                                        let tool_use_id = request_data
                                            .get("tool_use_id")
                                            .and_then(|v| v.as_str())
                                            .map(|s| s.to_string());
                                        let input = request_data
                                            .get("input")
                                            .cloned()
                                            .unwrap_or(serde_json::json!({}));

                                        if let Some(callback_id) = callback_id {
                                            let callbacks = hook_callbacks_clone.read().await;
                                            if let Some(callback) = callbacks.get(callback_id) {
                                                let context = HookContext { signal: None };

                                                // Try to parse as HookInput
                                                let hook_result = match serde_json::from_value::<
                                                    crate::types::HookInput,
                                                >(
                                                    input.clone()
                                                ) {
                                                    Ok(hook_input) => {
                                                        callback
                                                            .execute(
                                                                &hook_input,
                                                                tool_use_id.as_deref(),
                                                                &context,
                                                            )
                                                            .await
                                                    },
                                                    Err(parse_err) => {
                                                        error!(
                                                            "Failed to parse hook input (fallback): {}",
                                                            parse_err
                                                        );
                                                        Err(crate::errors::SdkError::MessageParseError {
                                                            error: format!("Invalid hook input: {parse_err}"),
                                                            raw: input.to_string(),
                                                        })
                                                    },
                                                };

                                                let response_json = match hook_result {
                                                    Ok(hook_output) => {
                                                        let output_value = serde_json::to_value(&hook_output)
                                                            .unwrap_or_else(|e| {
                                                                error!("Failed to serialize hook output (fallback): {}", e);
                                                                serde_json::json!({})
                                                            });

                                                        serde_json::json!({
                                                            "subtype": "success",
                                                            "request_id": Self::extract_request_id(&control_message),
                                                            "response": output_value
                                                        })
                                                    },
                                                    Err(e) => {
                                                        error!(
                                                            "Hook callback failed (fallback): {}",
                                                            e
                                                        );
                                                        serde_json::json!({
                                                            "subtype": "error",
                                                            "request_id": Self::extract_request_id(&control_message),
                                                            "error": e.to_string()
                                                        })
                                                    },
                                                };

                                                let mut transport =
                                                    transport_for_control.lock().await;
                                                if let Err(e) = transport
                                                    .send_sdk_control_response(response_json)
                                                    .await
                                                {
                                                    error!(
                                                        "Failed to send hook callback response (fallback): {}",
                                                        e
                                                    );
                                                }
                                            } else {
                                                warn!(
                                                    "No hook callback found for ID: {}",
                                                    callback_id
                                                );
                                            }
                                        } else {
                                            warn!(
                                                "Invalid hook_callback control message: missing callback_id"
                                            );
                                        }
                                    }
                                },
                                "mcp_message" => {
                                    // Handle MCP message
                                    if let Some(server_name) =
                                        request_data.get("server_name").and_then(|v| v.as_str())
                                        && let Some(message) = request_data.get("message")
                                    {
                                        debug!(
                                            "Processing MCP message for SDK server: {}",
                                            server_name
                                        );

                                        // Check if we have an SDK server with this name
                                        if let Some(server_arc) =
                                            sdk_mcp_servers_clone.get(server_name)
                                        {
                                            // Try to downcast to SdkMcpServer
                                            if let Some(sdk_server) = server_arc
                                                .downcast_ref::<crate::sdk_mcp::SdkMcpServer>(
                                            ) {
                                                // Call the SDK MCP server
                                                match sdk_server
                                                    .handle_message(message.clone())
                                                    .await
                                                {
                                                    Ok(mcp_result) => {
                                                        // Wrap response with proper structure
                                                        let response = serde_json::json!({
                                                            "subtype": "success",
                                                            "request_id": Self::extract_request_id(&control_message),
                                                            "response": {
                                                                "mcp_response": mcp_result
                                                            }
                                                        });

                                                        let mut transport =
                                                            transport_for_control.lock().await;
                                                        if let Err(e) = transport
                                                            .send_sdk_control_response(response)
                                                            .await
                                                        {
                                                            error!(
                                                                "Failed to send MCP response: {}",
                                                                e
                                                            );
                                                        }
                                                    },
                                                    Err(e) => {
                                                        error!("SDK MCP server error: {}", e);
                                                        let error_response = serde_json::json!({
                                                            "subtype": "error",
                                                            "request_id": Self::extract_request_id(&control_message),
                                                            "error": format!("MCP server error: {}", e)
                                                        });

                                                        let mut transport =
                                                            transport_for_control.lock().await;
                                                        if let Err(e) = transport
                                                            .send_sdk_control_response(
                                                                error_response,
                                                            )
                                                            .await
                                                        {
                                                            error!(
                                                                "Failed to send MCP error response: {}",
                                                                e
                                                            );
                                                        }
                                                    },
                                                }
                                            } else {
                                                warn!(
                                                    "SDK server '{}' is not of type SdkMcpServer",
                                                    server_name
                                                );
                                            }
                                        } else {
                                            warn!(
                                                "No SDK MCP server found with name: {}",
                                                server_name
                                            );
                                            let error_response = serde_json::json!({
                                                "subtype": "error",
                                                "request_id": Self::extract_request_id(&control_message),
                                                "error": format!("Server '{}' not found", server_name)
                                            });

                                            let mut transport = transport_for_control.lock().await;
                                            if let Err(e) = transport
                                                .send_sdk_control_response(error_response)
                                                .await
                                            {
                                                error!("Failed to send MCP error response: {}", e);
                                            }
                                        }
                                    }
                                },
                                _ => {
                                    debug!("Unknown SDK control subtype: {}", subtype);
                                },
                            }
                        }
                    }
                }
            });
        }
    }

    /// Stream input messages to the CLI stdin by converting JSON values to InputMessage
    #[allow(dead_code)]
    pub async fn stream_input<S>(&mut self, input_stream: S) -> Result<()>
    where
        S: Stream<Item = JsonValue> + Send + 'static,
    {
        let transport = self.transport.clone();

        tokio::spawn(async move {
            use futures::StreamExt;
            let mut stream = Box::pin(input_stream);

            while let Some(value) = stream.next().await {
                // Best-effort conversion from arbitrary JSON to InputMessage
                let input_msg_opt = Self::json_to_input_message(value);
                if let Some(input_msg) = input_msg_opt {
                    let mut guard = transport.lock().await;
                    if let Err(e) = guard.send_message(input_msg).await {
                        warn!("Failed to send streaming input message: {}", e);
                    }
                } else {
                    warn!("Invalid streaming input JSON; expected user message shape");
                }
            }

            // After streaming all inputs, signal end of input
            let mut guard = transport.lock().await;
            if let Err(e) = guard.end_input().await {
                warn!("Failed to signal end_input: {}", e);
            }
        });
        Ok(())
    }

    /// Receive messages
    #[allow(dead_code)]
    pub async fn receive_messages(&mut self) -> mpsc::Receiver<Result<Message>> {
        self.message_rx.take().expect("Receiver already taken")
    }

    /// Send interrupt request
    pub async fn interrupt(&mut self) -> Result<()> {
        let interrupt_request = SDKControlRequest::Interrupt(SDKControlInterruptRequest {
            subtype: "interrupt".to_string(),
        });

        self.send_control_request(interrupt_request).await?;
        Ok(())
    }

    /// Set permission mode via control protocol
    #[allow(dead_code)]
    pub async fn set_permission_mode(&mut self, mode: &str) -> Result<()> {
        let req = SDKControlRequest::SetPermissionMode(SDKControlSetPermissionModeRequest {
            subtype: "set_permission_mode".to_string(),
            mode: mode.to_string(),
        });
        // Ignore response payload; errors propagate
        let _ = self.send_control_request(req).await?;
        Ok(())
    }

    /// Set the active model via control protocol
    #[allow(dead_code)]
    pub async fn set_model(&mut self, model: Option<String>) -> Result<()> {
        let req = SDKControlRequest::SetModel(crate::types::SDKControlSetModelRequest {
            subtype: "set_model".to_string(),
            model,
        });
        let _ = self.send_control_request(req).await?;
        Ok(())
    }

    /// Rewind tracked files to their state at a specific user message
    ///
    /// Requires `enable_file_checkpointing` to be enabled in `ClaudeCodeOptions`.
    ///
    /// # Arguments
    ///
    /// * `user_message_id` - UUID of the user message to rewind to
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use nexus_claude::{ClaudeSDKClient, ClaudeCodeOptions};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let options = ClaudeCodeOptions::builder()
    ///     .enable_file_checkpointing(true)
    ///     .build();
    /// let mut client = ClaudeSDKClient::new(options);
    /// client.connect(None).await?;
    ///
    /// // Later, rewind to a checkpoint
    /// // client.rewind_files("user-message-uuid-here").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn rewind_files(&mut self, user_message_id: &str) -> Result<()> {
        let req = SDKControlRequest::RewindFiles(crate::types::SDKControlRewindFilesRequest::new(
            user_message_id,
        ));
        let _ = self.send_control_request(req).await?;
        Ok(())
    }

    /// Handle MCP message for SDK servers
    #[allow(dead_code)]
    async fn handle_mcp_message(
        &mut self,
        server_name: &str,
        message: &JsonValue,
    ) -> Result<JsonValue> {
        // Check if we have an SDK server with this name
        if let Some(_server) = self.sdk_mcp_servers.get(server_name) {
            // TODO: Implement actual MCP server invocation
            // For now, return a placeholder response
            debug!(
                "Handling MCP message for SDK server {}: {:?}",
                server_name, message
            );
            Ok(serde_json::json!({
                "jsonrpc": "2.0",
                "id": message.get("id"),
                "result": {
                    "content": "MCP server response placeholder"
                }
            }))
        } else {
            Err(SdkError::InvalidState {
                message: format!("No SDK MCP server found with name: {server_name}"),
            })
        }
    }

    /// Close the query handler
    #[allow(dead_code)]
    pub async fn close(&mut self) -> Result<()> {
        // Clean up resources
        let mut transport = self.transport.lock().await;
        transport.disconnect().await?;
        Ok(())
    }

    /// Get initialization result
    pub fn get_initialization_result(&self) -> Option<&JsonValue> {
        self.initialization_result.as_ref()
    }

    /// Convert arbitrary JSON value to InputMessage understood by CLI
    #[allow(dead_code)]
    fn json_to_input_message(v: JsonValue) -> Option<InputMessage> {
        // 1) Already in SDK message shape
        if let Some(obj) = v.as_object() {
            if let (Some(t), Some(message)) = (obj.get("type"), obj.get("message"))
                && t.as_str() == Some("user")
            {
                let parent = obj
                    .get("parent_tool_use_id")
                    .and_then(|p| p.as_str().map(|s| s.to_string()));
                let session_id = obj
                    .get("session_id")
                    .and_then(|s| s.as_str())
                    .unwrap_or("default")
                    .to_string();

                let im = InputMessage {
                    r#type: "user".to_string(),
                    message: message.clone(),
                    parent_tool_use_id: parent,
                    session_id,
                };
                return Some(im);
            }

            // 2) Simple wrapper: {"content":"...", "session_id":"..."}
            if let Some(content) = obj.get("content").and_then(|c| c.as_str()) {
                let session_id = obj
                    .get("session_id")
                    .and_then(|s| s.as_str())
                    .unwrap_or("default")
                    .to_string();
                return Some(InputMessage::user(content.to_string(), session_id));
            }
        }

        // 3) Bare string
        if let Some(s) = v.as_str() {
            return Some(InputMessage::user(s.to_string(), "default".to_string()));
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_request_id_supports_both_cases() {
        let snake = serde_json::json!({"request_id": "req_1"});
        let camel = serde_json::json!({"requestId": "req_2"});
        assert_eq!(
            Query::extract_request_id(&snake),
            Some(serde_json::json!("req_1"))
        );
        assert_eq!(
            Query::extract_request_id(&camel),
            Some(serde_json::json!("req_2"))
        );
    }

    #[test]
    fn test_json_to_input_message_from_string() {
        let v = serde_json::json!("Hello");
        let im = Query::json_to_input_message(v).expect("should convert");
        assert_eq!(im.r#type, "user");
        assert_eq!(im.session_id, "default");
        assert_eq!(im.message["content"].as_str().unwrap(), "Hello");
    }

    #[test]
    fn test_json_to_input_message_from_object_content() {
        let v = serde_json::json!({"content":"Ping","session_id":"s1"});
        let im = Query::json_to_input_message(v).expect("should convert");
        assert_eq!(im.session_id, "s1");
        assert_eq!(im.message["content"].as_str().unwrap(), "Ping");
    }

    #[test]
    fn test_json_to_input_message_full_user_shape() {
        let v = serde_json::json!({
            "type":"user",
            "message": {"role":"user","content":"Hi"},
            "session_id": "abc",
            "parent_tool_use_id": null
        });
        let im = Query::json_to_input_message(v).expect("should convert");
        assert_eq!(im.session_id, "abc");
        assert_eq!(im.message["role"].as_str().unwrap(), "user");
        assert_eq!(im.message["content"].as_str().unwrap(), "Hi");
    }
}
