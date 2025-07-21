use anyhow::{Result, anyhow};
use std::sync::Arc;
use parking_lot::RwLock;
use std::collections::HashMap;
use tokio::process::{Command, Child};
use tokio::io::{AsyncWriteExt, AsyncBufReadExt, BufReader};
use tokio::sync::mpsc;
use tracing::{info, error, warn};
use std::process::Stdio;

use crate::models::claude::ClaudeCodeOutput;

/// 会话进程管理器 - 每个会话复用一个 Claude 进程
pub struct SessionProcessManager {
    sessions: Arc<RwLock<HashMap<String, SessionProcess>>>,
    claude_command: String,
}

struct SessionProcess {
    child: Option<Child>,
    stdin_tx: mpsc::Sender<String>,
    conversation_id: String,
    created_at: std::time::Instant,
}

impl SessionProcessManager {
    pub fn new(claude_command: String) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            claude_command,
        }
    }

    /// 获取或创建会话进程
    pub async fn get_or_create_session(
        &self,
        conversation_id: String,
        model: String,
    ) -> Result<mpsc::Receiver<ClaudeCodeOutput>> {
        // 检查是否已有会话
        {
            let sessions = self.sessions.read();
            if sessions.contains_key(&conversation_id) {
                info!("Reusing existing session for conversation: {}", conversation_id);
                // TODO: 返回现有会话的接收器
            }
        }

        // 创建新会话
        info!("Creating new session process for conversation: {}", conversation_id);
        self.create_session(conversation_id, model).await
    }

    /// 创建新的会话进程
    async fn create_session(
        &self,
        conversation_id: String,
        model: String,
    ) -> Result<mpsc::Receiver<ClaudeCodeOutput>> {
        let mut cmd = Command::new(&self.claude_command);
        cmd.arg("--model").arg(&model)
            .arg("--output-format").arg("json")
            .arg("--dangerously-skip-permissions")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = cmd.spawn()?;
        
        let stdin = child.stdin.take().ok_or_else(|| anyhow!("Failed to get stdin"))?;
        let stdout = child.stdout.take().ok_or_else(|| anyhow!("Failed to get stdout"))?;
        let stderr = child.stderr.take().ok_or_else(|| anyhow!("Failed to get stderr"))?;

        // 创建输入通道
        let (stdin_tx, mut stdin_rx) = mpsc::channel::<String>(100);
        
        // 处理 stdin
        tokio::spawn(async move {
            let mut stdin = stdin;
            while let Some(msg) = stdin_rx.recv().await {
                if let Err(e) = stdin.write_all(msg.as_bytes()).await {
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
            }
        });

        // 创建输出通道
        let (output_tx, output_rx) = mpsc::channel(100);

        // 处理 stdout
        let output_tx_clone = output_tx.clone();
        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);
            let mut line = String::new();
            
            loop {
                line.clear();
                match reader.read_line(&mut line).await {
                    Ok(0) => break, // EOF
                    Ok(_) => {
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&line) {
                            let output = ClaudeCodeOutput {
                                r#type: json.get("type").and_then(|v| v.as_str()).unwrap_or("unknown").to_string(),
                                subtype: json.get("subtype").and_then(|v| v.as_str()).map(|s| s.to_string()),
                                data: json,
                            };
                            if output_tx_clone.send(output).await.is_err() {
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        error!("Error reading stdout: {}", e);
                        break;
                    }
                }
            }
        });

        // 处理 stderr
        tokio::spawn(async move {
            let mut reader = BufReader::new(stderr);
            let mut line = String::new();
            
            loop {
                line.clear();
                match reader.read_line(&mut line).await {
                    Ok(0) => break,
                    Ok(_) => {
                        warn!("Claude stderr: {}", line.trim());
                    }
                    Err(e) => {
                        error!("Error reading stderr: {}", e);
                        break;
                    }
                }
            }
        });

        // 保存会话
        let session = SessionProcess {
            child: Some(child),
            stdin_tx: stdin_tx.clone(),
            conversation_id: conversation_id.clone(),
            created_at: std::time::Instant::now(),
        };

        self.sessions.write().insert(conversation_id, session);

        Ok(output_rx)
    }

    /// 向会话发送消息
    pub async fn send_message(
        &self,
        conversation_id: &str,
        message: String,
    ) -> Result<()> {
        let sessions = self.sessions.read();
        if let Some(session) = sessions.get(conversation_id) {
            session.stdin_tx.send(message).await
                .map_err(|e| anyhow!("Failed to send message: {}", e))?;
            Ok(())
        } else {
            Err(anyhow!("Session not found: {}", conversation_id))
        }
    }

    /// 清理过期会话
    pub async fn cleanup_expired_sessions(&self, timeout_minutes: u64) {
        let mut sessions = self.sessions.write();
        let now = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(timeout_minutes * 60);

        sessions.retain(|id, session| {
            if now.duration_since(session.created_at) > timeout {
                info!("Cleaning up expired session: {}", id);
                if let Some(mut child) = session.child.as_ref() {
                    let _ = child.kill();
                }
                false
            } else {
                true
            }
        });
    }
}