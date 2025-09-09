# cc-sdk Integration Plan for claude-code-api

## Current State

- **claude-code-api**: OpenAI-compatible API server with custom Claude CLI integration
- **cc-sdk**: Standalone SDK for Claude CLI with full control protocol support

Both packages currently have independent implementations for interacting with Claude CLI.

## Benefits of Integration

1. **Code Reuse**: Eliminate duplicate CLI interaction logic
2. **Feature Parity**: Automatically get control protocol, permissions, hooks support
3. **Maintenance**: Single source of truth for CLI interaction
4. **Stability**: cc-sdk has comprehensive testing and error handling

## Integration Approach

### Option 1: Full Migration (Recommended)

Replace custom CLI handling in claude-code-api with cc-sdk:

```toml
# claude-code-api/Cargo.toml
[dependencies]
cc-sdk = { path = "../claude-code-sdk-rs", version = "0.1.11" }
```

**Changes needed:**

1. Replace `ClaudeManager` with `cc_sdk::ClaudeSDKClient`
2. Replace `SessionProcess` with cc-sdk's interactive client
3. Use cc-sdk's message types and streaming

**Example migration:**

```rust
// Before: Custom implementation
let mut cmd = Command::new(&self.claude_command);
cmd.arg("--output-format").arg("stream-json");

// After: Using cc-sdk
use cc_sdk::{ClaudeSDKClient, ClaudeCodeOptions};

let options = ClaudeCodeOptions::builder()
    .model(model)
    .cwd(project_path)
    .control_protocol_format(ControlProtocolFormat::Legacy)
    .build();

let mut client = ClaudeSDKClient::new(options);
client.connect(Some(prompt)).await?;
```

### Option 2: Gradual Migration

Keep existing implementation but use cc-sdk for new features:

1. Add cc-sdk dependency
2. Use cc-sdk for control protocol features
3. Gradually migrate other components
4. Eventually deprecate custom implementation

### Option 3: Keep Separate (Current)

Maintain two separate implementations:

- **Pros**: No breaking changes, independent development
- **Cons**: Duplicate code, maintenance overhead, feature divergence

## Recommended Actions

1. **Immediate**: Update workspace version to 0.1.11 to match cc-sdk
2. **Short-term**: Add cc-sdk as optional dependency for testing
3. **Medium-term**: Implement Option 1 (Full Migration) in a feature branch
4. **Long-term**: Deprecate custom CLI handling code

## Migration Checklist

- [ ] Update workspace version to 0.1.11
- [ ] Add cc-sdk dependency to claude-code-api
- [ ] Create feature branch for migration
- [ ] Replace ClaudeManager with cc-sdk client
- [ ] Update message handling to use cc-sdk types
- [ ] Migrate streaming functionality
- [ ] Update error handling
- [ ] Add control protocol features (permissions, hooks)
- [ ] Update tests
- [ ] Performance benchmarking
- [ ] Documentation update

## Risk Assessment

- **Low Risk**: cc-sdk is well-tested and stable
- **Medium Risk**: API changes may affect existing users
- **Mitigation**: Use feature flags for gradual rollout

## Timeline Estimate

- Full migration: 2-3 days
- Testing and validation: 1-2 days
- Documentation: 1 day

Total: ~1 week for complete migration