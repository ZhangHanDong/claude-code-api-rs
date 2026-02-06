// 移除 dead_code，激活进程池

use anyhow::{Result, anyhow};
use parking_lot::Mutex;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{error, info};

use super::claude_manager::ClaudeManager;
use crate::models::claude::ClaudeCodeOutput;

#[derive(Clone)]
pub struct ProcessPool {
    inner: Arc<ProcessPoolInner>,
}

struct ProcessPoolInner {
    manager: Arc<ClaudeManager>,
    pool: Mutex<Pool>,
    config: PoolConfig,
}

struct Pool {
    idle: VecDeque<PooledProcess>,
    #[allow(dead_code)]
    active: Vec<ActiveProcess>,
}

struct PooledProcess {
    session_id: String,
    #[allow(dead_code)]
    model: String,
    created_at: std::time::Instant,
}

struct ActiveProcess {
    #[allow(dead_code)]
    session_id: String,
    #[allow(dead_code)]
    in_use_since: std::time::Instant,
}

#[derive(Clone)]
pub struct PoolConfig {
    pub min_idle: usize,
    #[allow(dead_code)]
    pub max_idle: usize,
    #[allow(dead_code)]
    pub max_active: usize,
    pub idle_timeout_secs: u64,
    pub default_model: String,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            min_idle: 2,
            max_idle: 5,
            max_active: 20,
            idle_timeout_secs: 300, // 5 minutes
            default_model: "claude-opus-4-20250514".to_string(),
        }
    }
}

impl ProcessPool {
    pub fn new(manager: Arc<ClaudeManager>, config: PoolConfig) -> Self {
        let pool = ProcessPool {
            inner: Arc::new(ProcessPoolInner {
                manager,
                pool: Mutex::new(Pool {
                    idle: VecDeque::new(),
                    active: Vec::new(),
                }),
                config,
            }),
        };

        // 预启动最小空闲进程
        let pool_clone = pool.clone();
        tokio::spawn(async move {
            pool_clone.maintain_min_idle().await;
        });

        // 定期清理过期的空闲进程
        let pool_clone = pool.clone();
        tokio::spawn(async move {
            pool_clone.cleanup_loop().await;
        });

        pool
    }

    pub async fn get_or_create(
        &self,
        model: String,
        message: String,
    ) -> Result<(String, mpsc::Receiver<ClaudeCodeOutput>)> {
        // 直接创建新会话，暂时不使用池化（需要更复杂的实现）
        info!("Creating new Claude session for model: {}", model);
        self.inner
            .manager
            .create_session_with_message(None, None, Some(model), &message)
            .await
    }

    #[allow(dead_code)]
    pub async fn acquire(
        &self,
        model: Option<String>,
    ) -> Result<(String, mpsc::Receiver<ClaudeCodeOutput>)> {
        let model = model.unwrap_or_else(|| self.inner.config.default_model.clone());

        // 尝试从池中获取空闲进程
        let session_id = {
            let mut pool = self.inner.pool.lock();

            // 查找匹配模型的空闲进程
            let position = pool.idle.iter().position(|p| p.model == model);

            if let Some(pos) = position {
                let process = pool.idle.remove(pos).unwrap();
                let session_id = process.session_id.clone();

                pool.active.push(ActiveProcess {
                    session_id: session_id.clone(),
                    in_use_since: std::time::Instant::now(),
                });

                info!("Acquired process from pool: {}", session_id);
                Some(session_id)
            } else {
                None
            }
        };

        if let Some(session_id) = session_id {
            // 创建新的接收通道
            let (_tx, rx) = mpsc::channel(100);
            // TODO: 需要重新连接到现有进程的输出流
            Ok((session_id, rx))
        } else {
            // 检查是否达到最大活跃数
            {
                let pool = self.inner.pool.lock();
                if pool.active.len() >= self.inner.config.max_active {
                    return Err(anyhow!("Process pool exhausted"));
                }
            }

            // 创建新进程
            info!("Creating new process for model: {}", model);
            let result = self
                .inner
                .manager
                .create_interactive_session(None, None, Some(model.clone()))
                .await?;

            // 记录为活跃进程
            {
                let mut pool = self.inner.pool.lock();
                pool.active.push(ActiveProcess {
                    session_id: result.0.clone(),
                    in_use_since: std::time::Instant::now(),
                });
            }

            Ok(result)
        }
    }

    #[allow(dead_code)]
    pub async fn release(&self, session_id: String, model: String) {
        // 检查是否需要关闭进程
        let should_close = {
            let mut pool = self.inner.pool.lock();

            // 从活跃列表中移除
            pool.active.retain(|p| p.session_id != session_id);

            // 如果池未满，添加到空闲列表
            if pool.idle.len() < self.inner.config.max_idle {
                pool.idle.push_back(PooledProcess {
                    session_id: session_id.clone(),
                    model,
                    created_at: std::time::Instant::now(),
                });
                info!("Released process back to pool");
                false
            } else {
                true
            }
        }; // 释放锁

        // 在锁释放后执行异步操作
        if should_close {
            let _ = self.inner.manager.close_session(&session_id).await;
            info!("Pool full, closed process: {}", session_id);
        }
    }

    async fn maintain_min_idle(&self) {
        loop {
            let needed = {
                let pool = self.inner.pool.lock();
                let current_idle = pool.idle.len();
                self.inner.config.min_idle.saturating_sub(current_idle)
            };

            for _ in 0..needed {
                match self
                    .inner
                    .manager
                    .create_interactive_session(
                        None,
                        None,
                        Some(self.inner.config.default_model.clone()),
                    )
                    .await
                {
                    Ok((session_id, _)) => {
                        let mut pool = self.inner.pool.lock();
                        pool.idle.push_back(PooledProcess {
                            session_id,
                            model: self.inner.config.default_model.clone(),
                            created_at: std::time::Instant::now(),
                        });
                        info!("Pre-warmed process added to pool");
                    },
                    Err(e) => {
                        error!("Failed to create pre-warmed process: {}", e);
                    },
                }
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
        }
    }

    async fn cleanup_loop(&self) {
        let timeout = std::time::Duration::from_secs(self.inner.config.idle_timeout_secs);

        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;

            let expired = {
                let mut pool = self.inner.pool.lock();
                let mut expired = Vec::new();

                // 检查过期的空闲进程
                pool.idle.retain(|p| {
                    if p.created_at.elapsed() > timeout {
                        expired.push(p.session_id.clone());
                        false
                    } else {
                        true
                    }
                });

                expired
            };

            // 关闭过期进程
            for session_id in expired {
                let _ = self.inner.manager.close_session(&session_id).await;
                info!("Closed idle process due to timeout: {}", session_id);
            }
        }
    }
}
