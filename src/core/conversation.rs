use anyhow::Result;
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;
use tracing::info;

use crate::models::openai::{ChatMessage, MessageContent};

#[derive(Clone)]
pub struct ConversationManager {
    inner: Arc<ConversationManagerInner>,
}

struct ConversationManagerInner {
    conversations: RwLock<HashMap<String, Conversation>>,
    config: ConversationConfig,
}

#[derive(Clone)]
pub struct ConversationConfig {
    pub max_history_messages: usize,
    pub max_context_tokens: usize,
    pub session_timeout_minutes: i64,
}

impl Default for ConversationConfig {
    fn default() -> Self {
        Self {
            max_history_messages: 20,
            max_context_tokens: 100000,
            session_timeout_minutes: 30,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Conversation {
    pub id: String,
    pub messages: Vec<ChatMessage>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub metadata: ConversationMetadata,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ConversationMetadata {
    pub model: Option<String>,
    pub total_tokens: usize,
    pub turn_count: usize,
    pub project_path: Option<String>,
}

impl ConversationManager {
    pub fn new(config: ConversationConfig) -> Self {
        let manager = Self {
            inner: Arc::new(ConversationManagerInner {
                conversations: RwLock::new(HashMap::new()),
                config,
            }),
        };
        
        // 启动清理任务
        let manager_clone = manager.clone();
        tokio::spawn(async move {
            manager_clone.cleanup_loop().await;
        });
        
        manager
    }
    
    pub fn create_conversation(&self, model: Option<String>) -> String {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        
        let conversation = Conversation {
            id: id.clone(),
            messages: Vec::new(),
            created_at: now,
            updated_at: now,
            metadata: ConversationMetadata {
                model,
                ..Default::default()
            },
        };
        
        self.inner.conversations.write().insert(id.clone(), conversation);
        info!("Created new conversation: {}", id);
        
        id
    }
    
    pub fn add_message(
        &self, 
        conversation_id: &str, 
        message: ChatMessage
    ) -> Result<()> {
        let mut conversations = self.inner.conversations.write();
        
        if let Some(conversation) = conversations.get_mut(conversation_id) {
            conversation.messages.push(message);
            conversation.updated_at = Utc::now();
            conversation.metadata.turn_count += 1;
            
            // 限制历史消息数量
            if conversation.messages.len() > self.inner.config.max_history_messages {
                let remove_count = conversation.messages.len() - self.inner.config.max_history_messages;
                conversation.messages.drain(0..remove_count);
                info!("Trimmed {} old messages from conversation {}", remove_count, conversation_id);
            }
            
            Ok(())
        } else {
            Err(anyhow::anyhow!("Conversation not found"))
        }
    }
    
    pub fn get_conversation(&self, conversation_id: &str) -> Option<Conversation> {
        self.inner.conversations.read().get(conversation_id).cloned()
    }
    
    pub fn get_context_messages(
        &self, 
        conversation_id: &str,
        new_messages: &[ChatMessage]
    ) -> Vec<ChatMessage> {
        let conversations = self.inner.conversations.read();
        
        if let Some(conversation) = conversations.get(conversation_id) {
            let mut context = conversation.messages.clone();
            context.extend_from_slice(new_messages);
            
            // 智能裁剪上下文
            self.trim_context(context)
        } else {
            new_messages.to_vec()
        }
    }
    
    fn trim_context(&self, messages: Vec<ChatMessage>) -> Vec<ChatMessage> {
        // 简单的策略：保留系统消息和最近的消息
        let mut system_messages = Vec::new();
        let mut other_messages = Vec::new();
        
        for msg in messages {
            if msg.role == "system" {
                system_messages.push(msg);
            } else {
                other_messages.push(msg);
            }
        }
        
        // 估算token数（简化：每个字符约0.25个token）
        let estimate_tokens = |msgs: &[ChatMessage]| -> usize {
            msgs.iter()
                .map(|m| match &m.content {
                    MessageContent::Text(text) => text.len() / 4,
                    MessageContent::Array(parts) => parts.len() * 100, // 粗略估计
                })
                .sum()
        };
        
        let mut result = system_messages;
        let mut token_count = estimate_tokens(&result);
        
        // 从最新的消息开始添加
        for msg in other_messages.into_iter().rev() {
            let msg_tokens = estimate_tokens(std::slice::from_ref(&msg));
            if token_count + msg_tokens > self.inner.config.max_context_tokens {
                break;
            }
            result.push(msg);
            token_count += msg_tokens;
        }
        
        // 恢复正确的顺序
        if result.len() > 1 {
            let system_count = result.iter().filter(|m| m.role == "system").count();
            result[system_count..].reverse();
        }
        
        result
    }
    
    pub fn update_metadata(
        &self,
        conversation_id: &str,
        update_fn: impl FnOnce(&mut ConversationMetadata)
    ) -> Result<()> {
        let mut conversations = self.inner.conversations.write();
        
        if let Some(conversation) = conversations.get_mut(conversation_id) {
            update_fn(&mut conversation.metadata);
            conversation.updated_at = Utc::now();
            Ok(())
        } else {
            Err(anyhow::anyhow!("Conversation not found"))
        }
    }
    
    pub fn list_active_conversations(&self) -> Vec<(String, DateTime<Utc>)> {
        let conversations = self.inner.conversations.read();
        conversations
            .iter()
            .map(|(id, conv)| (id.clone(), conv.updated_at))
            .collect()
    }
    
    async fn cleanup_loop(&self) {
        let timeout = chrono::Duration::minutes(self.inner.config.session_timeout_minutes);
        
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(300)).await; // 每5分钟检查一次
            
            let now = Utc::now();
            let mut expired = Vec::new();
            
            {
                let conversations = self.inner.conversations.read();
                for (id, conv) in conversations.iter() {
                    if now - conv.updated_at > timeout {
                        expired.push(id.clone());
                    }
                }
            }
            
            if !expired.is_empty() {
                let mut conversations = self.inner.conversations.write();
                for id in expired {
                    conversations.remove(&id);
                    info!("Removed expired conversation: {}", id);
                }
            }
        }
    }
}