# Account Information Retrieval

This guide explains how to retrieve and display Claude account information in your applications using the cc-sdk.

## Overview

The `get_account_info()` method allows you to programmatically retrieve the current Claude account details through multiple methods:

1. **Environment Variable** (`ANTHROPIC_USER_EMAIL`) - Most reliable for SDK mode
2. **Config File** - Reads from Claude CLI configuration
3. **CLI Query** - Interactive `/status` command (may not work in SDK mode)

This is useful for:
- Verifying the correct account is being used
- Displaying account info in application UIs
- Logging session information
- Debugging authentication issues

## ⚠️ Important Notes

### Which Account Does cc-sdk Use?

**cc-sdk uses the same account that Claude CLI is currently logged in with.**

- Claude CLI stores authentication at the **system level** (in `~/.config/claude/` or similar)
- All terminal sessions share the **same authentication**
- When you use cc-sdk, it spawns the Claude CLI process, which uses the currently logged-in account
- **Not session-specific**: The account is system-wide, not per-terminal

**Example:**
```bash
# In any terminal, check current Claude account
echo "/status" | claude

# cc-sdk will use the SAME account
cargo run --example check_current_account
```

### ANTHROPIC_USER_EMAIL Purpose

The `ANTHROPIC_USER_EMAIL` environment variable is **NOT for authentication**. It's for:

