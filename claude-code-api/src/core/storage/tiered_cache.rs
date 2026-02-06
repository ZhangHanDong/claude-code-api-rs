//! Tiered caching with L1 (memory) and L2 (Neo4j)
//!
//! This module provides a two-level cache:
//! - L1: DashMap for fast, in-memory access
//! - L2: Neo4j for persistent, cross-restart storage
//!
//! Cache flow:
//! 1. Read: L1 hit → return | L1 miss → L2 lookup → populate L1 → return
//! 2. Write: Write to L1 → async write to L2

#![allow(dead_code)] // Public API - may not be used internally

use anyhow::Result;
use async_trait::async_trait;
use dashmap::DashMap;
use neo4rs::{Graph, query};
use serde::Serialize;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

use crate::core::cache::CacheStats;
use crate::models::openai::ChatCompletionResponse;

use super::traits::CacheStore;

/// Configuration for tiered cache
#[derive(Clone, Debug)]
pub struct TieredCacheConfig {
    /// Maximum entries in L1 cache
    pub l1_max_entries: usize,
    /// TTL for L1 cache entries in seconds
    pub l1_ttl_seconds: u64,
    /// Whether L2 (Neo4j) cache is enabled
    pub l2_enabled: bool,
    /// TTL for L2 cache entries in seconds
    pub l2_ttl_seconds: u64,
}

impl Default for TieredCacheConfig {
    fn default() -> Self {
        Self {
            l1_max_entries: 1000,
            l1_ttl_seconds: 3600, // 1 hour
            l2_enabled: true,
            l2_ttl_seconds: 86400, // 24 hours
        }
    }
}

/// L1 cache entry
#[derive(Clone)]
struct L1Entry {
    response: ChatCompletionResponse,
    created_at: Instant,
    hit_count: usize,
}

/// Tiered cache with L1 (DashMap) and L2 (Neo4j)
pub struct TieredCache {
    l1: DashMap<String, L1Entry>,
    l2: Option<Arc<Graph>>,
    config: TieredCacheConfig,
    l1_hits: std::sync::atomic::AtomicUsize,
    l2_hits: std::sync::atomic::AtomicUsize,
    misses: std::sync::atomic::AtomicUsize,
}

impl TieredCache {
    /// Create a new tiered cache with optional Neo4j L2
    pub fn new(config: TieredCacheConfig, neo4j_graph: Option<Arc<Graph>>) -> Self {
        let cache = Self {
            l1: DashMap::new(),
            l2: neo4j_graph,
            config,
            l1_hits: std::sync::atomic::AtomicUsize::new(0),
            l2_hits: std::sync::atomic::AtomicUsize::new(0),
            misses: std::sync::atomic::AtomicUsize::new(0),
        };

        // Start L1 cleanup task
        let l1_clone = cache.l1.clone();
        let ttl = cache.config.l1_ttl_seconds;
        tokio::spawn(async move {
            Self::l1_cleanup_loop(l1_clone, ttl).await;
        });

        cache
    }

    /// Create a new tiered cache with only L1 (memory)
    pub fn memory_only(config: TieredCacheConfig) -> Self {
        Self::new(config, None)
    }

    /// L1 cleanup background task
    async fn l1_cleanup_loop(cache: DashMap<String, L1Entry>, ttl_seconds: u64) {
        let ttl = Duration::from_secs(ttl_seconds);

        loop {
            tokio::time::sleep(Duration::from_secs(300)).await;

            let mut expired_keys = Vec::new();
            for entry in cache.iter() {
                if entry.value().created_at.elapsed() > ttl {
                    expired_keys.push(entry.key().clone());
                }
            }

            for key in expired_keys {
                cache.remove(&key);
            }

            debug!("L1 cache cleanup: {} entries remaining", cache.len());
        }
    }

