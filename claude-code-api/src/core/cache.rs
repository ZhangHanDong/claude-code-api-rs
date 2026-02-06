use dashmap::DashMap;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, info};

use crate::models::openai::{ChatCompletionResponse, ChatMessage};

#[derive(Clone)]
pub struct ResponseCache {
    inner: Arc<ResponseCacheInner>,
}

struct ResponseCacheInner {
    cache: DashMap<String, CacheEntry>,
    config: CacheConfig,
}

#[derive(Clone)]
pub struct CacheConfig {
    pub max_entries: usize,
    pub ttl_seconds: u64,
    pub enabled: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_entries: 1000,
            ttl_seconds: 3600, // 1 hour
            enabled: true,
        }
    }
}

#[derive(Clone, Debug)]
struct CacheEntry {
    response: ChatCompletionResponse,
    created_at: Instant,
    hit_count: usize,
}

impl ResponseCache {
    pub fn new(config: CacheConfig) -> Self {
        let cache = Self {
            inner: Arc::new(ResponseCacheInner {
                cache: DashMap::new(),
                config,
            }),
        };

        // 启动清理任务
        let cache_clone = cache.clone();
        tokio::spawn(async move {
            cache_clone.cleanup_loop().await;
        });

        cache
    }

    pub fn generate_key(model: &str, messages: &[ChatMessage]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(model.as_bytes());

        for msg in messages {
            hasher.update(msg.role.as_bytes());
            match &msg.content {
                Some(crate::models::openai::MessageContent::Text(text)) => {
                    hasher.update(text.as_bytes());
                },
                Some(crate::models::openai::MessageContent::Array(parts)) => {
                    for part in parts {
                        match part {
                            crate::models::openai::ContentPart::Text { text } => {
                                hasher.update(text.as_bytes());
                            },
                            crate::models::openai::ContentPart::ImageUrl { image_url } => {
                                hasher.update(image_url.url.as_bytes());
                            },
                        }
                    }
                },
                None => {
                    // Function calls don't affect cache key
                },
            }
        }

        format!("{:x}", hasher.finalize())
    }

    pub fn get(&self, key: &str) -> Option<ChatCompletionResponse> {
        if !self.inner.config.enabled {
            return None;
        }

        let mut entry = self.inner.cache.get_mut(key)?;

        // 检查是否过期
        if entry.created_at.elapsed() > Duration::from_secs(self.inner.config.ttl_seconds) {
            drop(entry);
            self.inner.cache.remove(key);
            debug!("Cache entry expired: {}", key);
            return None;
        }

        entry.hit_count += 1;
        let hit_count = entry.hit_count;
        let response = entry.response.clone();

        info!("Cache hit for key: {} (hits: {})", key, hit_count);
        Some(response)
    }

    pub fn put(&self, key: String, response: ChatCompletionResponse) {
        if !self.inner.config.enabled {
            return;
        }

        if self.inner.cache.len() >= self.inner.config.max_entries {
            self.evict_oldest();
        }

        let entry = CacheEntry {
            response,
            created_at: Instant::now(),
            hit_count: 0,
        };

        self.inner.cache.insert(key.clone(), entry);
        debug!("Cached response for key: {}", key);
    }

    fn evict_oldest(&self) {
        let mut oldest_key = None;
        let mut oldest_time = Instant::now();

        for entry in self.inner.cache.iter() {
            if entry.value().created_at < oldest_time {
                oldest_time = entry.value().created_at;
                oldest_key = Some(entry.key().clone());
            }
        }

        if let Some(key) = oldest_key {
            self.inner.cache.remove(&key);
            debug!("Evicted oldest cache entry: {}", key);
        }
    }

    async fn cleanup_loop(&self) {
        let ttl = Duration::from_secs(self.inner.config.ttl_seconds);

        loop {
            tokio::time::sleep(Duration::from_secs(300)).await; // 每5分钟清理一次

            let mut expired_keys = Vec::new();

            for entry in self.inner.cache.iter() {
                if entry.value().created_at.elapsed() > ttl {
                    expired_keys.push(entry.key().clone());
                }
            }

            for key in expired_keys {
                self.inner.cache.remove(&key);
                debug!("Removed expired cache entry: {}", key);
            }

            info!(
                "Cache cleanup: {} entries remaining",
                self.inner.cache.len()
            );
        }
    }

    pub fn stats(&self) -> CacheStats {
        let mut total_hits = 0;
        let mut total_entries = 0;

        for entry in self.inner.cache.iter() {
            total_entries += 1;
            total_hits += entry.value().hit_count;
        }

        CacheStats {
            total_entries,
            total_hits,
            enabled: self.inner.config.enabled,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct CacheStats {
    pub total_entries: usize,
    pub total_hits: usize,
    pub enabled: bool,
}
