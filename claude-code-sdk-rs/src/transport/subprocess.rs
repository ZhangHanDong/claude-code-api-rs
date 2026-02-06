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
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

/// Default buffer size for channels
const CHANNEL_BUFFER_SIZE: usize = 100;

/// Minimum required CLI version
const MIN_CLI_VERSION: (u32, u32, u32) = (2, 0, 0);

/// Simple semantic version struct
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct SemVer {
    major: u32,
    minor: u32,
    patch: u32,
}

impl SemVer {
    fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// Parse semantic version from string (e.g., "2.0.0" or "v2.0.0")
    fn parse(version: &str) -> Option<Self> {
        let version = version.trim().trim_start_matches('v');

        // Handle versions like "@anthropic-ai/claude-code/2.0.0"
        let version = if let Some(v) = version.split('/').next_back() {
            v
        } else {
            version
        };

        let parts: Vec<&str> = version.split('.').collect();
        if parts.len() < 2 {
            return None;
        }

        Some(Self {
            major: parts[0].parse().ok()?,
            minor: parts.get(1)?.parse().ok()?,
            patch: parts.get(2).and_then(|p| p.parse().ok()).unwrap_or(0),
        })
    }
}

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
    /// Receiver for SDK control requests
    sdk_control_rx: Option<mpsc::Receiver<serde_json::Value>>,
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
            sdk_control_rx: None,
            state: TransportState::Disconnected,
            request_counter: 0,
            close_stdin_after_prompt: false,
        })
    }

    /// Create a new subprocess transport with async initialization
    ///
    /// This version supports auto-downloading the CLI if `auto_download_cli` is enabled
    /// in the options and the CLI is not found.
    pub async fn new_async(options: ClaudeCodeOptions) -> Result<Self> {
        let cli_path = match find_claude_cli() {
            Ok(path) => path,
            Err(_) if options.auto_download_cli => {
                info!("Claude CLI not found, attempting automatic download...");
                crate::cli_download::download_cli(None, None).await?
            },
            Err(e) => return Err(e),
        };

        Ok(Self {
            options,
            cli_path,
            child: None,
            stdin_tx: None,
            message_broadcast_tx: None,
            control_rx: None,
            sdk_control_rx: None,
            state: TransportState::Disconnected,
            request_counter: 0,
            close_stdin_after_prompt: false,
        })
    }

    fn build_settings_value(&self) -> Option<String> {
        let has_settings = self.options.settings.is_some();
        let has_sandbox = self.options.sandbox.is_some();

        if !has_settings && !has_sandbox {
            return None;
        }

        // If only settings path and no sandbox, pass through as-is
        if has_settings && !has_sandbox {
            return self.options.settings.clone();
        }

        // If we have sandbox settings, merge into a JSON object (Python parity)
        let mut settings_obj = serde_json::Map::new();

        if let Some(ref settings) = self.options.settings {
            let settings_str = settings.trim();

            let load_as_json_string =
                |s: &str| -> Option<serde_json::Map<String, serde_json::Value>> {
                    match serde_json::from_str::<serde_json::Value>(s) {
                        Ok(serde_json::Value::Object(map)) => Some(map),
                        Ok(_) => {
                            warn!(
                                "Settings JSON must be an object; ignoring provided JSON settings"
                            );
                            None
                        },
                        Err(_) => None,
                    }
                };

            let load_from_file =
                |path: &Path| -> Option<serde_json::Map<String, serde_json::Value>> {
                    let content = std::fs::read_to_string(path).ok()?;
                    match serde_json::from_str::<serde_json::Value>(&content) {
                        Ok(serde_json::Value::Object(map)) => Some(map),
                        Ok(_) => {
                            warn!("Settings file JSON must be an object: {}", path.display());
                            None
                        },
                        Err(e) => {
                            warn!("Failed to parse settings file {}: {}", path.display(), e);
                            None
                        },
                    }
                };

            if settings_str.starts_with('{') && settings_str.ends_with('}') {
                if let Some(map) = load_as_json_string(settings_str) {
                    settings_obj = map;
                } else {
                    warn!(
                        "Failed to parse settings as JSON, treating as file path: {}",
                        settings_str
                    );
                    let settings_path = Path::new(settings_str);
                    if settings_path.exists() {
                        if let Some(map) = load_from_file(settings_path) {
                            settings_obj = map;
                        }
                    } else {
                        warn!("Settings file not found: {}", settings_path.display());
                    }
                }
            } else {
                let settings_path = Path::new(settings_str);
                if settings_path.exists() {
                    if let Some(map) = load_from_file(settings_path) {
                        settings_obj = map;
                    }
                } else {
                    warn!("Settings file not found: {}", settings_path.display());
                }
            }
        }

        if let Some(ref sandbox) = self.options.sandbox {
            match serde_json::to_value(sandbox) {
                Ok(value) => {
                    settings_obj.insert("sandbox".to_string(), value);
                },
                Err(e) => {
                    warn!("Failed to serialize sandbox settings: {}", e);
                },
            }
        }

        Some(serde_json::Value::Object(settings_obj).to_string())
    }

    /// Subscribe to messages without borrowing self (for lock-free consumption)
    pub fn subscribe_messages(
        &self,
    ) -> Option<Pin<Box<dyn Stream<Item = Result<Message>> + Send + 'static>>> {
        self.message_broadcast_tx.as_ref().map(|tx| {
            let rx = tx.subscribe();
            Box::pin(tokio_stream::wrappers::BroadcastStream::new(rx).filter_map(
                |result| async move {
                    match result {
                        Ok(msg) => Some(Ok(msg)),
                        Err(tokio_stream::wrappers::errors::BroadcastStreamRecvError::Lagged(
                            n,
                        )) => {
                            warn!("Receiver lagged by {} messages", n);
                            None
                        },
                    }
                },
            )) as Pin<Box<dyn Stream<Item = Result<Message>> + Send + 'static>>
        })
    }

    /// Receive SDK control requests
    #[allow(dead_code)]
    pub async fn receive_sdk_control_request(&mut self) -> Option<serde_json::Value> {
        if let Some(ref mut rx) = self.sdk_control_rx {
            rx.recv().await
        } else {
            None
        }
    }

    /// Take the SDK control receiver (can only be called once)
    pub fn take_sdk_control_receiver(&mut self) -> Option<mpsc::Receiver<serde_json::Value>> {
        self.sdk_control_rx.take()
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
            sdk_control_rx: None,
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
            sdk_control_rx: None,
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

        // Include partial messages if requested
        if self.options.include_partial_messages {
            cmd.arg("--include-partial-messages");
        }

        // Add debug-to-stderr flag if debug_stderr is set
        if self.options.debug_stderr.is_some() {
            cmd.arg("--debug-to-stderr");
        }

        // Handle max_output_tokens (priority: option > env var)
        // Maximum safe value is 32000, values above this may cause issues
        if let Some(max_tokens) = self.options.max_output_tokens {
            // Option takes priority - validate and cap at 32000
            let capped = max_tokens.clamp(1, 32000);
            cmd.env("CLAUDE_CODE_MAX_OUTPUT_TOKENS", capped.to_string());
            debug!("Setting max_output_tokens from option: {}", capped);
        } else {
            // Fall back to environment variable handling
            if let Ok(current_value) = std::env::var("CLAUDE_CODE_MAX_OUTPUT_TOKENS") {
                if let Ok(tokens) = current_value.parse::<u32>() {
                    if tokens > 32000 {
                        warn!(
                            "CLAUDE_CODE_MAX_OUTPUT_TOKENS={} exceeds maximum safe value of 32000, overriding to 32000",
                            tokens
                        );
                        cmd.env("CLAUDE_CODE_MAX_OUTPUT_TOKENS", "32000");
                    }
                    // If it's <= 32000, leave it as is
                } else {
                    // Invalid value, set to safe default
                    warn!(
                        "Invalid CLAUDE_CODE_MAX_OUTPUT_TOKENS value: {}, setting to 8192",
                        current_value
                    );
                    cmd.env("CLAUDE_CODE_MAX_OUTPUT_TOKENS", "8192");
                }
            }
        }

        // System prompts (match Python SDK behavior)
        //
        // Python always passes `--system-prompt ""` when `system_prompt` is None.
        if let Some(ref prompt_v2) = self.options.system_prompt_v2 {
            match prompt_v2 {
                crate::types::SystemPrompt::String(s) => {
                    cmd.arg("--system-prompt").arg(s);
                },
                crate::types::SystemPrompt::Preset { append, .. } => {
                    // Python only uses preset prompts to optionally append to the default preset.
                    // It does not pass a preset selector flag to the CLI.
                    if let Some(append_text) = append {
                        cmd.arg("--append-system-prompt").arg(append_text);
                    }
                },
            }
        } else {
            // Fallback to deprecated fields for backward compatibility
            #[allow(deprecated)]
            match self.options.system_prompt.as_deref() {
                Some(prompt) => {
                    cmd.arg("--system-prompt").arg(prompt);
                },
                None => {
                    cmd.arg("--system-prompt").arg("");
                },
            }
            #[allow(deprecated)]
            if let Some(ref prompt) = self.options.append_system_prompt {
                cmd.arg("--append-system-prompt").arg(prompt);
            }
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
            },
            PermissionMode::AcceptEdits => {
                cmd.arg("--permission-mode").arg("acceptEdits");
            },
            PermissionMode::Plan => {
                cmd.arg("--permission-mode").arg("plan");
            },
            PermissionMode::BypassPermissions => {
                cmd.arg("--permission-mode").arg("bypassPermissions");
            },
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

        // Max thinking tokens (extended thinking budget)
        // Only pass if non-zero to match Python SDK behavior
        if self.options.max_thinking_tokens > 0 {
            cmd.arg("--max-thinking-tokens")
                .arg(self.options.max_thinking_tokens.to_string());
        }

        // Working directory
        if let Some(ref cwd) = self.options.cwd {
            cmd.current_dir(cwd);
        }

        // Add environment variables
        for (key, value) in &self.options.env {
            cmd.env(key, value);
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

        // Settings value (merge sandbox into settings if provided)
        if let Some(settings_value) = self.build_settings_value() {
            cmd.arg("--settings").arg(settings_value);
        }

        // Additional directories
        for dir in &self.options.add_dirs {
            cmd.arg("--add-dir").arg(dir);
        }

        // Fork session if requested
        if self.options.fork_session {
            cmd.arg("--fork-session");
        }

        // ========== Phase 3 CLI args (Python SDK v0.1.12+ sync) ==========

        // Tools configuration (base set of tools)
        if let Some(ref tools) = self.options.tools {
            match tools {
                crate::types::ToolsConfig::List(list) => {
                    if list.is_empty() {
                        cmd.arg("--tools").arg("");
                    } else {
                        cmd.arg("--tools").arg(list.join(","));
                    }
                },
                crate::types::ToolsConfig::Preset(_preset) => {
                    // Preset object - 'claude_code' preset maps to 'default'
                    cmd.arg("--tools").arg("default");
                },
            }
        }

        // SDK betas
        if !self.options.betas.is_empty() {
            let betas: Vec<String> = self.options.betas.iter().map(|b| b.to_string()).collect();
            cmd.arg("--betas").arg(betas.join(","));
        }

        // Max budget USD
        if let Some(budget) = self.options.max_budget_usd {
            cmd.arg("--max-budget-usd").arg(budget.to_string());
        }

        // Fallback model
        if let Some(ref fallback) = self.options.fallback_model {
            cmd.arg("--fallback-model").arg(fallback);
        }

        // File checkpointing
        if self.options.enable_file_checkpointing {
            cmd.env("CLAUDE_CODE_ENABLE_SDK_FILE_CHECKPOINTING", "true");
        }

        // Output format for structured outputs (json_schema only)
        if let Some(ref format) = self.options.output_format
            && format.get("type").and_then(|v| v.as_str()) == Some("json_schema")
            && let Some(schema) = format.get("schema")
            && let Ok(schema_json) = serde_json::to_string(schema)
        {
            cmd.arg("--json-schema").arg(schema_json);
        }

        // Plugin directories
        for plugin in &self.options.plugins {
            match plugin {
                crate::types::SdkPluginConfig::Local { path } => {
                    cmd.arg("--plugin-dir").arg(path);
                },
            }
        }

        // Programmatic agents
        if let Some(ref agents) = self.options.agents
            && !agents.is_empty()
            && let Ok(json_str) = serde_json::to_string(agents)
        {
            cmd.arg("--agents").arg(json_str);
        }

        // Setting sources (comma-separated). Always pass a value for SDK parity with Python.
        let sources_value = self
            .options
            .setting_sources
            .as_ref()
            .map(|sources| {
                sources
                    .iter()
                    .map(|s| match s {
                        crate::types::SettingSource::User => "user",
                        crate::types::SettingSource::Project => "project",
                        crate::types::SettingSource::Local => "local",
                    })
                    .collect::<Vec<_>>()
                    .join(",")
            })
            .unwrap_or_default();
        cmd.arg("--setting-sources").arg(sources_value);

        // Extra arguments
        for (key, value) in &self.options.extra_args {
            let flag = if key.starts_with("--") || key.starts_with("-") {
                key.clone()
            } else {
                format!("--{key}")
            };
            cmd.arg(&flag);
            if let Some(val) = value {
                cmd.arg(val);
            }
        }

        // Set up process pipes
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Set environment variables to indicate SDK usage and version
        cmd.env("CLAUDE_CODE_ENTRYPOINT", "sdk-rust");
        cmd.env("CLAUDE_AGENT_SDK_VERSION", env!("CARGO_PKG_VERSION"));

        // Debug log the full command being executed
        debug!(
            "Executing Claude CLI command: {} {:?}",
            self.cli_path.display(),
            cmd.as_std().get_args().collect::<Vec<_>>()
        );

        cmd
    }

    /// Check CLI version and warn if below minimum required version
    async fn check_cli_version(&self) -> Result<()> {
        // Run the CLI with --version flag (with a timeout to avoid hanging)
        let output = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            tokio::process::Command::new(&self.cli_path)
                .arg("--version")
                .output(),
        )
        .await;

        let output = match output {
            Ok(Ok(output)) => output,
            Ok(Err(e)) => {
                warn!("Failed to check CLI version: {}", e);
                return Ok(()); // Don't fail connection, just warn
            },
            Err(_) => {
                warn!("CLI version check timed out after 5 seconds");
                return Ok(());
            },
        };

        let version_str = String::from_utf8_lossy(&output.stdout);
        let version_str = version_str.trim();

        if let Some(semver) = SemVer::parse(version_str) {
            let min_version = SemVer::new(MIN_CLI_VERSION.0, MIN_CLI_VERSION.1, MIN_CLI_VERSION.2);

            if semver < min_version {
                warn!(
                    "⚠️  Claude CLI version {}.{}.{} is below minimum required version {}.{}.{}",
                    semver.major,
                    semver.minor,
                    semver.patch,
                    MIN_CLI_VERSION.0,
                    MIN_CLI_VERSION.1,
                    MIN_CLI_VERSION.2
                );
                warn!(
                    "   Some features may not work correctly. Please upgrade with: npm install -g @anthropic-ai/claude-code@latest"
                );
            } else {
                info!(
                    "Claude CLI version: {}.{}.{}",
                    semver.major, semver.minor, semver.patch
                );
            }
        } else {
            debug!("Could not parse CLI version: {}", version_str);
        }

        Ok(())
    }

    /// Spawn the process and set up communication channels
    async fn spawn_process(&mut self) -> Result<()> {
        self.state = TransportState::Connecting;

        let mut cmd = self.build_command();
        info!("Starting Claude CLI with command: {:?}", cmd);

        if let Some(user) = self.options.user.as_deref() {
            apply_process_user(&mut cmd, user)?;
        }

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

        // Determine buffer size from options or use default
        let buffer_size = self
            .options
            .cli_channel_buffer_size
            .unwrap_or(CHANNEL_BUFFER_SIZE);

        // Create channels
        let (stdin_tx, mut stdin_rx) = mpsc::channel::<String>(buffer_size);
        // Use broadcast channel for messages to support multiple receivers
        let (message_broadcast_tx, _) = tokio::sync::broadcast::channel::<Message>(buffer_size);
        let (control_tx, control_rx) = mpsc::channel::<ControlResponse>(buffer_size);

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

        // Create channel for SDK control requests
        let (sdk_control_tx, sdk_control_rx) = mpsc::channel::<serde_json::Value>(buffer_size);

        // Spawn stdout handler
        let message_broadcast_tx_clone = message_broadcast_tx.clone();
        let control_tx_clone = control_tx.clone();
        let sdk_control_tx_clone = sdk_control_tx.clone();
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
                        // Check message type
                        if let Some(msg_type) = json.get("type").and_then(|v| v.as_str()) {
                            // Handle control responses - these are responses to OUR control requests
                            if msg_type == "control_response" {
                                debug!("Received control response: {:?}", json);

                                // Send to sdk_control channel for control protocol mode
                                let _ = sdk_control_tx_clone.send(json.clone()).await;

                                // Also parse and send to legacy control_tx for non-control-protocol mode
                                // (needed for interrupt functionality when query_handler is None)
                                // CLI returns: {"type":"control_response","response":{"subtype":"success","request_id":"..."}}
                                // or: {"type":"control_response","response":{"subtype":"error","request_id":"...","error":"..."}}
                                if let Some(response_obj) = json.get("response")
                                    && let Some(request_id) = response_obj
                                        .get("request_id")
                                        .or_else(|| response_obj.get("requestId"))
                                        .and_then(|v| v.as_str())
                                {
                                    // Determine success from subtype
                                    let subtype =
                                        response_obj.get("subtype").and_then(|v| v.as_str());
                                    let success = subtype == Some("success");

                                    let control_resp = ControlResponse::InterruptAck {
                                        request_id: request_id.to_string(),
                                        success,
                                    };
                                    let _ = control_tx_clone.send(control_resp).await;
                                }
                                continue;
                            }

                            // Handle control requests FROM CLI (standard format)
                            if msg_type == "control_request" {
                                debug!("Received control request from CLI: {:?}", json);
                                // Send the FULL message including requestId and request
                                let _ = sdk_control_tx_clone.send(json.clone()).await;
                                continue;
                            }

                            // Handle control messages (new format)
                            if msg_type == "control"
                                && let Some(control) = json.get("control")
                            {
                                debug!("Received control message: {:?}", control);
                                let _ = sdk_control_tx_clone.send(control.clone()).await;
                                continue;
                            }

                            // Handle SDK control requests FROM CLI (legacy format)
                            if msg_type == "sdk_control_request" {
                                // Send the FULL message including requestId
                                debug!("Received SDK control request (legacy): {:?}", json);
                                let _ = sdk_control_tx_clone.send(json.clone()).await;
                                continue;
                            }

                            // Check for system messages with SDK control subtypes
                            if msg_type == "system"
                                && let Some(subtype) = json.get("subtype").and_then(|v| v.as_str())
                                && subtype.starts_with("sdk_control:")
                            {
                                // This is an SDK control message
                                debug!("Received SDK control message: {}", subtype);
                                let _ = sdk_control_tx_clone.send(json.clone()).await;
                                // Still parse as regular message for now
                            }
                        }

                        // Try to parse as a regular message
                        match crate::message_parser::parse_message(json) {
                            Ok(Some(message)) => {
                                // Use broadcast send which doesn't fail if no receivers
                                let _ = message_broadcast_tx_clone.send(message);
                            },
                            Ok(None) => {
                                // Ignore non-message JSON
                            },
                            Err(e) => {
                                warn!("Failed to parse message: {}", e);
                            },
                        }
                    },
                    Err(e) => {
                        warn!("Failed to parse JSON: {} - Line: {}", e, line);
                    },
                }
            }
            info!("Stdout reader ended");
        });

        // Spawn stderr handler - capture error messages for better diagnostics
        let message_broadcast_tx_for_error = message_broadcast_tx.clone();
        let debug_stderr = self.options.debug_stderr.clone();
        let stderr_callback = self.options.stderr_callback.clone();
        tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            let mut error_buffer = Vec::new();

            while let Ok(Some(line)) = lines.next_line().await {
                if !line.trim().is_empty() {
                    // If debug_stderr is set, write to it
                    if let Some(ref debug_output) = debug_stderr {
                        let mut output = debug_output.lock().await;
                        let _ = writeln!(output, "{line}");
                        let _ = output.flush();
                    }

                    if let Some(ref callback) = stderr_callback {
                        callback.as_ref()(line.as_str());
                    }

                    error!("Claude CLI stderr: {}", line);
                    error_buffer.push(line.clone());

                    // Check for common error patterns
                    if line.contains("command not found") || line.contains("No such file") {
                        error!("Claude CLI binary not found or not executable");
                    } else if line.contains("ENOENT") || line.contains("spawn") {
                        error!("Failed to spawn Claude CLI process - binary may not be installed");
                    } else if line.contains("authentication")
                        || line.contains("API key")
                        || line.contains("Unauthorized")
                    {
                        error!(
                            "Claude CLI authentication error - please run 'claude-code api login'"
                        );
                    } else if line.contains("model")
                        && (line.contains("not available") || line.contains("not found"))
                    {
                        error!("Model not available for your account: {}", line);
                    } else if line.contains("Error:") || line.contains("error:") {
                        error!("Claude CLI error detected: {}", line);
                    }
                }
            }

            // If we collected any errors, log them
            if !error_buffer.is_empty() {
                let error_msg = error_buffer.join("\n");
                error!("Claude CLI stderr output collected:\n{}", error_msg);

                // Try to send an error message through the broadcast channel
                let _ = message_broadcast_tx_for_error.send(Message::System {
                    subtype: "error".to_string(),
                    data: serde_json::json!({
                        "source": "stderr",
                        "error": "Claude CLI error output",
                        "details": error_msg
                    }),
                });
            }
        });

        // Store handles
        self.child = Some(child);
        self.stdin_tx = Some(stdin_tx);
        self.message_broadcast_tx = Some(message_broadcast_tx);
        self.control_rx = Some(control_rx);
        self.sdk_control_rx = Some(sdk_control_rx);
        self.state = TransportState::Connected;

        Ok(())
    }
}