    /// Get from L1 cache
    fn get_l1(&self, key: &str) -> Option<ChatCompletionResponse> {
        let mut entry = self.l1.get_mut(key)?;

        // Check TTL
        if entry.created_at.elapsed() > Duration::from_secs(self.config.l1_ttl_seconds) {
            drop(entry);
            self.l1.remove(key);
            return None;
        }

        entry.hit_count += 1;
        self.l1_hits
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        Some(entry.response.clone())
    }

    /// Get from L2 cache (Neo4j)
    async fn get_l2(&self, key: &str) -> Option<ChatCompletionResponse> {
        let graph = self.l2.as_ref()?;

        if !self.config.l2_enabled {
            return None;
        }

        let q = query(
            "MATCH (c:NexusCacheEntry {key: $key})
            WHERE c.expires_at > datetime()
            RETURN c.response as response",
        )
        .param("key", key);

        match graph.execute(q).await {
            Ok(mut result) => {
                if let Ok(Some(row)) = result.next().await
                    && let Ok(response_json) = row.get::<String>("response")
                    && let Ok(response) = serde_json::from_str(&response_json)
                {
                    self.l2_hits
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    debug!("L2 cache hit for key: {}", key);
                    return Some(response);
                }
            },
            Err(e) => {
                warn!("L2 cache read error: {}", e);
            },
        }

        None
    }

    /// Promote from L2 to L1
    fn promote_to_l1(&self, key: String, response: ChatCompletionResponse) {
        // Evict oldest if at capacity
        if self.l1.len() >= self.config.l1_max_entries {
            self.evict_oldest_l1();
        }

        self.l1.insert(
            key,
            L1Entry {
                response,
                created_at: Instant::now(),
                hit_count: 0,
            },
        );
    }

    /// Evict oldest L1 entry
    fn evict_oldest_l1(&self) {
        let mut oldest_key = None;
        let mut oldest_time = Instant::now();

        for entry in self.l1.iter() {
            if entry.value().created_at < oldest_time {
                oldest_time = entry.value().created_at;
                oldest_key = Some(entry.key().clone());
            }
        }

        if let Some(key) = oldest_key {
            self.l1.remove(&key);
        }
    }

    /// Write to L2 cache (async, non-blocking)
    async fn write_l2(&self, key: &str, response: &ChatCompletionResponse) {
        let Some(graph) = &self.l2 else { return };

        if !self.config.l2_enabled {
            return;
        }

        let response_json = match serde_json::to_string(response) {
            Ok(json) => json,
            Err(e) => {
                warn!("Failed to serialize response for L2 cache: {}", e);
                return;
            },
        };

        let q = query(
            "MERGE (c:NexusCacheEntry {key: $key})
            SET c.response = $response,
                c.created_at = datetime(),
                c.expires_at = datetime() + duration({seconds: $ttl})",
        )
        .param("key", key)
        .param("response", response_json)
        .param("ttl", self.config.l2_ttl_seconds as i64);

        if let Err(e) = graph.run(q).await {
            warn!("L2 cache write error: {}", e);
        } else {
            debug!("Wrote to L2 cache: {}", key);
        }
    }

    /// Initialize L2 cache schema
    pub async fn init_l2_schema(&self) -> Result<()> {
        let Some(graph) = &self.l2 else {
            return Ok(());
        };

        let constraint = "CREATE CONSTRAINT nexus_cache_key IF NOT EXISTS FOR (c:NexusCacheEntry) REQUIRE c.key IS UNIQUE";

        if let Err(e) = graph.run(query(constraint)).await {
            debug!("Cache constraint creation result: {:?}", e);
        }

        info!("L2 cache schema initialized");
        Ok(())
    }

