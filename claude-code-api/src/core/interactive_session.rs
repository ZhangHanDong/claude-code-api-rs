use anyhow::{Result, anyhow};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{broadcast, mpsc};
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::core::claude_manager::ClaudeManager;
use crate::core::config::{FileAccessConfig, MCPConfig};
use crate::models::claude::ClaudeCodeOutput;

/// 交互式会话管理器 - 每个会话复用一个 Claude 进程
#[derive(Clone)]
pub struct InteractiveSessionManager {
    sessions: Arc<RwLock<HashMap<String, InteractiveSession>>>,
    claude_command: String,
    file_access_config: FileAccessConfig,
    mcp_config: MCPConfig,
}

struct InteractiveSession {
    #[allow(dead_code)]
    id: String,
    #[allow(dead_code)]
    conversation_id: String,
    child: Child,
    stdin_tx: mpsc::Sender<String>,
    output_tx: broadcast::Sender<ClaudeCodeOutput>,
    #[allow(dead_code)]
    model: String,
    #[allow(dead_code)]
    created_at: std::time::Instant,
    last_used: Arc<parking_lot::Mutex<std::time::Instant>>,
    // 添加互斥锁，确保一次只有一个请求与进程交互
    interaction_lock: Arc<tokio::sync::Mutex<()>>,
}

impl InteractiveSessionManager {
    pub fn new(_claude_manager: Arc<ClaudeManager>, claude_command: String) -> Self {
        let manager = Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            claude_command,
            file_access_config: FileAccessConfig::default(),
            mcp_config: MCPConfig::default(),
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
        let session_exists = self.sessions.read().contains_key(&conversation_id);

        if session_exists {
            info!("Reusing existing session: {}", conversation_id);

            // 在单独的任务中处理已存在的会话
            let sessions = self.sessions.clone();
            let conversation_id_clone = conversation_id.clone();
            let message_clone = message.clone();

            tokio::spawn(async move {
                let session_info = {
                    let sessions_guard = sessions.read();
                    sessions_guard.get(&conversation_id_clone).map(|s| {
                        (
                            s.stdin_tx.clone(),
                            s.output_tx.clone(),
                            Arc::clone(&s.last_used),
                            Arc::clone(&s.interaction_lock),
                        )
                    })
                };

                if let Some((stdin_tx, output_tx, last_used, interaction_lock)) = session_info {
                    // 获取交互锁，确保串行化访问
                    let _lock = interaction_lock.lock().await;
                    info!(
                        "Acquired interaction lock for session: {}",
                        conversation_id_clone
                    );

                    // 更新最后使用时间
                    *last_used.lock() = std::time::Instant::now();

                    // 创建专用的响应通道
                    let (request_tx, _request_rx) = mpsc::channel::<ClaudeCodeOutput>(100);

                    // 订阅输出广播
                    let mut output_rx = output_tx.subscribe();

                    // 启动响应收集任务
                    let response_handle = tokio::spawn(async move {
                        let mut responses = Vec::new();
                        let start_time = std::time::Instant::now();
                        let mut consecutive_empty_lines = 0;
                        let mut has_content = false;

                        loop {
                            // 使用较短的超时来检测响应结束
                            match tokio::time::timeout(
                                std::time::Duration::from_millis(500),
                                output_rx.recv(),
                            )
                            .await
                            {
                                Ok(Ok(output)) => {
                                    consecutive_empty_lines = 0;
                                    request_tx.send(output.clone()).await.ok();
                                    responses.push(output.clone());

                                    // 检查是否有实际内容
                                    if output.r#type == "text" || output.r#type == "content" {
                                        has_content = true;
                                    }

                                    // 检查错误响应
                                    if output.r#type == "error" {
                                        break;
                                    }
                                },
                                Ok(Err(_)) => {
                                    // 接收错误，通道关闭
                                    break;
                                },
                                Err(_) => {
                                    // 超时 - 检查是否已经有内容
                                    consecutive_empty_lines += 1;
                                    if has_content && consecutive_empty_lines >= 2 {
                                        // 如果已经收到内容，并且连续2次超时，认为响应完成
                                        info!("Response appears complete after timeout");
                                        break;
                                    }

                                    // 总超时保护
                                    if start_time.elapsed() > std::time::Duration::from_secs(30) {
                                        error!("Total timeout waiting for response");
                                        break;
                                    }
                                },
                            }
                        }

                        responses
                    });

                    // 发送消息
                    if let Err(e) = stdin_tx.send(message_clone).await {
                        error!("Failed to send message to session: {}", e);
                        response_tx
                            .send(ClaudeCodeOutput {
                                r#type: "error".to_string(),
                                subtype: None,
                                data: serde_json::json!({
                                    "error": format!("Failed to send message: {}", e)
                                }),
                            })
                            .await
                            .ok();
                        return;
                    }

                    // 等待响应收集完成
                    let responses = response_handle.await.unwrap_or_default();

                    // 转发响应到请求者
                    for output in responses {
                        response_tx.send(output).await.ok();
                    }

                    // 关闭通道
                    drop(response_tx);

                    info!(
                        "Released interaction lock for session: {}",
                        conversation_id_clone
                    );
                }
            });

