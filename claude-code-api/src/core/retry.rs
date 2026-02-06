#![allow(dead_code)]

use anyhow::Result;
use std::future::Future;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{error, info, warn};

#[derive(Clone)]
pub struct RetryConfig {
    pub max_retries: u32,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub exponential_base: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay_ms: 1000,
            max_delay_ms: 30000,
            exponential_base: 2.0,
        }
    }
}

pub struct RetryPolicy {
    config: RetryConfig,
}

impl RetryPolicy {
    pub fn new(config: RetryConfig) -> Self {
        Self { config }
    }

    pub async fn execute<F, Fut, T, E>(
        &self,
        operation_name: &str,
        mut operation: F,
    ) -> Result<T, E>
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = Result<T, E>>,
        E: std::fmt::Display,
    {
        let mut attempt = 0;
        let mut delay_ms = self.config.initial_delay_ms;

        loop {
            attempt += 1;

            match operation().await {
                Ok(result) => {
                    if attempt > 1 {
                        info!("{} succeeded after {} attempts", operation_name, attempt);
                    }
                    return Ok(result);
                },
                Err(err) => {
                    if attempt >= self.config.max_retries {
                        error!(
                            "{} failed after {} attempts: {}",
                            operation_name, attempt, err
                        );
                        return Err(err);
                    }

                    warn!(
                        "{} failed (attempt {}/{}): {}. Retrying in {}ms...",
                        operation_name, attempt, self.config.max_retries, err, delay_ms
                    );

                    sleep(Duration::from_millis(delay_ms)).await;

                    // Calculate next delay with exponential backoff
                    delay_ms = ((delay_ms as f64) * self.config.exponential_base) as u64;
                    delay_ms = delay_ms.min(self.config.max_delay_ms);
                },
            }
        }
    }

    pub fn should_retry<E: std::fmt::Display>(error: &E) -> bool {
        let error_str = error.to_string().to_lowercase();

        // Retry on these types of errors
        if error_str.contains("timeout")
            || error_str.contains("connection")
            || error_str.contains("temporarily unavailable")
            || error_str.contains("too many requests")
            || error_str.contains("overloaded")
        {
            return true;
        }

        // Don't retry on these
        if error_str.contains("invalid")
            || error_str.contains("unauthorized")
            || error_str.contains("forbidden")
            || error_str.contains("not found")
        {
            return false;
        }

        // Default to retry for unknown errors
        true
    }
}

#[derive(Clone)]
pub struct CircuitBreaker {
    failure_threshold: u32,
    recovery_timeout: Duration,
    failures: std::sync::Arc<std::sync::atomic::AtomicU32>,
    last_failure: std::sync::Arc<parking_lot::Mutex<Option<std::time::Instant>>>,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u32, recovery_timeout_secs: u64) -> Self {
        Self {
            failure_threshold,
            recovery_timeout: Duration::from_secs(recovery_timeout_secs),
            failures: std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0)),
            last_failure: std::sync::Arc::new(parking_lot::Mutex::new(None)),
        }
    }

    pub fn is_open(&self) -> bool {
        let failures = self.failures.load(std::sync::atomic::Ordering::Relaxed);
        if failures < self.failure_threshold {
            return false;
        }

        // Check if we should reset
        if let Some(last_failure) = *self.last_failure.lock()
            && last_failure.elapsed() > self.recovery_timeout
        {
            self.reset();
            return false;
        }

        true
    }

    pub fn record_success(&self) {
        self.reset();
    }

    pub fn record_failure(&self) {
        self.failures
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        *self.last_failure.lock() = Some(std::time::Instant::now());
    }

    fn reset(&self) {
        self.failures.store(0, std::sync::atomic::Ordering::Relaxed);
        *self.last_failure.lock() = None;
    }
}