#[async_trait]
impl Transport for SubprocessTransport {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    async fn connect(&mut self) -> Result<()> {
        if self.state == TransportState::Connected {
            return Ok(());
        }

        // Check CLI version before connecting
        if let Err(e) = self.check_cli_version().await {
            warn!("CLI version check failed: {}", e);
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

    fn receive_messages(
        &mut self,
    ) -> Pin<Box<dyn Stream<Item = Result<Message>> + Send + 'static>> {
        if let Some(ref tx) = self.message_broadcast_tx {
            // Create a new receiver from the broadcast sender
            let rx = tx.subscribe();
            // Convert broadcast receiver to stream
            Box::pin(tokio_stream::wrappers::BroadcastStream::new(rx).filter_map(
                |result| async move {
                    match result {
                        Ok(msg) => Some(Ok(msg)),
                        Err(tokio_stream::wrappers::errors::BroadcastStreamRecvError::Lagged(
                            n,
                        )) => {
                            warn!("Receiver lagged by {} messages", n);
                            None
                        },
                    }
                },
            ))
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
            },
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

    async fn send_sdk_control_request(&mut self, request: serde_json::Value) -> Result<()> {
        // The request is already properly formatted as {"type": "control_request", ...}
        // Just send it directly without wrapping
        let json = serde_json::to_string(&request)?;

        if let Some(ref tx) = self.stdin_tx {
            tx.send(json).await?;
            Ok(())
        } else {
            Err(SdkError::InvalidState {
                message: "Stdin channel not available".into(),
            })
        }
    }

    async fn send_sdk_control_response(&mut self, response: serde_json::Value) -> Result<()> {
        // Wrap the response in control_response format expected by CLI
        // The response should have: {"type": "control_response", "response": {...}}
        let control_response = serde_json::json!({
            "type": "control_response",
            "response": response
        });

        let json = serde_json::to_string(&control_response)?;

        if let Some(ref tx) = self.stdin_tx {
            tx.send(json).await?;
            Ok(())
        } else {
            Err(SdkError::InvalidState {
                message: "Stdin channel not available".into(),
            })
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

    fn take_sdk_control_receiver(
        &mut self,
    ) -> Option<tokio::sync::mpsc::Receiver<serde_json::Value>> {
        self.sdk_control_rx.take()
    }

    async fn end_input(&mut self) -> Result<()> {
        // Close stdin channel to signal end of input
        self.stdin_tx.take();
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
///
/// Search order:
/// 1. System PATH (`claude`, `claude-code`)
/// 2. SDK cache directory (auto-downloaded CLI)
/// 3. Common installation locations
pub fn find_claude_cli() -> Result<PathBuf> {
    // First check if it's in PATH - try both 'claude' and 'claude-code'
    for cmd_name in &["claude", "claude-code"] {
        if let Ok(path) = which::which(cmd_name) {
            debug!("Found Claude CLI in PATH at: {}", path.display());
            return Ok(path);
        }
    }

    // Check SDK cache directory (for auto-downloaded CLI)
    if let Some(cached_path) = crate::cli_download::get_cached_cli_path()
        && cached_path.exists()
        && cached_path.is_file()
    {
        debug!("Found cached Claude CLI at: {}", cached_path.display());
        return Ok(cached_path);
    }

    // Check common installation locations
    let home = dirs::home_dir().ok_or_else(|| SdkError::CliNotFound {
        searched_paths: "Unable to determine home directory".into(),
    })?;

    let locations = vec![
        // npm global installations
        home.join(".npm-global/bin/claude"),
        home.join(".npm-global/bin/claude-code"),
        PathBuf::from("/usr/local/bin/claude"),
        PathBuf::from("/usr/local/bin/claude-code"),
        // Local installations
        home.join(".local/bin/claude"),
        home.join(".local/bin/claude-code"),
        home.join("node_modules/.bin/claude"),
        home.join("node_modules/.bin/claude-code"),
        // Yarn installations
        home.join(".yarn/bin/claude"),
        home.join(".yarn/bin/claude-code"),
        // macOS specific npm location
        PathBuf::from("/opt/homebrew/bin/claude"),
        PathBuf::from("/opt/homebrew/bin/claude-code"),
        // Claude local directory
        home.join(".claude/local/claude"),
    ];

    let mut searched = Vec::new();
    for path in &locations {
        searched.push(path.display().to_string());
        if path.exists() && path.is_file() {
            debug!("Found Claude CLI at: {}", path.display());
            return Ok(path.clone());
        }
    }

    // Log detailed error information
    warn!("Claude CLI not found in any standard location");
    warn!("Searched paths: {:?}", searched);

    // Check if Node.js is installed
    if which::which("node").is_err() && which::which("npm").is_err() {
        error!("Node.js/npm not found - Claude CLI requires Node.js");
        return Err(SdkError::CliNotFound {
            searched_paths: format!(
                "Node.js is not installed. Install from https://nodejs.org/\n\n\
                Alternatively, enable auto_download_cli to automatically download the CLI:\n\
                ```rust\n\
                let options = ClaudeCodeOptions::builder()\n\
                    .auto_download_cli(true)\n\
                    .build();\n\
                ```\n\n\
                Searched in:\n{}",
                searched.join("\n")
            ),
        });
    }

    Err(SdkError::CliNotFound {
        searched_paths: format!(
            "Claude CLI not found.\n\n\
            Option 1 - Auto-download (recommended):\n\
            ```rust\n\
            let options = ClaudeCodeOptions::builder()\n\
                .auto_download_cli(true)\n\
                .build();\n\
            ```\n\n\
            Option 2 - Manual installation:\n\
            npm install -g @anthropic-ai/claude-code\n\n\
            Searched in:\n{}",
            searched.join("\n")
        ),
    })
}

pub(crate) fn apply_process_user(cmd: &mut Command, user: &str) -> Result<()> {
    let user = user.trim();
    if user.is_empty() {
        return Err(SdkError::ConfigError(
            "options.user must be a non-empty username or uid".into(),
        ));
    }

    apply_process_user_inner(cmd, user)
}

#[cfg(unix)]
fn apply_process_user_inner(cmd: &mut Command, user: &str) -> Result<()> {
    use std::ffi::CString;
    use std::mem::MaybeUninit;
    use std::os::unix::process::CommandExt;
    use std::ptr;

    fn passwd_buf_len() -> usize {
        let buf_len = unsafe { libc::sysconf(libc::_SC_GETPW_R_SIZE_MAX) };
        if buf_len <= 0 {
            16 * 1024
        } else {
            buf_len as usize
        }
    }

    fn lookup_by_name(name: &str) -> Result<(u32, u32)> {
        let name = CString::new(name)
            .map_err(|_| SdkError::ConfigError("options.user must not contain NUL bytes".into()))?;

        let mut pwd = MaybeUninit::<libc::passwd>::zeroed();
        let mut result: *mut libc::passwd = ptr::null_mut();
        let mut buf = vec![0u8; passwd_buf_len()];

        let rc = unsafe {
            libc::getpwnam_r(
                name.as_ptr(),
                pwd.as_mut_ptr(),
                buf.as_mut_ptr() as *mut libc::c_char,
                buf.len(),
                &mut result,
            )
        };
        if rc != 0 {
            return Err(SdkError::ConfigError(format!(
                "Failed to resolve options.user={}: getpwnam_r returned {}",
                name.to_string_lossy(),
                rc
            )));
        }
        if result.is_null() {
            return Err(SdkError::ConfigError(format!(
                "User not found: {}",
                name.to_string_lossy()
            )));
        }

        let pwd = unsafe { pwd.assume_init() };
        #[allow(clippy::unnecessary_cast)]
        Ok((pwd.pw_uid as u32, pwd.pw_gid as u32))
    }

    fn lookup_by_uid(uid: u32) -> Result<(u32, u32)> {
        let mut pwd = MaybeUninit::<libc::passwd>::zeroed();
        let mut result: *mut libc::passwd = ptr::null_mut();
        let mut buf = vec![0u8; passwd_buf_len()];

        let rc = unsafe {
            libc::getpwuid_r(
                uid as libc::uid_t,
                pwd.as_mut_ptr(),
                buf.as_mut_ptr() as *mut libc::c_char,
                buf.len(),
                &mut result,
            )
        };
        if rc != 0 {
            return Err(SdkError::ConfigError(format!(
                "Failed to resolve options.user={}: getpwuid_r returned {}",
                uid, rc
            )));
        }
        if result.is_null() {
            return Err(SdkError::ConfigError(format!(
                "User not found for uid: {}",
                uid
            )));
        }

        let pwd = unsafe { pwd.assume_init() };
        #[allow(clippy::unnecessary_cast)]
        Ok((pwd.pw_uid as u32, pwd.pw_gid as u32))
    }

    let (uid, gid) = match user.parse::<u32>() {
        Ok(uid) => lookup_by_uid(uid)?,
        Err(_) => lookup_by_name(user)?,
    };

    cmd.as_std_mut().uid(uid).gid(gid);
    Ok(())
}

#[cfg(not(unix))]
fn apply_process_user_inner(_cmd: &mut Command, _user: &str) -> Result<()> {
    Err(SdkError::NotSupported {
        feature: "options.user is only supported on Unix platforms".into(),
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

    #[test]
    fn test_semver_parse() {
        // Test basic version parsing
        let v = SemVer::parse("2.0.0").unwrap();
        assert_eq!(v.major, 2);
        assert_eq!(v.minor, 0);
        assert_eq!(v.patch, 0);

        // Test with 'v' prefix
        let v = SemVer::parse("v2.1.3").unwrap();
        assert_eq!(v.major, 2);
        assert_eq!(v.minor, 1);
        assert_eq!(v.patch, 3);

        // Test npm-style version
        let v = SemVer::parse("@anthropic-ai/claude-code/2.5.1").unwrap();
        assert_eq!(v.major, 2);
        assert_eq!(v.minor, 5);
        assert_eq!(v.patch, 1);

        // Test version without patch
        let v = SemVer::parse("2.1").unwrap();
        assert_eq!(v.major, 2);
        assert_eq!(v.minor, 1);
        assert_eq!(v.patch, 0);
    }

    #[test]
    fn test_semver_compare() {
        let v1 = SemVer::new(2, 0, 0);
        let v2 = SemVer::new(2, 0, 1);
        let v3 = SemVer::new(2, 1, 0);
        let v4 = SemVer::new(3, 0, 0);

        assert!(v1 < v2);
        assert!(v2 < v3);
        assert!(v3 < v4);
        assert!(v1 < v4);

        let min_version = SemVer::new(2, 0, 0);
        assert!(SemVer::new(1, 9, 9) < min_version);
        assert!(SemVer::new(2, 0, 0) >= min_version);
        assert!(SemVer::new(2, 1, 0) >= min_version);
    }
}
