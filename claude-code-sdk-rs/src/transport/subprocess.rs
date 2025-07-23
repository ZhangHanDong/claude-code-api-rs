//! Subprocess-based transport implementation
//!
//! This module implements the Transport trait using a subprocess to run the Claude CLI.

use super::{InputMessage, Transport, TransportState};
use crate::{
    errors::{Result, SdkError},
    types::{ClaudeCodeOptions, ControlRequest, ControlResponse, Message, PermissionMode},
};
use async_trait::async_trait;
use futures::stream::{Stream, StreamExt};
use std::path::PathBuf;
use std::pin::Pin;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

/// Default buffer size for channels
const CHANNEL_BUFFER_SIZE: usize = 100;

/// Subprocess-based transport for Claude CLI
pub struct SubprocessTransport {
    /// Configuration options
    options: ClaudeCodeOptions,
    /// CLI binary path
    cli_path: PathBuf,
    /// Child process
    child: Option<Child>,
    /// Sender for stdin
    stdin_tx: Option<mpsc::Sender<String>>,
    /// Sender for broadcasting messages to multiple receivers
    message_broadcast_tx: Option<tokio::sync::broadcast::Sender<Message>>,
    /// Receiver for control responses
    control_rx: Option<mpsc::Receiver<ControlResponse>>,
    /// Transport state
    state: TransportState,
    /// Request counter for control requests
    request_counter: u64,
    /// Whether to close stdin after initial prompt
    #[allow(dead_code)]
    close_stdin_after_prompt: bool,
}

impl SubprocessTransport {
    /// Create a new subprocess transport
    pub fn new(options: ClaudeCodeOptions) -> Result<Self> {
        let cli_path = find_claude_cli()?;
        Ok(Self {
            options,
            cli_path,
            child: None,
            stdin_tx: None,
            message_broadcast_tx: None,
            control_rx: None,
            state: TransportState::Disconnected,
            request_counter: 0,
            close_stdin_after_prompt: false,
        })
    }

    /// Create with a specific CLI path
    pub fn with_cli_path(options: ClaudeCodeOptions, cli_path: impl Into<PathBuf>) -> Self {
        Self {
            options,
            cli_path: cli_path.into(),
            child: None,
            stdin_tx: None,
            message_broadcast_tx: None,
            control_rx: None,
            state: TransportState::Disconnected,
            request_counter: 0,
            close_stdin_after_prompt: false,
        }
    }

    /// Set whether to close stdin after sending the initial prompt
    #[allow(dead_code)]
    pub fn set_close_stdin_after_prompt(&mut self, close: bool) {
        self.close_stdin_after_prompt = close;
    }
    
    /// Create transport for simple print mode (one-shot query)
    #[allow(dead_code)]
    pub fn for_print_mode(options: ClaudeCodeOptions, _prompt: String) -> Result<Self> {
        let cli_path = find_claude_cli()?;
        Ok(Self {
            options,
            cli_path,
            child: None,
            stdin_tx: None,
            message_broadcast_tx: None,
            control_rx: None,
            state: TransportState::Disconnected,
            request_counter: 0,
            close_stdin_after_prompt: true,
        })
    }

    /// Build the command with all necessary arguments
    fn build_command(&self) -> Command {
        let mut cmd = Command::new(&self.cli_path);

        // Always use output-format stream-json and verbose (like Python SDK)
        cmd.arg("--output-format").arg("stream-json");
        cmd.arg("--verbose");
        
        // For streaming/interactive mode, also add input-format stream-json
        cmd.arg("--input-format").arg("stream-json");

        // System prompts
        if let Some(ref prompt) = self.options.system_prompt {
            cmd.arg("--system-prompt").arg(prompt);
        }
        if let Some(ref prompt) = self.options.append_system_prompt {
            cmd.arg("--append-system-prompt").arg(prompt);
        }

        // Tool configuration
        if !self.options.allowed_tools.is_empty() {
            cmd.arg("--allowedTools")
                .arg(self.options.allowed_tools.join(","));
        }
        if !self.options.disallowed_tools.is_empty() {
            cmd.arg("--disallowedTools")
                .arg(self.options.disallowed_tools.join(","));
        }

        // Permission mode
        match self.options.permission_mode {
            PermissionMode::Default => {
                cmd.arg("--permission-mode").arg("default");
            }
            PermissionMode::AcceptEdits => {
                cmd.arg("--permission-mode").arg("acceptEdits");
            }
            PermissionMode::BypassPermissions => {
                cmd.arg("--permission-mode").arg("bypassPermissions");
            }
        }

        // Model
        if let Some(ref model) = self.options.model {
            cmd.arg("--model").arg(model);
        }
        
        // Permission prompt tool
        if let Some(ref tool_name) = self.options.permission_prompt_tool_name {
            cmd.arg("--permission-prompt-tool").arg(tool_name);
        }

        // Max turns
        if let Some(max_turns) = self.options.max_turns {
            cmd.arg("--max-turns").arg(max_turns.to_string());
        }

        // Note: max_thinking_tokens is not currently supported by Claude CLI

        // Working directory
        if let Some(ref cwd) = self.options.cwd {
            cmd.current_dir(cwd);
        }

        // MCP servers - use --mcp-config with JSON format like Python SDK
        if !self.options.mcp_servers.is_empty() {
            let mcp_config = serde_json::json!({
                "mcpServers": self.options.mcp_servers
            });
            cmd.arg("--mcp-config").arg(mcp_config.to_string());
        }

        // Continue/resume
        if self.options.continue_conversation {
            cmd.arg("--continue");
        }
        if let Some(ref resume_id) = self.options.resume {
            cmd.arg("--resume").arg(resume_id);
        }

        // Set up process pipes
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Set environment variable to indicate SDK usage
        cmd.env("CLAUDE_CODE_ENTRYPOINT", "sdk-rust");

        cmd
    }

