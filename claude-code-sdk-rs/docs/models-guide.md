# Claude Models Guide (2025)

This guide provides comprehensive information about using different Claude models with the cc-sdk.

## Available Models (as of 2025)

### Opus 4.1 - Most Capable
The latest and most powerful model, suitable for complex reasoning, creative tasks, and detailed analysis.

**Model identifiers:**
- `"opus-4.1"` - Recommended alias
- `"claude-opus-4-1-20250805"` - Full model name
- `"opus"` - Latest Opus (currently 4.1)

**Example usage:**
```rust
use cc_sdk::{query, ClaudeCodeOptions, Result};
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<()> {
    let options = ClaudeCodeOptions::builder()
        .model("opus-4.1")
        .max_thinking_tokens(10000)  // Opus 4.1 supports advanced reasoning
        .build();
    
    let mut messages = query(
        "Analyze this complex algorithm and suggest optimizations",
        Some(options)
    ).await?;
    
    while let Some(msg) = messages.next().await {
        println!("{:?}", msg?);
    }
    
    Ok(())
}
```

### Sonnet 4 - Balanced Performance
Excellent balance between capability and speed, ideal for most applications.

**Model identifiers:**
- `"sonnet-4"` - Recommended alias
- `"claude-sonnet-4-20250514"` - Full model name
- `"sonnet"` - Latest Sonnet (currently 4)

**Example usage:**
```rust
use cc_sdk::{InteractiveClient, ClaudeCodeOptions, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let options = ClaudeCodeOptions::builder()
        .model("sonnet-4")
        .permission_mode(cc_sdk::PermissionMode::AcceptEdits)
        .build();
    
    let mut client = InteractiveClient::new(options)?;
    client.connect().await?;
    
    let messages = client.send_and_receive(
        "Write a REST API in Rust".to_string()
    ).await?;
    
    // Process responses...
    client.disconnect().await?;
    Ok(())
}
```

### Previous Generation Models

#### Claude 3.5 Sonnet
- Model ID: `"claude-3-5-sonnet-20241022"`
- Good for general tasks, previous generation

#### Claude 3.5 Haiku
- Model ID: `"claude-3-5-haiku-20241022"`
- Fastest response times, suitable for simple tasks

## Choosing the Right Model

### Use Opus 4.1 when you need:
- Complex reasoning and analysis
- Creative writing and content generation
- Advanced code generation and refactoring
- Multi-step problem solving
- Maximum capability regardless of speed

### Use Sonnet 4 when you need:
- Balanced performance and speed
- General programming assistance
- Interactive conversations
- Most day-to-day tasks
- Cost-effective powerful assistance

### Use Haiku when you need:
- Fast responses
- Simple queries
- High-volume processing
- Minimal latency

## Model Features Comparison

| Feature | Opus 4.1 | Sonnet 4 | Haiku 3.5 |
|---------|----------|----------|-----------|
| Reasoning | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐ |
| Speed | ⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ |
| Creativity | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐ |
| Code Quality | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐ |
| Context Length | Maximum | High | Standard |
| Thinking Tokens | 10000+ | 8000 | 4000 |

## Advanced Configuration Examples

### Using Extra Arguments with Models
```rust
use cc_sdk::{ClaudeCodeOptions, PermissionMode};
use std::collections::HashMap;

let mut extra_args = HashMap::new();
extra_args.insert("temperature".to_string(), Some("0.7".to_string()));
extra_args.insert("verbose".to_string(), None);

let options = ClaudeCodeOptions::builder()
    .model("opus-4.1")
    .permission_mode(PermissionMode::Plan)  // New in v0.1.7
    .extra_args(extra_args)  // New in v0.1.7
    .max_thinking_tokens(15000)
    .build();
```

### Interactive Session with Model Selection
```rust
use cc_sdk::{InteractiveClient, ClaudeCodeOptions, Result};

async fn create_client_with_model(model: &str) -> Result<InteractiveClient> {
    let options = ClaudeCodeOptions::builder()
        .model(model)
        .system_prompt("You are an expert Rust developer")
        .build();
    
    Ok(InteractiveClient::new(options)?)
}

#[tokio::main]
async fn main() -> Result<()> {
    // Try Opus 4.1 first
    let mut client = create_client_with_model("opus-4.1").await?;
    
    // Fallback to Sonnet 4 if needed
    if client.connect().await.is_err() {
        println!("Opus 4.1 unavailable, falling back to Sonnet 4");
        client = create_client_with_model("sonnet-4").await?;
        client.connect().await?;
    }
    
    // Use the client...
    Ok(())
}
```

## Checking Model Availability

```rust
use cc_sdk::{query, ClaudeCodeOptions, Result};
use futures::StreamExt;

async fn test_model(model_name: &str) -> bool {
    let options = ClaudeCodeOptions::builder()
        .model(model_name)
        .max_turns(1)
        .build();
    
    match query("Say 'OK'", Some(options)).await {
        Ok(mut stream) => {
            while let Some(msg) = stream.next().await {
                if msg.is_ok() {
                    return true;
                }
            }
            false
        }
        Err(_) => false
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let models = vec![
        "opus-4.1",
        "sonnet-4",
        "opus",
        "sonnet",
        "haiku"
    ];
    
    for model in models {
        if test_model(model).await {
            println!("✓ {} is available", model);
        } else {
            println!("✗ {} is not available", model);
        }
    }
    
    Ok(())
}
```

## Error Handling for Invalid Models

```rust
use cc_sdk::{query, ClaudeCodeOptions, SdkError, Result};

async fn safe_query_with_fallback(prompt: &str) -> Result<()> {
    // Try with preferred model
    let result = query_with_model(prompt, "opus-4.1").await;
    
    match result {
        Ok(_) => Ok(()),
        Err(SdkError::CliError { message, .. }) if message.contains("Invalid model") => {
            println!("Model not available, trying fallback...");
            query_with_model(prompt, "sonnet").await
        }
        Err(e) => Err(e)
    }
}

async fn query_with_model(prompt: &str, model: &str) -> Result<()> {
    let options = ClaudeCodeOptions::builder()
        .model(model)
        .build();
    
    let mut messages = query(prompt, Some(options)).await?;
    // Process messages...
    Ok(())
}
```

## Tips for Model Usage

1. **Always specify a model** - Don't rely on defaults as they may change
2. **Use aliases for flexibility** - `"opus"` and `"sonnet"` automatically use the latest versions
3. **Handle model unavailability** - Implement fallback logic for production systems
4. **Consider cost vs performance** - Opus 4.1 is most capable but may be slower/more expensive
5. **Test with different models** - Performance can vary based on task type

## Environment Variables

You can also set the default model via environment variables:

```bash
export CLAUDE_MODEL="opus-4.1"
```

Then in your code:
```rust
let model = std::env::var("CLAUDE_MODEL").unwrap_or_else(|_| "sonnet-4".to_string());
let options = ClaudeCodeOptions::builder()
    .model(model)
    .build();
```

## Version History

- **2025-08**: Opus 4.1 released (`claude-opus-4-1-20250805`)
- **2025-05**: Sonnet 4 released (`claude-sonnet-4-20250514`)
- **2024-10**: Claude 3.5 series (Sonnet, Haiku)

## See Also

- [README.md](../README.md) - Getting started guide
- [API Documentation](https://docs.rs/cc-sdk) - Full API reference
- [Examples](../examples/) - More code examples