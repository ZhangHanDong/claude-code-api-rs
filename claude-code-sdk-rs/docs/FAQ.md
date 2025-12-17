# Frequently Asked Questions (FAQ)

## Account & Authentication

### Q: Which account does cc-sdk use when I run it?

**A:** cc-sdk uses **the same account that Claude CLI is currently logged in with** on your system.

- Claude CLI stores authentication at the system level (not per-terminal)
- All terminal sessions, programs, and cc-sdk instances share the same authentication
- When cc-sdk runs, it spawns the Claude CLI process, which inherits the system-wide login

**To verify which account is active:**
```bash
# Check in terminal
echo "/status" | claude

# Or use cc-sdk
cargo run --example check_current_account
```

### Q: Can I use different accounts in different terminal sessions?

**A: No.** Claude CLI authentication is **system-wide**, not session-specific.

- All terminals use the same logged-in account
- If you switch accounts in one terminal (via `/login`), it affects ALL terminals
- This is a Claude CLI design, not a cc-sdk limitation

**Workaround for multi-account scenarios:**
- Use different system user accounts (Linux/macOS users)
- Use containers/VMs for isolation
- Use the `ANTHROPIC_USER_EMAIL` environment variable to **label** which account you intend to use (though it won't change the actual account)

### Q: What does ANTHROPIC_USER_EMAIL do?

**A:** The `ANTHROPIC_USER_EMAIL` environment variable is for **identification**, not authentication.

**What it DOES:**
- ✅ Helps cc-sdk identify which account is being used (for logging/debugging)
- ✅ Enables `get_account_info()` to work reliably in SDK mode
- ✅ Labels API operations by account in logs
- ✅ Useful in multi-account environments for tracking

**What it DOES NOT do:**
- ❌ Does NOT change which account is used
- ❌ Does NOT authenticate you
- ❌ Does NOT bypass Claude CLI authentication

**The actual account used is determined by Claude CLI's authentication state.**

### Q: How do I switch to a different account?

**A:** Use Claude CLI's login functionality:

```bash
# Method 1: Interactive
claude
# Then in the interactive session:
/login

# Method 2: Check current account first
echo "/status" | claude

# Method 3: Logout completely (if needed)
# Then login again with different credentials
```

**Note:** This is a Claude CLI operation and affects all terminals and cc-sdk sessions system-wide.

### Q: Why does get_account_info() fail or return wrong information?

**A:** In SDK/programmatic mode, the `/status` command doesn't work reliably.

**Solution:**
```bash
# Set this environment variable to match your Claude CLI account
export ANTHROPIC_USER_EMAIL="your-actual-email@example.com"
```

**This should match the account that Claude CLI is logged in with.**

## Environment Variables

### Q: Where should I set environment variables?

**A:** It depends on your use case:

**Development (recommended):**
```bash
# Create .env file in project root
cat > .env << EOF
ANTHROPIC_USER_EMAIL=dev@example.com
CLAUDE_MODEL=claude-sonnet-4-5-20250929
EOF

# Don't commit .env to git!
echo ".env" >> .gitignore
```

**Permanent (all projects):**
```bash
# Add to ~/.bashrc or ~/.zshrc
echo 'export ANTHROPIC_USER_EMAIL="your@email.com"' >> ~/.bashrc
source ~/.bashrc
```

**Temporary (single command):**
```bash
ANTHROPIC_USER_EMAIL="user@example.com" cargo run
```

### Q: Do I need to use dotenv crate?

**A:** No, but it's convenient.

**Without dotenv:**
```bash
# Set in shell before running
export ANTHROPIC_USER_EMAIL="user@example.com"
cargo run
```

**With dotenv:**
```rust
use dotenv::dotenv;

fn main() {
    dotenv().ok();  // Loads .env file
    // Now env vars are available
}
```

## Model Selection

### Q: How do I use Claude Sonnet 4.5?

**A:** Multiple ways:

```rust
// Method 1: Directly in code
let options = ClaudeCodeOptions::builder()
    .model("claude-sonnet-4-5-20250929")
    .build();

// Method 2: Using helper function
use cc_sdk::model_recommendation::latest_sonnet;
let options = ClaudeCodeOptions::builder()
    .model(latest_sonnet())
    .build();

// Method 3: Via environment variable
// In .env or shell:
// export CLAUDE_MODEL="claude-sonnet-4-5-20250929"
let model = std::env::var("CLAUDE_MODEL")
    .unwrap_or_else(|_| "claude-sonnet-4-5-20250929".to_string());
let options = ClaudeCodeOptions::builder()
    .model(model)
    .build();
```

### Q: What's the difference between model names and aliases?

**A:**

**Full names (specific versions):**
- `claude-sonnet-4-5-20250929` - Sonnet 4.5 (latest)
- `claude-opus-4-1-20250805` - Opus 4.1
- `claude-3-5-haiku-20241022` - Haiku 3.5

**Aliases (may change over time):**
- `sonnet` - Latest Sonnet (currently points to Sonnet 4.5)
- `opus` - Latest Opus (currently points to Opus 4.1)
- `haiku` - Latest Haiku

**Recommendation:** Use full names for reproducibility, aliases for always-latest.

## Common Issues

### Q: "No account information received" error

**Problem:** `get_account_info()` returns error about no account information.

**Cause:** `/status` command doesn't work in SDK mode.

**Solution:**
```bash
export ANTHROPIC_USER_EMAIL="your-email@example.com"
```

**Verify it's set:**
```bash
echo $ANTHROPIC_USER_EMAIL
```

### Q: SDK using wrong account

**Problem:** cc-sdk seems to use a different account than expected.

**Diagnosis:**
```bash
# Check which account Claude CLI is using
echo "/status" | claude

# cc-sdk MUST use the same account
cargo run --example check_current_account
```

**Common causes:**
1. **Multiple Claude CLI installations** - Check which one is being used
2. **ANTHROPIC_USER_EMAIL mismatch** - This variable is for identification only, not authentication
3. **Shared system** - Someone else changed the logged-in account

**Solution:**
- Verify Claude CLI account: `echo "/status" | claude`
- If wrong, login to correct account: `claude` then `/login`
- Set ANTHROPIC_USER_EMAIL to match: `export ANTHROPIC_USER_EMAIL="correct@email.com"`

### Q: "Connection refused" or "Command not found"

**Problem:** Can't connect to Claude CLI

**Possible causes:**

1. **Claude CLI not installed**
   ```bash
   # Install it
   npm install -g @anthropic-ai/claude-code
   ```

2. **Claude CLI not in PATH**
   ```bash
   # Find it
   which claude

   # If not found, add to PATH or set explicitly
   export CLAUDE_CODE_CLI_PATH="/path/to/claude"
   ```

3. **Permissions issue**
   ```bash
   # Make sure it's executable
   chmod +x $(which claude)
   ```

## Performance & Cost

### Q: How can I reduce token usage and costs?

**A:** See the [Token Optimization Guide](TOKEN_OPTIMIZATION.md), but here are quick tips:

```rust
use cc_sdk::model_recommendation::cheapest_model;
use cc_sdk::token_tracker::BudgetLimit;

let options = ClaudeCodeOptions::builder()
    .model(cheapest_model())  // Use Haiku
    .max_output_tokens(2000)  // Limit response length
    .max_turns(3)             // Limit conversation length
    .build();

let mut client = ClaudeSDKClient::new(options);

// Set budget
client.set_budget_limit(
    BudgetLimit::with_cost(5.0),  // $5 maximum
    Some(|msg| eprintln!("⚠️ {}", msg))
).await;
```

### Q: Which model should I use?

**A:** Depends on your needs:

| Use Case | Recommended Model | Why |
|----------|------------------|-----|
| Most tasks | Sonnet 4.5 | Best balance of quality/speed/cost |
| Simple queries | Haiku 3.5 | Fastest, cheapest |
| Complex reasoning | Opus 4.1 | Most capable |
| High volume | Haiku 3.5 | Cost-effective at scale |
| Production critical | Sonnet 4.5 | Reliable, high quality |

## Development

### Q: How do I enable debug logging?

**A:**
```bash
# For entire application
export RUST_LOG=debug

# Only for cc-sdk
export RUST_LOG=cc_sdk=debug

# Fine-grained
export RUST_LOG=cc_sdk::client=debug,cc_sdk::transport=trace
```

### Q: Can I use this in production?

**A:** Yes, but consider:

**✅ Production Ready:**
- Stable API (v0.3.0+)
- Comprehensive error handling
- Token usage tracking
- Budget limits

**⚠️ Important for Production:**
- Set `ANTHROPIC_USER_EMAIL` for account identification
- Use `.env` files, not hard-coded credentials
- Implement proper error handling
- Monitor token usage and costs
- Set budget limits
- Use appropriate log levels (not `debug` in production)

**Example production setup:**
```rust
use cc_sdk::{ClaudeSDKClient, ClaudeCodeOptions, Result};
use cc_sdk::token_tracker::BudgetLimit;

async fn production_setup() -> Result<ClaudeSDKClient> {
    // Validate environment
    let account_email = std::env::var("ANTHROPIC_USER_EMAIL")
        .expect("ANTHROPIC_USER_EMAIL must be set in production");

    // Configure with limits
    let options = ClaudeCodeOptions::builder()
        .model("claude-sonnet-4-5-20250929")
        .max_output_tokens(4000)
        .max_turns(10)
        .build();

    let mut client = ClaudeSDKClient::new(options);

    // Set budget
    client.set_budget_limit(
        BudgetLimit::with_cost(100.0),
        Some(|msg| {
            eprintln!("[BUDGET] {}", msg);
            // Send alert to monitoring system
        })
    ).await;

    Ok(client)
}
```

## Troubleshooting

### Q: Where can I find more help?

**A:**

1. **Documentation:**
   - [README](../README.md) - Getting started
   - [Environment Variables](ENVIRONMENT_VARIABLES.md) - Configuration
   - [Account Info](ACCOUNT_INFO.md) - Account management
   - [Model Guide](models-guide.md) - Model selection
   - [Token Optimization](TOKEN_OPTIMIZATION.md) - Cost reduction

2. **Examples:**
   - See `examples/` directory for working code
   - Run examples: `cargo run --example <name>`

3. **Testing:**
   ```bash
   # Run all tests
   cargo test

   # Run specific example
   cargo run --example check_current_account
   ```

4. **Report Issues:**
   - Check GitHub issues
   - Create new issue with reproduction steps
   - Include: OS, Claude CLI version, SDK version, error messages

### Q: How do I check my SDK version?

**A:**
```bash
# In Cargo.toml
grep "cc-sdk" Cargo.toml

# Or check installed version
cargo tree | grep cc-sdk
```

**Latest version:** Check [crates.io](https://crates.io/crates/cc-sdk)

## See Also

- [Environment Variables Guide](ENVIRONMENT_VARIABLES.md)
- [Account Information Guide](ACCOUNT_INFO.md)
- [Model Selection Guide](models-guide.md)
- [Token Optimization](TOKEN_OPTIMIZATION.md)
- [Examples Directory](../examples/)
