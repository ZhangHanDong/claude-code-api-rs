# Token Optimization Guide

Complete guide to minimizing token usage and costs when using Claude Code SDK.

## Problem: Weekly Token Limits

Claude Code has weekly usage limits that can be frustrating, especially for heavy users. This guide provides SDK-level strategies to help you:

- Reduce token consumption
- Monitor and control costs
- Choose cost-effective models
- Avoid hitting weekly limits

## Quick Start: Most Effective Strategies

```rust
use cc_sdk::{ClaudeCodeOptions, ClaudeSDKClient, ModelRecommendation};
use cc_sdk::token_tracker::BudgetLimit;

let options = ClaudeCodeOptions::builder()
    // 1. Use cheaper model for simple tasks
    .model("claude-3-5-haiku-20241022")  // ~15x cheaper than Opus

    // 2. Limit conversation length
    .max_turns(Some(3))

    // 3. Restrict output tokens
    .max_output_tokens(2000)  // vs default ~8000+

    // 4. Limit allowed tools (reduce tool call overhead)
    .allowed_tools(vec!["Read".to_string(), "Write".to_string()])

    // 5. Skip permission confirmations
    .permission_mode(PermissionMode::BypassPermissions)

    .build();

let mut client = ClaudeSDKClient::new(options);

// 6. Set budget limits with alerts
client.set_budget_limit(
    BudgetLimit::with_cost(5.0),  // $5 max
    Some(|msg| eprintln!("⚠️  {}", msg))
).await;
```

**Expected savings**: 80-90% token reduction vs. default configuration

## Strategy 1: Model Selection

### Cost Comparison

| Model | Relative Cost | Best For |
|-------|---------------|----------|
| **Haiku** (`claude-3-5-haiku-20241022`) | **1x** (baseline) | Simple tasks, fast responses |
| **Sonnet** (`sonnet`) | **~5x** | Balanced tasks, general use |
| **Opus** (`opus`) | **~15x** | Complex tasks, critical work |

### Using ModelRecommendation

```rust
use cc_sdk::ModelRecommendation;

let recommender = ModelRecommendation::default();

// Automatic recommendations
let model = recommender.suggest("simple").unwrap();  // → "claude-3-5-haiku-20241022"
let model = recommender.suggest("balanced").unwrap(); // → "sonnet"
let model = recommender.suggest("complex").unwrap();  // → "opus"

// Custom recommendations
let mut custom = ModelRecommendation::default();
custom.add("code_review", "sonnet");
custom.add("documentation", "claude-3-5-haiku-20241022");
```

### Rule of Thumb

- **Documentation, simple Q&A**: Haiku
- **Code generation, refactoring**: Sonnet
- **Architecture decisions, complex debugging**: Opus

##  Strategy 2: Output Token Limits

Control maximum response length to prevent verbose outputs:

```rust
let options = ClaudeCodeOptions::builder()
    .max_output_tokens(1000)  // Limit to 1000 tokens
    .build();
```

**Priority**: `max_output_tokens` option > `CLAUDE_CODE_MAX_OUTPUT_TOKENS` env var

```bash
# Also works via environment variable
export CLAUDE_CODE_MAX_OUTPUT_TOKENS=2000
```

**Safe range**: 1 - 32000 tokens

## Strategy 3: Conversation Length Control

Limit conversation turns to prevent long, expensive exchanges:

```rust
let options = ClaudeCodeOptions::builder()
    .max_turns(Some(3))  // Max 3 back-and-forth turns
    .build();
```

**Recommended values**:
- Quick tasks: 1-2 turns
- Standard queries: 3-5 turns
- Complex interactions: 5-10 turns

## Strategy 4: Tool Usage Control

Reduce overhead from tool calls:

```rust
let options = ClaudeCodeOptions::builder()
    // Only allow essential tools
    .allowed_tools(vec![
        "Read".to_string(),
        "Write".to_string(),
        "Bash".to_string(),
    ])

    // Or block expensive tools
    .disallowed_tools(vec![
        "WebSearch".to_string(),  // Can use many tokens
        "Task".to_string(),        // Spawns sub-agents
    ])

    .build();
```

## Strategy 5: Token Usage Monitoring

Track consumption and set budgets:

```rust
use cc_sdk::token_tracker::BudgetLimit;

let mut client = ClaudeSDKClient::new(options);

// Set budget with warning at 80%
client.set_budget_limit(
    BudgetLimit::with_both(10.0, 1_000_000),  // $10 or 1M tokens
    Some(|msg| {
        eprintln!("Budget warning: {}", msg);
        // Send alert, log to file, etc.
    })
).await;

// Check usage anytime
let usage = client.get_usage_stats().await;
println!("Tokens used: {} (input: {}, output: {})",
    usage.total_tokens(),
    usage.total_input_tokens,
    usage.total_output_tokens
);
println!("Cost: ${:.2}", usage.total_cost_usd);
println!("Avg per session: {:.0} tokens", usage.avg_tokens_per_session());

// Check if exceeded
if client.is_budget_exceeded().await {
    eprintln!("Budget exceeded! Stopping.");
    return;
}
```

### Budget Configuration Options