    /// Get extended statistics
    pub fn extended_stats(&self) -> TieredCacheStats {
        let l1_hits = self.l1_hits.load(std::sync::atomic::Ordering::Relaxed);
        let l2_hits = self.l2_hits.load(std::sync::atomic::Ordering::Relaxed);
        let misses = self.misses.load(std::sync::atomic::Ordering::Relaxed);

        TieredCacheStats {
            l1_entries: self.l1.len(),
            l1_hits,
            l2_hits,
            misses,
            l2_enabled: self.l2.is_some() && self.config.l2_enabled,
            hit_rate: if l1_hits + l2_hits + misses > 0 {
                (l1_hits + l2_hits) as f64 / (l1_hits + l2_hits + misses) as f64
            } else {
                0.0
            },
        }
    }
}

#[async_trait]
impl CacheStore for TieredCache {
    async fn get(&self, key: &str) -> Option<ChatCompletionResponse> {
        // Try L1 first
        if let Some(response) = self.get_l1(key) {
            return Some(response);
        }

        // Try L2
        if let Some(response) = self.get_l2(key).await {
            // Promote to L1
            self.promote_to_l1(key.to_string(), response.clone());
            return Some(response);
        }

        self.misses
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        None
    }

    async fn put(&self, key: String, response: ChatCompletionResponse) {
        // Write to L1
        if self.l1.len() >= self.config.l1_max_entries {
            self.evict_oldest_l1();
        }

        self.l1.insert(
            key.clone(),
            L1Entry {
                response: response.clone(),
                created_at: Instant::now(),
                hit_count: 0,
            },
        );

        // Async write to L2
        self.write_l2(&key, &response).await;
    }

    async fn stats(&self) -> CacheStats {
        let extended = self.extended_stats();
        CacheStats {
            total_entries: extended.l1_entries,
            total_hits: extended.l1_hits + extended.l2_hits,
            enabled: true,
        }
    }

    async fn cleanup(&self) -> Result<usize> {
        let mut count = 0;

        // Cleanup L1
        let ttl = Duration::from_secs(self.config.l1_ttl_seconds);
        let mut expired = Vec::new();

        for entry in self.l1.iter() {
            if entry.value().created_at.elapsed() > ttl {
                expired.push(entry.key().clone());
            }
        }

        for key in expired {
            self.l1.remove(&key);
            count += 1;
        }

        // Cleanup L2
        if let Some(graph) = &self.l2 {
            let q = query(
                "MATCH (c:NexusCacheEntry)
                WHERE c.expires_at < datetime()
                DELETE c
                RETURN count(c) as deleted",
            );

            if let Ok(mut result) = graph.execute(q).await
                && let Ok(Some(row)) = result.next().await
                && let Ok(deleted) = row.get::<i64>("deleted")
            {
                count += deleted as usize;
            }
        }

        info!("Cache cleanup: removed {} entries", count);
        Ok(count)
    }
}

/// Extended statistics for tiered cache
#[derive(Debug, Clone, Serialize)]
pub struct TieredCacheStats {
    pub l1_entries: usize,
    pub l1_hits: usize,
    pub l2_hits: usize,
    pub misses: usize,
    pub l2_enabled: bool,
    pub hit_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::openai::Usage;

    #[tokio::test]
    async fn test_l1_cache_only() {
        let cache = TieredCache::memory_only(TieredCacheConfig::default());

        let response = ChatCompletionResponse {
            id: "test".to_string(),
            object: "chat.completion".to_string(),
            created: 0,
            model: "test".to_string(),
            choices: vec![],
            usage: Usage {
                prompt_tokens: 0,
                completion_tokens: 0,
                total_tokens: 0,
            },
            conversation_id: None,
        };

        cache.put("test-key".to_string(), response.clone()).await;

        let cached = cache.get("test-key").await;
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().id, "test");
    }

    #[tokio::test]
    async fn test_cache_miss() {
        let cache = TieredCache::memory_only(TieredCacheConfig::default());

        let cached = cache.get("nonexistent").await;
        assert!(cached.is_none());

        let stats = cache.extended_stats();
        assert_eq!(stats.misses, 1);
    }
}
