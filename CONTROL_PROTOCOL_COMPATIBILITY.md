# Control Protocol Compatibility Guide

## Overview

The Rust SDK supports both legacy and new control protocol formats to ensure compatibility with different versions of the Claude Code CLI.

## Protocol Formats

### Legacy Format (Default)
- **Request**: `{"type": "sdk_control_request", "request": {...}}`
- **Response**: `{"type": "sdk_control_response", "response": {...}}`
- **Compatibility**: All CLI versions
- **Use Case**: Maximum compatibility with existing deployments

### New Format
- **Request/Response**: `{"type": "control", "control": {...}}`
- **Compatibility**: Newer CLI versions only
- **Use Case**: Future-proof, unified protocol

## Configuration

### 1. Programmatic Configuration

```rust
use cc_sdk::{ClaudeCodeOptions, ControlProtocolFormat};

let options = ClaudeCodeOptions::builder()
    .control_protocol_format(ControlProtocolFormat::Legacy)  // Default
    // or ControlProtocolFormat::Control for new format
    // or ControlProtocolFormat::Auto for future auto-detection
    .build();
```

### 2. Environment Variable

Set `CLAUDE_CODE_CONTROL_FORMAT` to override programmatic settings:

```bash
export CLAUDE_CODE_CONTROL_FORMAT=legacy  # Use legacy format
# or
export CLAUDE_CODE_CONTROL_FORMAT=control  # Use new format
```

## Receiving Messages

The SDK **always** supports both formats when receiving messages (dual-stack):
- Automatically detects and handles `type=control`
- Also recognizes legacy `type=sdk_control_request` and `system sdk_control:*` variants

## Migration Strategy

1. **Current (v0.1.10+)**: Default to legacy format for maximum compatibility
2. **Check CLI Version**: Verify your CLI supports new format before switching
3. **Gradual Migration**: Use environment variable for testing without code changes
4. **Future**: Auto-detection will negotiate the best format automatically

## Compatibility Matrix

| SDK Version | Default Format | Supports Legacy | Supports New |
|-------------|----------------|-----------------|--------------|
| v0.1.10+    | Legacy         | ✅ Send/Receive | ✅ Receive, ⚠️ Send (opt-in) |
| Future      | Auto           | ✅ Send/Receive | ✅ Send/Receive |

## Testing Your Configuration

Run the included example to verify your setup:

```bash
cargo run --example control_format_demo
```

## Troubleshooting

### Symptoms of Format Mismatch
- Permission callbacks not triggered
- Hook callbacks not executed  
- MCP server messages not delivered
- Control requests appear to be ignored

### Solution
1. Ensure you're using legacy format (default)
2. Or verify your CLI version supports new format before switching
3. Check logs for "Unknown message type" errors from CLI

## Python SDK Compatibility

This implementation follows the same strategy as the Python SDK:
- Default to legacy format for compatibility
- Provide options for new format when CLI is ready
- Environment variable override for flexibility