```rust
// Cost-based budget
let budget = BudgetLimit::with_cost(5.0);

// Token-based budget
let budget = BudgetLimit::with_tokens(500_000);

// Both limits
let budget = BudgetLimit::with_both(10.0, 1_000_000);

// Custom warning threshold (default 80%)
let budget = BudgetLimit::with_cost(5.0)
    .with_warning_threshold(0.9);  // Warn at 90%
```

## Strategy 6: Permission Mode

Skip interactive permission prompts to reduce back-and-forth:

```rust
let options = ClaudeCodeOptions::builder()
    .permission_mode(PermissionMode::BypassPermissions)
    .build();
```

**Warning**: Only use when you trust the code being generated/modified.

## Complete Optimized Example

```rust
use cc_sdk::{ClaudeCodeOptions, ClaudeSDKClient, PermissionMode, Result};
use cc_sdk::model_recommendation::ModelRecommendation;
use cc_sdk::token_tracker::BudgetLimit;
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Choose cost-effective model
    let recommender = ModelRecommendation::default();
    let model = recommender.suggest("simple").unwrap();

    // 2. Configure for minimal token usage
    let options = ClaudeCodeOptions::builder()
        .model(model)
        .max_turns(Some(2))
        .max_output_tokens(1500)
        .allowed_tools(vec!["Read".to_string()])
        .permission_mode(PermissionMode::BypassPermissions)
        .build();

    let mut client = ClaudeSDKClient::new(options);

    // 3. Set budget
    client.set_budget_limit(
        BudgetLimit::with_cost(1.0),
        Some(|msg| eprintln!("⚠️  {}", msg))
    ).await;

    // 4. Execute query
    client.connect(Some("What is 2+2?".to_string())).await?;

    let mut messages = client.receive_messages().await;
    while let Some(msg) = messages.next().await {
        println!("{:?}", msg?);
    }

    // 5. Check usage
    let usage = client.get_usage_stats().await;
    println!("\n📊 Usage: {} tokens, ${:.3}",
        usage.total_tokens(), usage.total_cost_usd);

    client.disconnect().await?;
    Ok(())
}
```

## Advanced: Multi-Account Management

While not an SDK feature, you can rotate accounts to bypass limits:

```rust
struct AccountPool {
    accounts: Vec<(String, String)>,  // (api_key, name)
    current: usize,
}

impl AccountPool {
    fn rotate(&mut self) -> &str {
        self.current = (self.current + 1) % self.accounts.len();
        &self.accounts[self.current].0
    }
}
```

**Note**: Check Anthropic's terms of service regarding multi-account usage.

## Common Pitfalls

### 1. ❌ Using Opus for Simple Tasks

```rust
// BAD: $0.15 for a simple calculation
.model("opus")
```

```rust
// GOOD: $0.01 for the same result
.model("claude-3-5-haiku-20241022")
```

### 2. ❌ No Output Limits

```rust
// BAD: Can generate 8000+ tokens
// (no max_output_tokens set)
```

```rust
// GOOD: Controlled output
.max_output_tokens(2000)
```

### 3. ❌ Allowing All Tools

```rust
// BAD: Claude may use expensive tools like WebSearch
// (no allowed_tools restriction)
```

```rust
// GOOD: Only essential tools
.allowed_tools(vec!["Read".to_string(), "Write".to_string()])
```

### 4. ❌ No Monitoring

```rust
// BAD: No idea how much you're spending
let client = ClaudeSDKClient::new(options);
```

```rust
// GOOD: Track and limit
client.set_budget_limit(BudgetLimit::with_cost(5.0), Some(alert)).await;
```

## Cost Estimation Formula

Approximate cost per query:

```
Cost ≈ (Input Tokens × Input Price) + (Output Tokens × Output Price)

Typical ranges:
- Haiku: $0.001 - $0.01 per query
- Sonnet: $0.005 - $0.05 per query
- Opus: $0.015 - $0.15 per query
```

## Real-World Savings Examples

### Example 1: Documentation Generation

**Before optimization**:
- Model: Opus
- No token limits
- Cost: ~$0.20 per file

**After optimization**:
```rust
.model("claude-3-5-haiku-20241022")
.max_output_tokens(3000)
.max_turns(Some(1))
```
- Cost: ~$0.02 per file
- **Savings: 90%**

### Example 2: Code Review

**Before**:
- Model: Sonnet
- All tools allowed
- Cost: ~$0.08 per review

**After**:
```rust
.model("sonnet")
.allowed_tools(vec!["Read".to_string()])
.max_turns(Some(2))
```
- Cost: ~$0.04 per review
- **Savings: 50%**

## Summary: Best Practices

1. ✅ **Always set `max_output_tokens`** (recommended: 1000-3000)
2. ✅ **Use Haiku for 80% of tasks** (upgrade only when needed)
3. ✅ **Limit `max_turns`** (1-3 for most queries)
4. ✅ **Restrict `allowed_tools`** (only what you need)
5. ✅ **Monitor with `TokenUsageTracker`**
6. ✅ **Set budgets** to prevent overspending
7. ✅ **Use `BypassPermissions`** for trusted operations

## Future Enhancements (Phase 2+)

Coming soon:
- Context compression strategies
- Tool call budgets
- Batch query optimization
- Automatic model downgrading when budget low

## Getting Help

- SDK Documentation: [README.md](../README.md)
- Examples: [examples/](../examples/)
- Issues: [GitHub Issues](https://github.com/anthropics/claude-code/issues)
