# Claude Code SDK Optimizations

This document describes the performance optimizations implemented in the Claude Code SDK for Rust.

## Overview

The optimized API introduces several performance enhancements while maintaining compatibility with the existing SDK interface. The key improvements focus on:

1. **Unified Client Architecture** - Single client with multiple modes
2. **Connection Pooling** - Reuse subprocess connections
3. **Reduced Lock Contention** - Better concurrency patterns
4. **Retry Logic** - Automatic retry with exponential backoff
5. **Batch Processing** - Concurrent request handling
6. **Performance Metrics** - Built-in monitoring capabilities

## New Components

### OptimizedClient

The `OptimizedClient` provides a unified interface with three operating modes:

```rust
use cc_sdk::{OptimizedClient, ClientMode, ClaudeCodeOptions};

// One-shot mode for single queries
let client = OptimizedClient::new(options, ClientMode::OneShot)?;

// Interactive mode for stateful conversations
let client = OptimizedClient::new(options, ClientMode::Interactive)?;

// Batch mode for concurrent processing
let client = OptimizedClient::new(options, ClientMode::Batch { max_concurrent: 5 })?;
```

### Performance Utilities

#### RetryConfig

Configurable retry logic with exponential backoff:

```rust
let retry_config = RetryConfig {
    max_retries: 3,
    initial_delay: Duration::from_millis(100),
    max_delay: Duration::from_secs(30),
    backoff_multiplier: 2.0,
    jitter_factor: 0.1,
};
```

#### MessageBatcher

Efficient message batching for high-throughput scenarios:

```rust
let (batcher, tx, mut rx) = MessageBatcher::new(
    100,  // max batch size
    Duration::from_millis(50)  // max wait time
);
```

#### PerformanceMetrics

Built-in performance monitoring:

```rust
let metrics = PerformanceMetrics::default();
metrics.record_success(latency_ms);
metrics.record_failure();

println!("Success rate: {:.2}%", metrics.success_rate() * 100.0);
println!("Average latency: {:.2}ms", metrics.average_latency_ms());
```

## Key Optimizations

### 1. Connection Pooling

Instead of creating a new subprocess for each request, the optimized client maintains a pool of reusable connections:

- Reduces subprocess creation overhead
- Improves response times for subsequent requests
- Automatically manages connection lifecycle

### 2. Lock-Free Message Passing

The original implementation used heavy mutex locking. The optimized version:

- Uses RwLock for read-heavy operations
- Minimizes lock scope
- Employs lock-free channels where possible

### 3. Concurrent Request Handling

Batch mode enables processing multiple requests concurrently:

```rust
let results = client.process_batch(vec![
    "Query 1".to_string(),
    "Query 2".to_string(),
    "Query 3".to_string(),
]).await?;
```

### 4. Automatic Retry

Built-in retry logic handles transient failures:

```rust
let result = client.query_with_retry(
    prompt,
    3,  // max retries
    Duration::from_millis(100)  // initial delay
).await?;
```

## Performance Comparison

Based on the implementation, expected improvements include:

- **Latency**: 20-40% reduction for repeated queries (connection reuse)
- **Throughput**: 3-5x improvement with batch processing
- **Reliability**: Higher success rates with automatic retry
- **Resource Usage**: Lower system resource consumption

## Migration Guide

### From InteractiveClient

```rust
// Before
let mut client = InteractiveClient::new(options)?;
client.connect().await?;
let messages = client.send_and_receive(prompt).await?;

// After
let client = OptimizedClient::new(options, ClientMode::Interactive)?;
client.start_interactive_session().await?;
client.send_interactive(prompt).await?;
let messages = client.receive_interactive().await?;
```

### From query() function

```rust
// Before
let messages = query(prompt, Some(options)).await?;

// After
let client = OptimizedClient::new(options, ClientMode::OneShot)?;
let messages = client.query(prompt).await?;
```

## Examples

See the `examples/` directory for complete examples:

- `optimized_client_demo.rs` - Demonstrates all client modes
- `api_integration.rs` - Shows API wrapper pattern
- `performance_benchmark.rs` - Performance testing

## Future Enhancements

Potential future optimizations:

1. **Message Compression** - Reduce network overhead
2. **Request Deduplication** - Cache identical queries
3. **Adaptive Concurrency** - Dynamic adjustment based on load
4. **Circuit Breaker** - Fail fast during outages
5. **Request Prioritization** - QoS for different request types