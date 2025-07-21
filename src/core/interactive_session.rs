use anyhow::{Result, anyhow};
use std::sync::Arc;
use parking_lot::RwLock;
use std::collections::HashMap;
use tokio::process::{Command, Child};
use tokio::io::{AsyncWriteExt, AsyncBufReadExt, BufReader};
use tokio::sync::{mpsc, broadcast};
use tracing::{info, error, warn};
use std::process::Stdio;
use uuid::Uuid;

use crate::models::claude::ClaudeCodeOutput;
use crate::core::config::{FileAccessConfig, MCPConfig};

/// 交互式会话管理器 - 每个会话复用一个 Claude 进程
#[derive(Clone)]
pub struct InteractiveSessionManager {
    sessions: Arc<RwLock<HashMap<String, InteractiveSession>>>,
    claude_command: String,
    file_access_config: FileAccessConfig,
    mcp_config: MCPConfig,
}

struct InteractiveSession {
    id: String,
    conversation_id: String,
    child: Child,
    stdin_tx: mpsc::Sender<String>,
    output_tx: broadcast::Sender<ClaudeCodeOutput>,
    model: String,
    created_at: std::time::Instant,
    last_used: parking_lot::Mutex<std::time::Instant>,
    // 添加互斥锁，确保一次只有一个请求与进程交互
    interaction_lock: tokio::sync::Mutex<()>,
}

impl InteractiveSessionManager {
    pub fn new(
        claude_command: String, 
        file_access_config: FileAccessConfig,
        mcp_config: MCPConfig,
    ) -> Self {
        let manager = Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            claude_command,
            file_access_config,
            mcp_config,
        };

