//! CLI Launcher for WebSocket sessions
//!
//! Spawns Claude Code CLI processes with `--sdk-url` pointed at the
//! bridge's CLI WebSocket endpoint. Monitors process lifecycle.

use serde::Serialize;
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use tokio::process::Command;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Info about a launched CLI process
#[derive(Debug, Clone, Serialize)]
pub struct WsSessionInfo {
    pub session_id: String,
    pub pid: Option<u32>,
    pub model: Option<String>,
    pub cwd: Option<String>,
    pub permission_mode: Option<String>,
    pub is_running: bool,
}

/// Launches and manages CLI processes that connect via WebSocket
pub struct WsCliLauncher {
    sessions: Arc<RwLock<HashMap<String, WsSessionInfo>>>,
    claude_command: String,
    server_port: u16,
}

impl WsCliLauncher {
    /// Create a new launcher
    pub fn new(claude_command: String, server_port: u16) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            claude_command,
            server_port,
        }
    }

    /// Launch a new CLI process for a session.
    ///
    /// The CLI will connect back to `ws://localhost:{port}/ws/cli/{session_id}`
    /// and communicate via NDJSON over the WebSocket.
    pub async fn launch(
        &self,
        session_id: &str,
        model: Option<&str>,
        cwd: Option<&str>,
        permission_mode: Option<&str>,
        resume_session_id: Option<&str>,
    ) -> anyhow::Result<u32> {
        let sdk_url = format!(
            "ws://127.0.0.1:{}/ws/cli/{session_id}",
            self.server_port
        );

        let mut cmd = Command::new(&self.claude_command);

        // Required flags for WebSocket + NDJSON mode
        cmd.arg("--sdk-url")
            .arg(&sdk_url)
            .arg("--print")
            .arg("--output-format")
            .arg("stream-json")
            .arg("--input-format")
            .arg("stream-json")
            .arg("--verbose")
            .arg("-p")
            .arg(""); // placeholder prompt (ignored when --sdk-url is used)

        // Optional: model override
        if let Some(m) = model {
            cmd.arg("--model").arg(m);
        }

        // Optional: permission mode
        if let Some(mode) = permission_mode {
            cmd.arg("--permission-mode").arg(mode);
        }

        // Optional: resume a previous CLI session
        if let Some(resume_id) = resume_session_id {
            cmd.arg("--resume").arg(resume_id);
        }

        // Optional: working directory
        if let Some(dir) = cwd {
            cmd.current_dir(dir);
        }

        // stdin is null (all communication via WebSocket)
        // stdout/stderr are piped for debug logging
        cmd.stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        info!("Launching CLI for session {session_id}: {sdk_url}");
        debug!("CLI command: {:?}", cmd);

        let mut child = cmd.spawn().map_err(|e| {
            anyhow::anyhow!("Failed to spawn Claude CLI: {e}")
        })?;

        let pid = child.id();
        info!(
            "CLI process launched for session {session_id}, pid={pid:?}"
        );

        // Record session info
        let info = WsSessionInfo {
            session_id: session_id.to_string(),
            pid,
            model: model.map(String::from),
            cwd: cwd.map(String::from),
            permission_mode: permission_mode.map(String::from),
            is_running: true,
        };
        self.sessions
            .write()
            .await
            .insert(session_id.to_string(), info);

        // Spawn a task to monitor process exit
        let sessions = self.sessions.clone();
        let session_id_owned = session_id.to_string();

        // Capture stdout/stderr for debug logging
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        tokio::spawn(async move {
            // Log stdout in background
            if let Some(stdout) = stdout {
                let sid = session_id_owned.clone();
                tokio::spawn(async move {
                    use tokio::io::{AsyncBufReadExt, BufReader};
                    let reader = BufReader::new(stdout);
                    let mut lines = reader.lines();
                    while let Ok(Some(line)) = lines.next_line().await {
                        debug!("[CLI stdout {sid}] {line}");
                    }
                });
            }

            // Log stderr in background
            if let Some(stderr) = stderr {
                let sid = session_id_owned.clone();
                tokio::spawn(async move {
                    use tokio::io::{AsyncBufReadExt, BufReader};
                    let reader = BufReader::new(stderr);
                    let mut lines = reader.lines();
                    while let Ok(Some(line)) = lines.next_line().await {
                        warn!("[CLI stderr {sid}] {line}");
                    }
                });
            }

            // Wait for process exit
            match child.wait().await {
                Ok(status) => {
                    info!(
                        "CLI process for session {session_id_owned} exited: {status}"
                    );
                }
                Err(e) => {
                    error!(
                        "Error waiting for CLI process (session {session_id_owned}): {e}"
                    );
                }
            }

            // Mark session as not running
            if let Some(info) = sessions.write().await.get_mut(&session_id_owned) {
                info.is_running = false;
            }
        });

        Ok(pid.unwrap_or(0))
    }

    /// Kill a CLI process for a session.
    ///
    /// Uses `kill -SIGTERM` via std::process::Command for cross-platform support
    /// without requiring the `libc` crate.
    pub async fn kill(&self, session_id: &str) -> anyhow::Result<()> {
        let info = self.sessions.read().await.get(session_id).cloned();
        if let Some(info) = info {
            if let Some(pid) = info.pid {
                // Use the `kill` command on Unix to send SIGTERM
                #[cfg(unix)]
                {
                    let _ = std::process::Command::new("kill")
                        .arg("-TERM")
                        .arg(pid.to_string())
                        .status();
                    info!("Sent SIGTERM to CLI process {pid} for session {session_id}");
                }
                #[cfg(not(unix))]
                {
                    let _ = std::process::Command::new("taskkill")
                        .args(["/PID", &pid.to_string(), "/F"])
                        .status();
                    info!("Sent taskkill to CLI process {pid} for session {session_id}");
                }
            }
        }
        // Remove from tracking
        self.sessions.write().await.remove(session_id);
        Ok(())
    }

    /// Get info about a session's CLI process
    pub async fn get_session_info(&self, session_id: &str) -> Option<WsSessionInfo> {
        self.sessions.read().await.get(session_id).cloned()
    }

    /// List all launched sessions
    pub async fn list_sessions(&self) -> Vec<WsSessionInfo> {
        self.sessions.read().await.values().cloned().collect()
    }
}
