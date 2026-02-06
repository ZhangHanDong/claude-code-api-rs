use anyhow::{Result, anyhow};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::mpsc;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::core::config::{FileAccessConfig, MCPConfig};
use crate::models::claude::ClaudeCodeOutput;

pub struct ClaudeProcess {
    #[allow(dead_code)]
    pub id: String,
    pub child: Option<Child>,
    #[allow(dead_code)]
    pub project_path: Option<String>,
}

pub struct ClaudeManager {
    processes: Arc<RwLock<HashMap<String, ClaudeProcess>>>,
    claude_command: String,
    #[allow(dead_code)]
    file_access_config: FileAccessConfig,
    mcp_config: MCPConfig,
}

impl ClaudeManager {
    pub fn new(
        claude_command: String,
        file_access_config: FileAccessConfig,
        mcp_config: MCPConfig,
    ) -> Self {
        Self {
            processes: Arc::new(RwLock::new(HashMap::new())),
            claude_command,
            file_access_config,
            mcp_config,
        }
    }

    #[allow(dead_code)]
    pub async fn create_interactive_session(
        &self,
        session_id: Option<String>,
        project_path: Option<String>,
        model: Option<String>,
    ) -> Result<(String, mpsc::Receiver<ClaudeCodeOutput>)> {
        let session_id = session_id.unwrap_or_else(|| Uuid::new_v4().to_string());

        let mut cmd = Command::new(&self.claude_command);
        // 交互模式，使用 stream-json 输出以支持多轮对话
        cmd.arg("--output-format")
            .arg("stream-json")
            .arg("--verbose");

        if let Some(model) = model {
            cmd.arg("--model").arg(model);
        }

        if let Some(ref path) = project_path {
            cmd.arg("--cwd").arg(path);
        }

        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        info!(
            "Starting interactive Claude session {} with command: {:?}",
            session_id, cmd
        );

        let mut child = cmd.spawn()?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow!("Failed to get stdout"))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| anyhow!("Failed to get stderr"))?;

        let (tx, rx) = mpsc::channel(100);

        tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();

            while let Ok(Some(line)) = lines.next_line().await {
                if !line.trim().is_empty() {
                    warn!("Claude stderr: {}", line);
                }
            }
        });

        let tx_clone = tx.clone();
        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();

            while let Ok(Some(line)) = lines.next_line().await {
                if line.trim().is_empty() {
                    continue;
                }

                match serde_json::from_str::<serde_json::Value>(&line) {
                    Ok(json) => {
                        // 转换为 ClaudeCodeOutput 格式
                        let output = ClaudeCodeOutput {
                            r#type: json
                                .get("type")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown")
                                .to_string(),
                            subtype: json
                                .get("subtype")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string()),
                            data: json,
                        };

                        if tx_clone.send(output).await.is_err() {
                            break;
                        }
                    },
                    Err(e) => {
                        error!(
                            "Failed to parse Claude output as JSON: {} - Line: {}",
                            e, line
                        );
                    },
                }
            }
        });

        let process = ClaudeProcess {
            id: session_id.clone(),
            child: Some(child),
            project_path,
        };

        self.processes.write().insert(session_id.clone(), process);

        Ok((session_id, rx))
    }

    pub async fn create_session_with_message(
        &self,
        session_id: Option<String>,
        project_path: Option<String>,
        model: Option<String>,
        message: &str,
    ) -> Result<(String, mpsc::Receiver<ClaudeCodeOutput>)> {
        let session_id = session_id.unwrap_or_else(|| Uuid::new_v4().to_string());

        let mut cmd = Command::new(&self.claude_command);
        cmd.arg("--print")
            .arg("--verbose")  // stream-json 需要 verbose
            .arg("--output-format").arg("stream-json");

        if let Some(model) = model {
            cmd.arg("--model").arg(model);
        }

        if let Some(ref path) = project_path {
            cmd.arg("--cwd").arg(path);
        }

        // 默认跳过权限检查以提高性能
        cmd.arg("--dangerously-skip-permissions");

        if self.mcp_config.enabled {
            if let Some(ref config_file) = self.mcp_config.config_file {
                cmd.arg("--mcp-config").arg(config_file);
            } else if let Some(ref config_json) = self.mcp_config.config_json {
                cmd.arg("--mcp-config").arg(config_json);
            }

            if self.mcp_config.strict {
                cmd.arg("--strict-mcp-config");
            }

            if self.mcp_config.debug {
                cmd.arg("--debug");
            }
        }

        // 不要将 message 作为命令行参数
        // cmd.arg(message);

        cmd.stdin(Stdio::piped())  // 改为 piped 以便写入
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        info!(
            "Starting Claude process for session {} with command: {:?}",
            session_id, cmd
        );

        let mut child = cmd.spawn()?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow!("Failed to get stdin"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow!("Failed to get stdout"))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| anyhow!("Failed to get stderr"))?;

        // 将消息写入 stdin
        use tokio::io::AsyncWriteExt;
        let message_bytes = message.as_bytes().to_vec();
        tokio::spawn(async move {
            let mut stdin = stdin;
            if let Err(e) = stdin.write_all(&message_bytes).await {
                error!("Failed to write to stdin: {}", e);
            }
            // 关闭 stdin 以表示输入结束
            drop(stdin);
        });

        let (tx, rx) = mpsc::channel(100);

        let session_id_clone = session_id.clone();
        let child_id = child.id();
        tokio::spawn(async move {
            info!(
                "Monitoring Claude process {} for session {}",
                child_id.unwrap_or(0),
                session_id_clone
            );
        });

        tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();

            while let Ok(Some(line)) = lines.next_line().await {
                error!("Claude stderr: {}", line);
            }
            info!("Claude stderr stream ended");
        });

        let tx_clone = tx.clone();
        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();

            while let Ok(Some(line)) = lines.next_line().await {
                if line.trim().is_empty() {
                    continue;
                }

                info!("Claude output line: {}", line);

                match serde_json::from_str::<ClaudeCodeOutput>(&line) {
                    Ok(output) => {
                        info!(
                            "Parsed Claude output: type={}, subtype={:?}",
                            output.r#type, output.subtype
                        );
                        if tx_clone.send(output).await.is_err() {
                            break;
                        }
                    },
                    Err(e) => {
                        error!("Failed to parse Claude output: {} - Line: {}", e, line);
                    },
                }
            }
            info!("Claude output stream ended");
        });

        let process = ClaudeProcess {
            id: session_id.clone(),
            child: Some(child),
            project_path,
        };

        self.processes.write().insert(session_id.clone(), process);

        Ok((session_id, rx))
    }

    #[allow(dead_code)]
    pub async fn send_message(&self, session_id: &str, message: &str) -> Result<()> {
        let stdin = {
            let mut processes = self.processes.write();
            let process = processes
                .get_mut(session_id)
                .ok_or_else(|| anyhow!("Session not found"))?;

            if let Some(ref mut child) = process.child {
                child.stdin.take()
            } else {
                None
            }
        };

        if let Some(mut stdin) = stdin {
            use tokio::io::AsyncWriteExt;
            info!("Writing message to stdin: {} bytes", message.len());
            stdin.write_all(message.as_bytes()).await?;
            stdin.write_all(b"\n").await?;
            stdin.flush().await?;
            info!("Message sent successfully");

            // 把 stdin 放回去
            let mut processes = self.processes.write();
            if let Some(process) = processes.get_mut(session_id)
                && let Some(ref mut child) = process.child
            {
                child.stdin = Some(stdin);
            }
        } else {
            error!("No stdin available for session {}", session_id);
        }

        Ok(())
    }

    pub async fn close_session(&self, session_id: &str) -> Result<()> {
        let child = {
            let mut processes = self.processes.write();
            processes
                .remove(session_id)
                .and_then(|mut p| p.child.take())
        };

        if let Some(mut child) = child {
            child.kill().await?;
            info!("Closed session {}", session_id);
        }

        Ok(())
    }

    #[allow(dead_code)]
    pub fn get_session_info(&self, session_id: &str) -> Option<(String, Option<String>)> {
        let processes = self.processes.read();
        processes
            .get(session_id)
            .map(|p| (p.id.clone(), p.project_path.clone()))
    }

    #[allow(dead_code)]
    pub async fn cleanup(&self) {
        let children: Vec<_> = {
            let mut processes = self.processes.write();
            processes
                .drain()
                .filter_map(|(_, mut p)| p.child.take())
                .collect()
        };

        for mut child in children {
            let _ = child.kill().await;
        }
    }
}

impl Drop for ClaudeManager {
    fn drop(&mut self) {
        let processes = self.processes.read();
        for (id, _) in processes.iter() {
            error!("Warning: Claude process {} still running at shutdown", id);
        }
    }
}