        // 启动清理任务
        let sessions_clone = manager.sessions.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(300)).await; // 每5分钟
                Self::cleanup_expired_sessions(sessions_clone.clone(), 30).await; // 30分钟超时
            }
        });

        // 注释掉这里的预热逻辑，因为在 main.rs 中会调用 prewarm_default_session
        // 避免创建重复的预热进程

        manager
    }

    /// 获取或创建会话，并发送消息
    pub async fn get_or_create_session_and_send(
        &self,
        conversation_id: Option<String>,
        model: String,
        message: String,
    ) -> Result<(String, mpsc::Receiver<ClaudeCodeOutput>)> {
        let conversation_id = conversation_id.unwrap_or_else(|| Uuid::new_v4().to_string());
        
        // 创建此次请求的输出接收器
        let (response_tx, response_rx) = mpsc::channel(100);
        
        // 检查是否已有会话
        let existing_session = {
            let sessions = self.sessions.read();
            if let Some(session) = sessions.get(&conversation_id) {
                info!("Reusing existing session: {}", conversation_id);
                
                // 更新最后使用时间
                *session.last_used.lock() = std::time::Instant::now();
                
                // 订阅输出
                let mut output_rx = session.output_tx.subscribe();
                let response_tx_clone = response_tx.clone();
                
                // 启动转发任务
                tokio::spawn(async move {
                    while let Ok(output) = output_rx.recv().await {
                        if response_tx_clone.send(output).await.is_err() {
                            break;
                        }
                    }
                });
                
                // 克隆 stdin_tx 以便在锁释放后使用
                let stdin_tx = session.stdin_tx.clone();
                Some((conversation_id.clone(), stdin_tx))
            } else {
                None
            }
        };
        
        // 在锁释放后发送消息
        let existing_session = if let Some((_conv_id, stdin_tx)) = existing_session {
            match stdin_tx.send(message.clone()).await {
                Ok(_) => true,
                Err(e) => {
                    error!("Failed to send message to existing session: {}", e);
                    false
                }
            }
        } else {
            false
        };

        if existing_session {
            return Ok((conversation_id, response_rx));
        }

        // 创建新会话
        info!("Creating new interactive session: {}", conversation_id);
        self.create_session(conversation_id.clone(), model, message, response_tx).await?;
        
        Ok((conversation_id, response_rx))
    }

    /// 创建新的交互式会话
    async fn create_session(
        &self,
        conversation_id: String,
        model: String,
        initial_message: String,
        initial_response_tx: mpsc::Sender<ClaudeCodeOutput>,
    ) -> Result<()> {
        let mut cmd = Command::new(&self.claude_command);
        
        // 使用交互模式 - 不要使用 --output-format，因为它只能与 --print 一起使用
        cmd.arg("--model").arg(&model);

        // 文件访问权限
        if self.file_access_config.skip_permissions {
            cmd.arg("--dangerously-skip-permissions");
        }

        // MCP 配置
        if self.mcp_config.enabled {
            if let Some(ref config_file) = self.mcp_config.config_file {
                cmd.arg("--mcp-config").arg(config_file);
            }
        }

        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        info!("Starting interactive Claude session with command: {:?}", cmd);

        let mut child = cmd.spawn()?;
        
        let stdin = child.stdin.take().ok_or_else(|| anyhow!("Failed to get stdin"))?;
        let stdout = child.stdout.take().ok_or_else(|| anyhow!("Failed to get stdout"))?;
        let stderr = child.stderr.take().ok_or_else(|| anyhow!("Failed to get stderr"))?;

        // 创建通道
        let (stdin_tx, mut stdin_rx) = mpsc::channel::<String>(100);
        let (output_tx, _) = broadcast::channel(100);

        // 为初始请求订阅输出
        let mut initial_output_rx = output_tx.subscribe();
        tokio::spawn(async move {
            while let Ok(output) = initial_output_rx.recv().await {
                if initial_response_tx.send(output).await.is_err() {
                    break;
                }
            }
        });

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
                info!("Sent message to Claude process");
            }
        });

        // 处理 stdout
        let conversation_id_clone = conversation_id.clone();
        let output_tx_clone = output_tx.clone();
        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            
            while let Ok(Some(line)) = lines.next_line().await {
                if line.trim().is_empty() {
                    continue;
                }
                
                info!("Claude output: {}", line);
                
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&line) {
                    let output = ClaudeCodeOutput {
                        r#type: json.get("type").and_then(|v| v.as_str()).unwrap_or("unknown").to_string(),
                        subtype: json.get("subtype").and_then(|v| v.as_str()).map(|s| s.to_string()),
                        data: json,
                    };
                    
                    // 广播输出到所有订阅者
                    let _ = output_tx_clone.send(output);
                }
            }
            info!("Claude stdout stream ended for session: {}", conversation_id_clone);
        });

        // 处理 stderr
        tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            
            while let Ok(Some(line)) = lines.next_line().await {
                warn!("Claude stderr: {}", line);
            }
        });

        // 发送初始消息（如果不为空）
        if !initial_message.is_empty() {
            stdin_tx.send(initial_message).await
                .map_err(|e| anyhow!("Failed to send initial message: {}", e))?;
        }

        // 保存会话
        let session = InteractiveSession {
            id: Uuid::new_v4().to_string(),
            conversation_id: conversation_id.clone(),
            child,
            stdin_tx,
            output_tx,
            model,
            created_at: std::time::Instant::now(),
            last_used: parking_lot::Mutex::new(std::time::Instant::now()),
            interaction_lock: tokio::sync::Mutex::new(()),
        };

        self.sessions.write().insert(conversation_id, session);

        Ok(())
    }

    /// 清理过期会话
    async fn cleanup_expired_sessions(
        sessions: Arc<RwLock<HashMap<String, InteractiveSession>>>,
        timeout_minutes: u64,
    ) {
        let mut sessions = sessions.write();
        let now = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(timeout_minutes * 60);

        let expired: Vec<String> = sessions
            .iter()
            .filter(|(_, session)| {
                let last_used = *session.last_used.lock();
                now.duration_since(last_used) > timeout
            })
            .map(|(id, _)| id.clone())
            .collect();

        for id in expired {
            if let Some(mut session) = sessions.remove(&id) {
                info!("Cleaning up expired session: {}", id);
                let _ = session.child.kill();
            }
        }
    }

    /// 关闭指定会话
    pub async fn close_session(&self, conversation_id: &str) -> Result<()> {
        let mut sessions = self.sessions.write();
        if let Some(mut session) = sessions.remove(conversation_id) {
            info!("Closing session: {}", conversation_id);
            session.child.kill().await?;
            Ok(())
        } else {
            Err(anyhow!("Session not found: {}", conversation_id))
        }
    }

    /// 预热一个默认进程，用于第一个请求
    pub async fn prewarm_default_session(&self) -> Result<()> {
        info!("Pre-warming default Claude process for faster first request");
        
        // 暂时跳过预热，因为需要先修复交互式会话的基本功能
        // TODO: 实现预热逻辑
        
        Ok(())
    }
    
    /// 获取活跃会话数
    pub fn active_sessions(&self) -> usize {
        self.sessions.read().len()
    }
}

impl Drop for InteractiveSessionManager {
    fn drop(&mut self) {
        // 清理所有会话
        let mut sessions = self.sessions.write();
        for (id, mut session) in sessions.drain() {
            info!("Cleaning up session on shutdown: {}", id);
            let _ = session.child.kill();
        }
    }
}