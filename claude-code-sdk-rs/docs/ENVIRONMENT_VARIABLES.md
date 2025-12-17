# Environment Variables Guide

This guide covers all environment variables supported by the cc-sdk.

## Quick Reference

| Variable | Purpose | Required | Example |
|----------|---------|----------|---------|
| `ANTHROPIC_USER_EMAIL` | Account identification | **Recommended** | `user@example.com` |
| `CLAUDE_MODEL` | Default model | Optional | `claude-sonnet-4-5-20250929` |
| `CLAUDE_CODE_MAX_OUTPUT_TOKENS` | Max output tokens | Optional | `8192` |
| `CLAUDE_CODE_CLI_PATH` | CLI executable path | Optional | `/usr/local/bin/claude` |
| `RUST_LOG` | Logging level | Optional | `debug` |

## ANTHROPIC_USER_EMAIL ⭐ Recommended

**Purpose**: Identifies the current user account for SDK operations

**Type**: String (email address)
**Required**: Highly recommended for SDK mode
**Default**: None (will attempt config file detection)
**Example**: `john.doe@company.com`

### Why It's Important

In SDK/programmatic mode, the `/status` command doesn't work reliably like it does in interactive CLI. Setting this variable ensures:

- ✅ Reliable account identification
- ✅ Consistent logging and debugging
- ✅ Multi-account scenario support
- ✅ Audit trail for API usage

### Setting the Variable

**Shell (temporary):**
```bash
export ANTHROPIC_USER_EMAIL="your-email@example.com"
```

**Shell profile (permanent):**
```bash
# Add to ~/.bashrc or ~/.zshrc
echo 'export ANTHROPIC_USER_EMAIL="your-email@example.com"' >> ~/.bashrc
source ~/.bashrc
```

**.env file (recommended for projects):**
```bash
# .env
ANTHROPIC_USER_EMAIL=your-email@example.com
```

**Inline for single command:**
```bash
ANTHROPIC_USER_EMAIL="your-email@example.com" cargo run
```

### Usage in Code

```rust
use cc_sdk::{ClaudeSDKClient, ClaudeCodeOptions, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let mut client = ClaudeSDKClient::new(ClaudeCodeOptions::default());
    client.connect(None).await?;

    // Automatically uses ANTHROPIC_USER_EMAIL if set
    match client.get_account_info().await {
        Ok(info) => println!("Account: {}", info),
        Err(_) => println!("Set ANTHROPIC_USER_EMAIL environment variable"),
    }

    Ok(())
}
```

### Verification

Check if it's set:
```bash
echo $ANTHROPIC_USER_EMAIL
```

Test with example:
```bash
ANTHROPIC_USER_EMAIL="test@example.com" cargo run --example account_info
```

## CLAUDE_MODEL

**Purpose**: Specifies the default Claude model to use

**Type**: String (model identifier)
**Required**: Optional
**Default**: Uses Claude CLI default (usually latest Sonnet)
**Example**: `claude-sonnet-4-5-20250929`

### Supported Models

- `claude-sonnet-4-5-20250929` - Latest Sonnet 4.5 (recommended)
- `claude-opus-4-1-20250805` - Opus 4.1 (most capable)
- `claude-3-5-haiku-20241022` - Haiku 3.5 (fastest, cheapest)
- `sonnet` - Alias for latest Sonnet
- `opus` - Alias for latest Opus
- `haiku` - Alias for latest Haiku

### Setting the Variable

```bash
export CLAUDE_MODEL="claude-sonnet-4-5-20250929"
```

### Usage in Code

```rust
let model = std::env::var("CLAUDE_MODEL")
    .unwrap_or_else(|_| "claude-sonnet-4-5-20250929".to_string());

let options = ClaudeCodeOptions::builder()
    .model(model)
    .build();
```

## CLAUDE_CODE_MAX_OUTPUT_TOKENS

Controls the maximum number of output tokens that Claude CLI will generate in a single response.

### Valid Range
- **Minimum**: 1
- **Maximum Safe Value**: 32000
- **Recommended Default**: 8192

### Important Notes

1. **Maximum Limit**: The maximum safe value is **32000**. Values above this may cause:
   - Claude CLI to exit with error code 1
   - Timeouts or hanging processes
   - Unpredictable behavior

2. **SDK Protection**: As of v0.1.9, the Rust SDK automatically handles invalid values:
   - Values > 32000 are automatically capped at 32000
   - Non-numeric values are replaced with 8192
   - This ensures your application won't crash due to invalid settings

3. **Setting the Variable**:
   ```bash
   # Maximum safe value
   export CLAUDE_CODE_MAX_OUTPUT_TOKENS=32000
   
   # Conservative recommended value
   export CLAUDE_CODE_MAX_OUTPUT_TOKENS=8192
   
   # Remove the variable (use Claude's default)
   unset CLAUDE_CODE_MAX_OUTPUT_TOKENS
   ```

4. **Common Issues**:
   - `Error: Invalid env var CLAUDE_CODE_MAX_OUTPUT_TOKENS: 50000` - Value too high
   - Process exits immediately with code 1 - Invalid value
   - Timeouts during generation - Value may be too high

### Testing Your Configuration

You can test if your value works with:

```bash
CLAUDE_CODE_MAX_OUTPUT_TOKENS=32000 echo "Say hello" | claude --dangerously-skip-permissions
```

