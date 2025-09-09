//! Internal query implementation with control protocol support
//!
//! This module provides the internal Query struct that handles control protocol,
//! permissions, hooks, and MCP server integration.

use crate::{
    errors::{Result, SdkError},
    transport::Transport,
    types::{
        CanUseTool, HookCallback, HookContext, HookMatcher, Message,
        PermissionResult,
        SDKControlInitializeRequest, SDKControlPermissionRequest, SDKControlRequest,
        SDKHookCallbackRequest, SDKControlInterruptRequest, ToolPermissionContext,
    },
};
use futures::stream::{Stream, StreamExt};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, RwLock};
use tracing::{debug, error, warn};

/// Internal query handler with control protocol support
pub struct Query {
    /// Transport layer (shared with client)
    transport: Arc<Mutex<crate::transport::SubprocessTransport>>,
    /// Whether in streaming mode
    is_streaming_mode: bool,
    /// Tool permission callback
    can_use_tool: Option<Arc<dyn CanUseTool>>,
    /// Hook configurations
    hooks: Option<HashMap<String, Vec<HookMatcher>>>,
    /// SDK MCP servers
    sdk_mcp_servers: HashMap<String, Arc<dyn std::any::Any + Send + Sync>>,
    /// Message channel sender
    message_tx: mpsc::Sender<Result<Message>>,
    /// Message channel receiver
    message_rx: Option<mpsc::Receiver<Result<Message>>>,
    /// Initialization result
    initialization_result: Option<JsonValue>,
    /// Active hook callbacks
    hook_callbacks: Arc<RwLock<HashMap<String, Arc<dyn HookCallback>>>>,
    /// Hook callback counter
    callback_counter: Arc<Mutex<u64>>,
}