1. **SDK identification**: Helps cc-sdk identify which account is being used (since `/status` doesn't work reliably in SDK mode)
2. **Logging/debugging**: Track which account made which API calls
3. **Multi-account scenarios**: Label SDK operations by account

**It does NOT change which account is used** - that's determined by Claude CLI's authentication.

### Switching Accounts

To use a different account with cc-sdk:

```bash
# Method 1: Interactive login (changes system-wide account)
claude
# Then use /login command in the interactive session

# Method 2: Logout and re-login
# (This is a Claude CLI operation, not cc-sdk specific)
```

### Recommended Setup

For reliable SDK operation:

```bash
export ANTHROPIC_USER_EMAIL="your-email@example.com"
```

This should match the account Claude CLI is logged in with.

## Basic Usage

```rust
use cc_sdk::{ClaudeSDKClient, ClaudeCodeOptions, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let mut client = ClaudeSDKClient::new(ClaudeCodeOptions::default());

    // Connect to Claude CLI
    client.connect(None).await?;

    // Get account information
    let account_info = client.get_account_info().await?;
    println!("Account: {}", account_info);

    client.disconnect().await?;
    Ok(())
}
```

## How It Works

The `get_account_info()` method attempts to retrieve account information through multiple fallback methods:

1. **Environment Variable Check**: First checks `ANTHROPIC_USER_EMAIL` environment variable
2. **Config File Reading**: Attempts to read from Claude CLI config files at:
   - `~/.config/claude/config.json`
   - `~/.claude/config.json`
3. **CLI Query**: As a last resort, sends `/status` command to Claude CLI

**Note**: This method requires an active connection. Make sure to call `connect()` before calling `get_account_info()`.

## Examples

### Example 1: Using Environment Variable (Recommended)

```rust
use cc_sdk::{ClaudeSDKClient, ClaudeCodeOptions, Result};

#[tokio::main]
async fn main() -> Result<()> {
    // Set environment variable before creating client
    std::env::set_var("ANTHROPIC_USER_EMAIL", "user@example.com");

    let mut client = ClaudeSDKClient::new(ClaudeCodeOptions::default());
    client.connect(None).await?;

    match client.get_account_info().await {
        Ok(info) => println!("✅ Account: {}", info),
        Err(e) => eprintln!("❌ Error: {}", e),
    }

    client.disconnect().await?;
    Ok(())
}
```

Or set it from your shell before running:

```bash
export ANTHROPIC_USER_EMAIL="user@example.com"
cargo run --example account_info
```

### Example 2: Simple Account Info Display with Fallback

```rust
use cc_sdk::{ClaudeSDKClient, ClaudeCodeOptions, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let mut client = ClaudeSDKClient::new(ClaudeCodeOptions::default());
    client.connect(None).await?;

    match client.get_account_info().await {
        Ok(info) => println!("✅ Account: {}", info),
        Err(e) => {
            eprintln!("❌ Error: {}", e);
            eprintln!("Tip: Set ANTHROPIC_USER_EMAIL environment variable");
        }
    }

    client.disconnect().await?;
    Ok(())
}
```

### Example 2: Session Startup with Account Verification

```rust
use cc_sdk::{ClaudeSDKClient, ClaudeCodeOptions, Result};

async fn start_session_with_verification() -> Result<()> {
    let options = ClaudeCodeOptions::builder()
        .model("claude-sonnet-4-5-20250929")
        .build();

    let mut client = ClaudeSDKClient::new(options);

    println!("🔌 Connecting...");
    client.connect(None).await?;

    // Display account info at session start
    println!("📋 Verifying account...");
    if let Ok(account_info) = client.get_account_info().await {
        println!("Current account:\n{}", account_info);
    }

    // Continue with session...

    Ok(())
}
```

### Example 3: Error Handling

```rust
use cc_sdk::{ClaudeSDKClient, ClaudeCodeOptions, Result, SdkError};

async fn get_account_with_fallback() -> Result<String> {
    let mut client = ClaudeSDKClient::new(ClaudeCodeOptions::default());

    client.connect(None).await?;

    match client.get_account_info().await {
        Ok(info) => Ok(info),
        Err(SdkError::InvalidState { message }) => {
            eprintln!("Warning: {}", message);
            Ok("Account info not available".to_string())
        }
        Err(e) => Err(e),
    }
}
```

### Example 4: Logging Account Info

```rust
use cc_sdk::{ClaudeSDKClient, ClaudeCodeOptions, Result};
use tracing::{info, error};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    let mut client = ClaudeSDKClient::new(ClaudeCodeOptions::default());
    client.connect(None).await?;

    match client.get_account_info().await {
        Ok(account_info) => {
            info!("Session started with account: {}", account_info);
        }
        Err(e) => {
            error!("Failed to retrieve account info: {}", e);
        }
    }

    // Continue with application logic...

    Ok(())
}
```

### Example 5: Multi-Account Verification

```rust
use cc_sdk::{ClaudeSDKClient, ClaudeCodeOptions, Result};

async fn verify_expected_account(expected_email: &str) -> Result<bool> {
    let mut client = ClaudeSDKClient::new(ClaudeCodeOptions::default());
    client.connect(None).await?;

    let account_info = client.get_account_info().await?;
    let is_correct = account_info.contains(expected_email);

    if !is_correct {
        eprintln!("⚠️  Warning: Expected account '{}', but got:", expected_email);
        eprintln!("{}", account_info);
    }

    client.disconnect().await?;
    Ok(is_correct)
}

#[tokio::main]
async fn main() -> Result<()> {
    let expected = "user@example.com";

    if verify_expected_account(expected).await? {
        println!("✅ Correct account verified");
    } else {
        println!("❌ Account mismatch detected");
    }

    Ok(())
}
```

## Common Use Cases

### 1. Application Startup Verification

Display account information when your application starts to ensure users are logged in with the correct account:

```rust
async fn initialize_app() -> Result<()> {
    let mut client = ClaudeSDKClient::new(ClaudeCodeOptions::default());
    client.connect(None).await?;

    println!("╔════════════════════════════════════╗");
    println!("║  Application Starting...          ║");
    println!("╠════════════════════════════════════╣");

    if let Ok(account) = client.get_account_info().await {
        println!("║  Account: {:<24}║", account);
    }

    println!("╚════════════════════════════════════╝");

    Ok(())
}
```

### 2. Debug Logging

Include account information in debug logs:

```rust
use tracing::debug;

async fn start_debug_session() -> Result<()> {
    let mut client = ClaudeSDKClient::new(ClaudeCodeOptions::default());
    client.connect(None).await?;

    if let Ok(account) = client.get_account_info().await {
        debug!("Session account: {}", account);
    }

    // Continue with session...
    Ok(())
}
```

### 3. Cost Tracking by Account

Track costs per account for billing or analytics:

```rust
use cc_sdk::token_tracker::BudgetLimit;

async fn track_usage_by_account() -> Result<()> {
    let mut client = ClaudeSDKClient::new(ClaudeCodeOptions::default());
    client.connect(None).await?;

    let account = client.get_account_info().await?;

    // Set budget tracking
    client.set_budget_limit(
        BudgetLimit::with_cost(10.0),
        Some(move |msg| {
            eprintln!("Account '{}': {}", account, msg);
        })
    ).await;

    // Continue with work...

    let usage = client.get_usage_stats().await;
    println!("Account '{}' used ${:.4}", account, usage.total_cost_usd);

    Ok(())
}
```

## Error Handling

The `get_account_info()` method can return the following errors:

### InvalidState Error

Occurs when the client is not connected:

```rust
match client.get_account_info().await {
    Err(SdkError::InvalidState { message }) => {
        eprintln!("Connection error: {}", message);
        // Make sure to call connect() first
    }
    _ => {}
}
```

### Empty Response

If no account information is returned:

```rust
match client.get_account_info().await {
    Ok(info) if info.is_empty() => {
        eprintln!("No account information available");
    }
    Ok(info) => println!("Account: {}", info),
    Err(e) => eprintln!("Error: {}", e),
}
```

## Best Practices

1. **Call After Connect**: Always ensure the client is connected before calling `get_account_info()`
   ```rust
   client.connect(None).await?;
   let account = client.get_account_info().await?;
   ```

2. **Handle Errors Gracefully**: Account info retrieval may fail; handle errors appropriately
   ```rust
   let account = client.get_account_info().await
       .unwrap_or_else(|_| "Unknown".to_string());
   ```

3. **Cache When Appropriate**: The account doesn't change during a session, so you can cache it
   ```rust
   let account_info = client.get_account_info().await?;
   // Store and reuse account_info instead of calling again
   ```

4. **Use for Verification**: Verify expected accounts in production environments
   ```rust
   let account = client.get_account_info().await?;
   assert!(account.contains("production@company.com"),
           "Wrong account! Expected production account.");
   ```

## Limitations

- Requires an active connection to Claude CLI
- The `/status` command must be supported by your Claude CLI version
- Account information format may vary depending on Claude CLI version
- Network latency may affect retrieval time

## See Also

- [Client API Documentation](https://docs.rs/cc-sdk/latest/cc_sdk/struct.ClaudeSDKClient.html)
- [Examples Directory](../examples/) - Complete working examples
- [Error Handling Guide](../README.md#error-handling)