            return Ok((conversation_id, response_rx));
        }

        // 创建新会话
        info!("Creating new interactive session: {}", conversation_id);
        self.create_session(conversation_id.clone(), model, message, response_tx)
            .await?;

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
        if self.mcp_config.enabled
            && let Some(ref config_file) = self.mcp_config.config_file
        {
            cmd.arg("--mcp-config").arg(config_file);
        }

        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        info!(
            "Starting interactive Claude session with command: {:?}",
            cmd
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

        // 创建通道
        let (stdin_tx, mut stdin_rx) = mpsc::channel::<String>(100);
        let (output_tx, _) = broadcast::channel(100);

        // 为初始请求创建专用通道
        let (initial_tx, mut initial_rx) = mpsc::channel::<ClaudeCodeOutput>(100);

        // 启动初始响应收集任务
        let initial_response_tx_clone = initial_response_tx.clone();
        tokio::spawn(async move {
            let start_time = std::time::Instant::now();
            let mut has_content = false;
            let mut consecutive_timeouts = 0;

            loop {
                match tokio::time::timeout(std::time::Duration::from_millis(500), initial_rx.recv())
                    .await
                {
                    Ok(Some(output)) => {
                        consecutive_timeouts = 0;
                        if output.r#type == "text" || output.r#type == "content" {
                            has_content = true;
                        }
                        if initial_response_tx_clone.send(output).await.is_err() {
                            break;
                        }
                    },
                    Ok(None) => break, // 通道关闭
                    Err(_) => {
                        // 超时
                        consecutive_timeouts += 1;
                        if has_content && consecutive_timeouts >= 2 {
                            info!("Initial response appears complete");
                            break;
                        }
                        if start_time.elapsed() > std::time::Duration::from_secs(30) {
                            error!("Timeout waiting for initial response");
                            break;
                        }
                    },
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
        let initial_tx_clone = initial_tx.clone();
        let is_first_response = Arc::new(parking_lot::Mutex::new(true));

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

                    // 如果是第一次响应，发送到初始通道
                    let should_send = {
                        let mut is_first = is_first_response.lock();
                        if *is_first {
                            // 检查是否响应结束
                            if output.r#type == "error"
                                || (output.r#type == "text" && line.contains("Human:"))
                            {
                                *is_first = false;
                            }
                            true
                        } else {
                            false
                        }
                    };

                    if should_send {
                        let _ = initial_tx_clone.send(output.clone()).await;
                    }

                    // 广播输出到所有订阅者
                    let _ = output_tx_clone.send(output);
                }
            }
            info!(
                "Claude stdout stream ended for session: {}",
                conversation_id_clone
            );
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
            stdin_tx
                .send(initial_message)
                .await
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
            last_used: Arc::new(parking_lot::Mutex::new(std::time::Instant::now())),
            interaction_lock: Arc::new(tokio::sync::Mutex::new(())),
        };

        self.sessions.write().insert(conversation_id, session);

        Ok(())
    }

    /// 清理过期会话
    async fn cleanup_expired_sessions(
        sessions: Arc<RwLock<HashMap<String, InteractiveSession>>>,
        timeout_minutes: u64,
    ) {
        let now = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(timeout_minutes * 60);

        // Collect expired sessions while holding the lock
        let expired_sessions: Vec<(String, InteractiveSession)> = {
            let mut sessions = sessions.write();
            let expired_ids: Vec<String> = sessions
                .iter()
                .filter(|(_, session)| {
                    let last_used = *session.last_used.lock();
                    now.duration_since(last_used) > timeout
                })
                .map(|(id, _)| id.clone())
                .collect();

            expired_ids
                .into_iter()
                .filter_map(|id| sessions.remove(&id).map(|s| (id, s)))
                .collect()
        };
        // Lock is released here

        // Now kill the expired sessions without holding the lock
        for (id, mut session) in expired_sessions {
            info!("Cleaning up expired session: {}", id);
            let _ = session.child.kill().await;
        }
    }

    /// 关闭指定会话
    #[allow(dead_code)]
    pub async fn close_session(&self, conversation_id: &str) -> Result<()> {
        let session_opt = {
            let mut sessions = self.sessions.write();
            sessions.remove(conversation_id)
        };
        if let Some(mut session) = session_opt {
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
    #[allow(dead_code)]
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
            // Note: In Drop we can't await, so we start the kill and let it complete
            // The process will be cleaned up by the OS regardless
            #[allow(clippy::let_underscore_future)]
            let _ = session.child.kill();
        }
    }
}