impl Query {
    /// Create a new Query handler
    pub fn new(
        transport: Arc<Mutex<crate::transport::SubprocessTransport>>,
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
        }
    }

    /// Start the query handler
    pub async fn start(&mut self) -> Result<()> {
        // Start control request handler task
        self.start_control_handler().await;
        Ok(())
    }

    /// Initialize the control protocol
    pub async fn initialize(&mut self) -> Result<()> {
        // Send initialize request
        let init_request = SDKControlRequest::Initialize(SDKControlInitializeRequest {
            subtype: "initialize".to_string(),
            hooks: self.hooks.as_ref().map(|h| {
                h.iter()
                    .map(|(k, v)| {
                        (
                            k.clone(),
                            serde_json::json!({
                                "matchers": v.iter().map(|m| m.matcher.clone()).collect::<Vec<_>>()
                            }),
                        )
                    })
                    .collect()
            }),
        });

        // Send control request
        self.send_control_request(init_request).await?;

        // The init response will be received by the main client's message receiver
        // and can be accessed via ClaudeSDKClient::get_server_info()
        // We don't wait here to avoid blocking
        debug!("Initialization request sent");
        Ok(())
    }

    /// Send a control request
    async fn send_control_request(&mut self, request: SDKControlRequest) -> Result<()> {
        let mut transport = self.transport.lock().await;
        
        // Convert to JSON
        let json = serde_json::to_value(&request)?;
        
        debug!("Sending SDK control request: {:?}", json);
        
        // Send via transport
        transport.send_sdk_control_request(json).await?;
        
        Ok(())
    }

    /// Handle permission request
    async fn handle_permission_request(&mut self, request: SDKControlPermissionRequest) -> Result<()> {
        if let Some(ref can_use_tool) = self.can_use_tool {
            let context = ToolPermissionContext {
                signal: None,
                suggestions: request.permission_suggestions.unwrap_or_default(),
            };

            let result = can_use_tool
                .can_use_tool(&request.tool_name, &request.input, &context)
                .await;

            // Send response back
            let response = match result {
                PermissionResult::Allow(allow) => {
                    serde_json::json!({
                        "behavior": "allow",
                        "updatedInput": allow.updated_input,
                        "updatedPermissions": allow.updated_permissions,
                    })
                }
                PermissionResult::Deny(deny) => {
                    serde_json::json!({
                        "behavior": "deny",
                        "message": deny.message,
                        "interrupt": deny.interrupt,
                    })
                }
            };

            // Send response back through transport
            let mut transport = self.transport.lock().await;
            transport.send_sdk_control_response(response).await?;
            debug!("Permission response sent");
        }
        
        Ok(())
    }

    /// Handle hook callback request
    async fn handle_hook_callback(&mut self, request: SDKHookCallbackRequest) -> Result<()> {
        let callbacks = self.hook_callbacks.read().await;
        
        if let Some(callback) = callbacks.get(&request.callback_id) {
            let context = HookContext { signal: None };
            
            let response = callback
                .execute(&request.input, request.tool_use_id.as_deref(), &context)
                .await;

            // Send response back through transport
            let mut transport = self.transport.lock().await;
            let response_json = serde_json::json!({
                "subtype": "success",
                "request_id": request.callback_id,
                "response": response
            });
            transport.send_sdk_control_response(response_json).await?;
            debug!("Hook callback response sent");
        }
        
        Ok(())
    }

    /// Start control request handler task
    async fn start_control_handler(&mut self) {
        let transport = self.transport.clone();
        let can_use_tool = self.can_use_tool.clone();
        let hook_callbacks = self.hook_callbacks.clone();
        let sdk_mcp_servers = self.sdk_mcp_servers.clone();
        
        // Take ownership of the SDK control receiver to avoid holding locks
        let sdk_control_rx = {
            let mut transport_lock = transport.lock().await;
            if let Some(subprocess) = transport_lock.as_any_mut().downcast_mut::<crate::transport::SubprocessTransport>() {
                subprocess.take_sdk_control_receiver()
            } else {
                None
            }
        }; // Lock released here
        
        if let Some(mut control_rx) = sdk_control_rx {
            tokio::spawn(async move {
                // Now we can receive control requests without holding any locks
                let transport_for_control = transport;
                let can_use_tool_clone = can_use_tool;
                let hook_callbacks_clone = hook_callbacks;
                let sdk_mcp_servers_clone = sdk_mcp_servers;
                
                loop {
                    // Receive control request without holding lock
                    let control_request = control_rx.recv().await;
                    
                    if let Some(control_request) = control_request {
                        debug!("Received SDK control request: {:?}", control_request);
                        
                        // Parse and handle the control request
                        if let Some(subtype) = control_request.get("subtype").and_then(|v| v.as_str()) {
                            match subtype {
                                "can_use_tool" => {
                                    // Handle permission request
                                    if let Ok(request) = serde_json::from_value::<SDKControlPermissionRequest>(control_request.clone()) {
                                        // Handle with can_use_tool callback
                                        if let Some(ref can_use_tool) = can_use_tool_clone {
                                            let context = ToolPermissionContext {
                                                signal: None,
                                                suggestions: request.permission_suggestions.unwrap_or_default(),
                                            };
                                                
                                            let result = can_use_tool
                                                .can_use_tool(&request.tool_name, &request.input, &context)
                                                .await;
                                                
                                            let permission_response = match result {
                                                PermissionResult::Allow(allow) => {
                                                    serde_json::json!({
                                                        "behavior": "allow",
                                                        "updatedInput": allow.updated_input,
                                                        "updatedPermissions": allow.updated_permissions,
                                                    })
                                                }
                                                PermissionResult::Deny(deny) => {
                                                    serde_json::json!({
                                                        "behavior": "deny",
                                                        "message": deny.message,
                                                        "interrupt": deny.interrupt,
                                                    })
                                                }
                                            };
                                                
                                            // Wrap response with proper structure
                                            let response = serde_json::json!({
                                                "subtype": "permission_response",
                                                "request_id": control_request.get("request_id").cloned(),
                                                "response": permission_response
                                            });
                                                
                                            // Send response
                                            let mut transport = transport_for_control.lock().await;
                                            if let Err(e) = transport.send_sdk_control_response(response).await {
                                                error!("Failed to send permission response: {}", e);
                                            }
                                        }
                                    }
                                }
                                "hook_callback" => {
                                    // Handle hook callback
                                    if let Ok(request) = serde_json::from_value::<SDKHookCallbackRequest>(control_request.clone()) {
                                        let callbacks = hook_callbacks_clone.read().await;
                                        
                                        if let Some(callback) = callbacks.get(&request.callback_id) {
                                            let context = HookContext { signal: None };
                                            
                                            let response = callback
                                                .execute(&request.input, request.tool_use_id.as_deref(), &context)
                                                .await;
                                            
                                            // Send response back through transport
                                            let response_json = serde_json::json!({
                                                "subtype": "success",
                                                "request_id": request.callback_id,
                                                "response": response
                                            });
                                            
                                            let mut transport = transport_for_control.lock().await;
                                            if let Err(e) = transport.send_sdk_control_response(response_json).await {
                                                error!("Failed to send hook callback response: {}", e);
                                            }
                                        } else {
                                            warn!("No hook callback found for ID: {}", request.callback_id);
                                        }
                                    }
                                }
                                "mcp_message" => {
                                    // Handle MCP message
                                    if let Some(server_name) = control_request.get("mcp_server_name").and_then(|v| v.as_str()) {
                                        if let Some(message) = control_request.get("message") {
                                            debug!("Processing MCP message for SDK server: {}", server_name);
                                            
                                            // Check if we have an SDK server with this name
                                            if let Some(_server) = sdk_mcp_servers_clone.get(server_name) {
                                                // TODO: Implement actual MCP server invocation
                                                // For now, return a placeholder response
                                                let mcp_result = serde_json::json!({
                                                    "jsonrpc": "2.0",
                                                    "id": message.get("id"),
                                                    "result": {
                                                        "content": "MCP server response placeholder"
                                                    }
                                                });
                                                
                                                // Wrap response with proper structure like permission/hook responses
                                                let response = serde_json::json!({
                                                    "subtype": "mcp_response",
                                                    "request_id": control_request.get("request_id").cloned(),
                                                    "response": mcp_result
                                                });
                                                
                                                let mut transport = transport_for_control.lock().await;
                                                if let Err(e) = transport.send_sdk_control_response(response).await {
                                                    error!("Failed to send MCP response: {}", e);
                                                }
                                            } else {
                                                warn!("No SDK MCP server found with name: {}", server_name);
                                            }
                                        }
                                    }
                                }
                                _ => {
                                    debug!("Unknown SDK control subtype: {}", subtype);
                                }
                            }
                        }
                    }
                }
            });
        }
    }

    /// Stream input messages
    pub async fn stream_input<S>(&mut self, input_stream: S) -> Result<()>
    where
        S: Stream<Item = JsonValue> + Send + 'static,
    {
        let transport = self.transport.clone();
        
        tokio::spawn(async move {
            let mut stream = Box::pin(input_stream);
            
            while let Some(message) = stream.next().await {
                let _transport = transport.lock().await;
                
                // Convert JSON to InputMessage
                // TODO: Proper conversion
                debug!("Streaming input message: {:?}", message);
            }
        });
        
        Ok(())
    }

    /// Receive messages
    pub async fn receive_messages(&mut self) -> mpsc::Receiver<Result<Message>> {
        self.message_rx.take().expect("Receiver already taken")
    }

    /// Send interrupt request
    pub async fn interrupt(&mut self) -> Result<()> {
        let interrupt_request = SDKControlRequest::Interrupt(SDKControlInterruptRequest {
            subtype: "interrupt".to_string(),
        });
        
        self.send_control_request(interrupt_request).await
    }

    /// Handle MCP message for SDK servers
    async fn handle_mcp_message(&mut self, server_name: &str, message: &JsonValue) -> Result<JsonValue> {
        // Check if we have an SDK server with this name
        if let Some(_server) = self.sdk_mcp_servers.get(server_name) {
            // TODO: Implement actual MCP server invocation
            // For now, return a placeholder response
            debug!("Handling MCP message for SDK server {}: {:?}", server_name, message);
            Ok(serde_json::json!({
                "jsonrpc": "2.0",
                "id": message.get("id"),
                "result": {
                    "content": "MCP server response placeholder"
                }
            }))
        } else {
            Err(SdkError::InvalidState {
                message: format!("No SDK MCP server found with name: {}", server_name),
            })
        }
    }

    /// Close the query handler
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
}