    /// Spawn the process and set up communication channels
    async fn spawn_process(&mut self) -> Result<()> {
        self.state = TransportState::Connecting;

        let mut cmd = self.build_command();
        info!("Starting Claude CLI with command: {:?}", cmd);

        let mut child = cmd.spawn().map_err(|e| {
            error!("Failed to spawn Claude CLI: {}", e);
            SdkError::ProcessError(e)
        })?;

        // Get stdio handles
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| SdkError::ConnectionError("Failed to get stdin".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| SdkError::ConnectionError("Failed to get stdout".into()))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| SdkError::ConnectionError("Failed to get stderr".into()))?;

        // Create channels
        let (stdin_tx, mut stdin_rx) = mpsc::channel::<String>(CHANNEL_BUFFER_SIZE);
        // Use broadcast channel for messages to support multiple receivers
        let (message_broadcast_tx, _) = tokio::sync::broadcast::channel::<Message>(CHANNEL_BUFFER_SIZE);
        let (control_tx, control_rx) = mpsc::channel::<ControlResponse>(CHANNEL_BUFFER_SIZE);

        // Spawn stdin handler
        tokio::spawn(async move {
            let mut stdin = stdin;
            debug!("Stdin handler started");
            while let Some(line) = stdin_rx.recv().await {
                debug!("Received line from channel: {}", line);
                if let Err(e) = stdin.write_all(line.as_bytes()).await {
                    error!("Failed to write to stdin: {}", e);
                    break;
                }
                if let Err(e) = stdin.write_all(b"\n").await {
                    error!("Failed to write newline: {}", e);
                    break;
                }
                if let Err(e) = stdin.flush().await {
                    error!("Failed to flush stdin: {}", e);
                    break;
                }
                debug!("Successfully sent to Claude process: {}", line);
            }
            debug!("Stdin handler ended");
        });

        // Spawn stdout handler
        let message_broadcast_tx_clone = message_broadcast_tx.clone();
        let control_tx_clone = control_tx.clone();
        tokio::spawn(async move {
            debug!("Stdout handler started");
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();

            while let Ok(Some(line)) = lines.next_line().await {
                if line.trim().is_empty() {
                    continue;
                }

                debug!("Claude output: {}", line);

                // Try to parse as JSON
                match serde_json::from_str::<serde_json::Value>(&line) {
                    Ok(json) => {
                        // Check if it's a control response
                        if let Some(msg_type) = json.get("type").and_then(|v| v.as_str()) {
                            if msg_type == "control_response" {
                                if let Ok(control_resp) =
                                    serde_json::from_value::<ControlResponse>(json.clone())
                                {
                                    let _ = control_tx_clone.send(control_resp).await;
                                    continue;
                                }
                            }
                        }

                        // Try to parse as a message
                        match crate::message_parser::parse_message(json) {
                            Ok(Some(message)) => {
                                // Use broadcast send which doesn't fail if no receivers
                                let _ = message_broadcast_tx_clone.send(message);
                            }
                            Ok(None) => {
                                // Ignore non-message JSON
                            }
                            Err(e) => {
                                warn!("Failed to parse message: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Failed to parse JSON: {} - Line: {}", e, line);
                    }
                }
            }
            info!("Stdout reader ended");
        });

        // Spawn stderr handler
        tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();

            while let Ok(Some(line)) = lines.next_line().await {
                if !line.trim().is_empty() {
                    warn!("Claude stderr: {}", line);
                }
            }
        });

        // Store handles
        self.child = Some(child);
        self.stdin_tx = Some(stdin_tx);
        self.message_broadcast_tx = Some(message_broadcast_tx);
        self.control_rx = Some(control_rx);
        self.state = TransportState::Connected;

        Ok(())
    }
}

#[async_trait]
impl Transport for SubprocessTransport {
    async fn connect(&mut self) -> Result<()> {
        if self.state == TransportState::Connected {
            return Ok(());
        }

        self.spawn_process().await?;
        info!("Connected to Claude CLI");
        Ok(())
    }