If the command hangs or errors, reduce the value.

## Other Environment Variables

### CLAUDE_CODE_CLI_PATH

**Purpose**: Override default Claude CLI executable location
**Type**: String (file path)
**Required**: Optional
**Default**: Searches system PATH

```bash
export CLAUDE_CODE_CLI_PATH="/opt/claude/bin/claude"
```

**Useful when:**
- Multiple Claude CLI versions installed
- Custom installation location
- Running in containers or custom environments

### RUST_LOG

**Purpose**: Control logging output verbosity
**Type**: String (log level)
**Required**: Optional
**Default**: `info`

```bash
# Enable debug logging for entire application
export RUST_LOG=debug

# Enable debug logging only for cc_sdk
export RUST_LOG=cc_sdk=debug

# Fine-grained control
export RUST_LOG=cc_sdk::client=debug,cc_sdk::transport=trace
```

**Log levels** (in order of verbosity):
- `error` - Only errors
- `warn` - Warnings and errors
- `info` - Informational messages (default)
- `debug` - Detailed debugging information
- `trace` - Very verbose tracing

### CLAUDE_CODE_ENTRYPOINT

**Purpose**: Internal marker for SDK usage
**Type**: String
**Required**: No (automatically set)
**Default**: `sdk-rust`

**Note**: Set automatically by the SDK. Do not manually set.

### Standard Variables

The SDK respects standard environment variables:
- `PATH` - To find the Claude CLI binary
- `HOME` - For locating configuration files
- Standard Node.js/npm environment variables

## Using .env Files

### Setup

**1. Create .env file:**
```bash
cp examples/.env.example .env
```

**2. Edit .env:**
```bash
# .env
ANTHROPIC_USER_EMAIL=your-email@example.com
CLAUDE_MODEL=claude-sonnet-4-5-20250929
CLAUDE_CODE_MAX_OUTPUT_TOKENS=8192
RUST_LOG=cc_sdk=info
```

**3. Add to .gitignore:**
```bash
echo ".env" >> .gitignore
```

### Integration with dotenv

**Add to Cargo.toml:**
```toml
[dependencies]
dotenv = "0.15"
```

**Load in code:**
```rust
use dotenv::dotenv;

#[tokio::main]
async fn main() {
    // Load .env file at startup
    dotenv().ok();

    // Now environment variables are available
    let email = std::env::var("ANTHROPIC_USER_EMAIL");
}
```

## Best Practices

### ✅ DO

1. **Always set ANTHROPIC_USER_EMAIL** for SDK usage
2. **Use .env files** for local development
3. **Add .env to .gitignore** to avoid committing secrets
4. **Validate environment variables** at startup
5. **Provide sensible defaults** for optional variables
6. **Document required variables** in your README

### ❌ DON'T

1. **Don't commit .env files** to version control
2. **Don't hard-code credentials** in source code
3. **Don't log sensitive environment variables**
4. **Don't use production credentials** in development
5. **Don't assume variables are always set** without checking

## Examples

### Complete Setup

```rust
use cc_sdk::{ClaudeCodeOptions, ClaudeSDKClient, Result};
use dotenv::dotenv;

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env file
    dotenv().ok();

    // Validate required variables
    if std::env::var("ANTHROPIC_USER_EMAIL").is_err() {
        eprintln!("Error: ANTHROPIC_USER_EMAIL not set");
        eprintln!("Please set in .env file or environment");
        std::process::exit(1);
    }

    // Read configuration from environment
    let model = std::env::var("CLAUDE_MODEL")
        .unwrap_or_else(|_| "claude-sonnet-4-5-20250929".to_string());

    let max_tokens = std::env::var("CLAUDE_CODE_MAX_OUTPUT_TOKENS")
        .ok()
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(8192);

    // Build options
    let options = ClaudeCodeOptions::builder()
        .model(model)
        .max_output_tokens(max_tokens)
        .build();

    // Create client
    let mut client = ClaudeSDKClient::new(options);
    client.connect(None).await?;

    // Verify account
    let account = client.get_account_info().await?;
    println!("Using account: {}", account);

    // ... rest of your code ...

    Ok(())
}
```

## Troubleshooting

### Account Info Not Available

**Problem**: `get_account_info()` fails

**Solution**:
```bash
export ANTHROPIC_USER_EMAIL="your-email@example.com"
echo $ANTHROPIC_USER_EMAIL  # Verify it's set
cargo run
```

### .env File Not Loading

**Problem**: Variables in .env not available

**Solution**:
```rust
// Make sure dotenv is loaded at the start
dotenv().ok();

// Check if loaded
if std::env::var("ANTHROPIC_USER_EMAIL").is_err() {
    eprintln!("Failed to load .env file");
}
```

### Model Not Being Used

**Problem**: CLAUDE_MODEL ignored

**Solution**:
```rust
// Make sure to read and use the variable
let model = std::env::var("CLAUDE_MODEL")
    .unwrap_or_else(|_| "claude-sonnet-4-5-20250929".to_string());

let options = ClaudeCodeOptions::builder()
    .model(model)  // Use the env var
    .build();
```

## See Also

- [Account Information Guide](ACCOUNT_INFO.md) - Account retrieval details
- [Model Selection Guide](models-guide.md) - Choosing the right model
- [Token Optimization](TOKEN_OPTIMIZATION.md) - Cost management
- [Examples](../examples/) - Working code examples