    async fn send_message(&mut self, message: InputMessage) -> Result<()> {
        if self.state != TransportState::Connected {
            return Err(SdkError::InvalidState {
                message: "Not connected".into(),
            });
        }

        let json = serde_json::to_string(&message)?;
        debug!("Serialized message: {}", json);
        
        if let Some(ref tx) = self.stdin_tx {
            debug!("Sending message to stdin channel");
            tx.send(json).await?;
            debug!("Message sent to channel");
            Ok(())
        } else {
            Err(SdkError::InvalidState {
                message: "Stdin channel not available".into(),
            })
        }
    }

    fn receive_messages(&mut self) -> Pin<Box<dyn Stream<Item = Result<Message>> + Send + '_>> {
        if let Some(ref tx) = self.message_broadcast_tx {
            // Create a new receiver from the broadcast sender
            let rx = tx.subscribe();
            // Convert broadcast receiver to stream
            Box::pin(tokio_stream::wrappers::BroadcastStream::new(rx).filter_map(|result| async move {
                match result {
                    Ok(msg) => Some(Ok(msg)),
                    Err(tokio_stream::wrappers::errors::BroadcastStreamRecvError::Lagged(n)) => {
                        warn!("Receiver lagged by {} messages", n);
                        None
                    }
                }
            }))
        } else {
            Box::pin(futures::stream::empty())
        }
    }

    async fn send_control_request(&mut self, request: ControlRequest) -> Result<()> {
        if self.state != TransportState::Connected {
            return Err(SdkError::InvalidState {
                message: "Not connected".into(),
            });
        }

        self.request_counter += 1;
        let control_msg = match request {
            ControlRequest::Interrupt { request_id } => {
                serde_json::json!({
                    "type": "control_request",
                    "request": {
                        "type": "interrupt",
                        "request_id": request_id
                    }
                })
            }
        };

        let json = serde_json::to_string(&control_msg)?;
        
        if let Some(ref tx) = self.stdin_tx {
            tx.send(json).await?;
            Ok(())
        } else {
            Err(SdkError::InvalidState {
                message: "Stdin channel not available".into(),
            })
        }
    }

    async fn receive_control_response(&mut self) -> Result<Option<ControlResponse>> {
        if let Some(ref mut rx) = self.control_rx {
            Ok(rx.recv().await)
        } else {
            Ok(None)
        }
    }

    fn is_connected(&self) -> bool {
        self.state == TransportState::Connected
    }

    async fn disconnect(&mut self) -> Result<()> {
        if self.state != TransportState::Connected {
            return Ok(());
        }

        self.state = TransportState::Disconnecting;

        // Close stdin channel
        self.stdin_tx.take();

        // Kill the child process
        if let Some(mut child) = self.child.take() {
            match child.kill().await {
                Ok(()) => info!("Claude CLI process terminated"),
                Err(e) => warn!("Failed to kill Claude CLI process: {}", e),
            }
        }

        self.state = TransportState::Disconnected;
        Ok(())
    }
}

impl Drop for SubprocessTransport {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            // Try to kill the process
            let _ = child.start_kill();
        }
    }
}

/// Find the Claude CLI binary
pub(crate) fn find_claude_cli() -> Result<PathBuf> {
    // First check if it's in PATH
    if let Ok(path) = which::which("claude") {
        return Ok(path);
    }

    // Check common installation locations
    let home = dirs::home_dir().ok_or_else(|| {
        SdkError::CliNotFound {
            searched_paths: "Unable to determine home directory".into(),
        }
    })?;

    let locations = vec![
        home.join(".npm-global/bin/claude"),
        PathBuf::from("/usr/local/bin/claude"),
        home.join(".local/bin/claude"),
        home.join("node_modules/.bin/claude"),
        home.join(".yarn/bin/claude"),
    ];

    let mut searched = Vec::new();
    for path in &locations {
        searched.push(path.display().to_string());
        if path.exists() && path.is_file() {
            return Ok(path.clone());
        }
    }

    // Check if Node.js is installed
    if which::which("node").is_err() {
        return Err(SdkError::CliNotFound {
            searched_paths: format!(
                "Node.js is not installed. Install from https://nodejs.org/\n\nSearched in:\n{}",
                searched.join("\n")
            ),
        });
    }

    Err(SdkError::CliNotFound {
        searched_paths: format!(
            "Claude CLI not found. Install with:\n  npm install -g @anthropic-ai/claude-code\n\nSearched in:\n{}",
            searched.join("\n")
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_claude_cli_error_message() {
        // Test error message format without relying on CLI not being found
        let error = SdkError::CliNotFound {
            searched_paths: "test paths".to_string(),
        };
        let error_msg = error.to_string();
        assert!(error_msg.contains("npm install -g @anthropic-ai/claude-code"));
        assert!(error_msg.contains("test paths"));
    }

    #[tokio::test]
    async fn test_transport_lifecycle() {
        let options = ClaudeCodeOptions::default();
        let transport = SubprocessTransport::new(options).unwrap_or_else(|_| {
            // Use a dummy path for testing
            SubprocessTransport::with_cli_path(ClaudeCodeOptions::default(), "/usr/bin/true")
        });

        assert!(!transport.is_connected());
        assert_eq!(transport.state, TransportState::Disconnected);
    }
}