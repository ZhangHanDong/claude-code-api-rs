# url-preview Troubleshooting Guide

## Common Issues and Solutions

### 1. "Invalid model name" Error

**Problem**: 
```
Error: API Error: 400 {"type":"error","error":{"type":"invalid_request_error","message":"system: Invalid model name"}}
```

**Cause**: Using `claude-3-opus-20240229` which is not a valid model name for Claude CLI.

**Solution**: Use one of these valid model names:
- `claude-3-5-haiku-20241022`
- `claude-3-5-sonnet-20241022` 
- `opus` (alias for latest Opus)
- `sonnet` (alias for latest Sonnet)

**Fix in url-preview**:
```rust
// Change from:
"claude-3-opus-20240229"
// To:
"opus" // or any other valid model
```

### 2. "No function call or valid JSON in response" Error

**Problem**:
```
Error: External service error: OpenAI - No function call or valid JSON in response
```

**Cause**: The response doesn't contain `tool_calls` array that url-preview expects.

**Possible Reasons**:
1. Claude didn't return JSON in its response
2. The JSON detection failed
3. Old version of claude-code-api that only supports function_call

**Solution**:
1. Make sure you're using the latest claude-code-api with tool_calls support
2. Check claude-code-api logs to see what Claude is returning
3. Try a simpler prompt to test JSON extraction

### 3. Connection Refused

**Problem**:
```
Error: Connection refused (os error 61)
```

**Solution**: Start claude-code-api:
```bash
cargo run --bin ccapi
# or
./target/release/ccapi
```

### 4. Timeout Errors

**Problem**: Request times out after 30-60 seconds

**Solution**: 
1. Set longer timeout in `.env`:
   ```
   CLAUDE_CODE__CLAUDE__TIMEOUT_SECONDS=600
   ```
2. Use a faster model (haiku)
3. Reduce content size or simplify schema

## Debugging Steps

### 1. Test claude-code-api Directly

```bash
# Test if API is running
curl http://localhost:8080/health

# Test tool calling
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-3-5-haiku-20241022",
    "messages": [{"role": "user", "content": "Return JSON: {\"test\": \"hello\"}"}],
    "tools": [{
      "type": "function",
      "function": {
        "name": "test",
        "description": "Test function",
        "parameters": {"type": "object", "properties": {"test": {"type": "string"}}}
      }
    }]
  }' | jq '.'
```

### 2. Check Response Format

The response should have this structure:
```json
{
  "choices": [{
    "message": {
      "tool_calls": [{
        "id": "call_xxx",
        "type": "function",
        "function": {
          "name": "extract_data",
          "arguments": "{\"title\": \"...\", ...}"
        }
      }]
    }
  }]
}
```

### 3. Enable Debug Logging

```bash
# Run claude-code-api with debug logging
RUST_LOG=debug cargo run --bin ccapi

# Run url-preview with debug logging
RUST_LOG=url_preview=debug cargo run --example claude_api_working --features llm
```

### 4. Check url-preview Code

url-preview checks for responses in this order:
1. `message.tool_calls` (preferred)
2. `message.content` with JSON extraction (fallback)

Make sure your version of url-preview has this fallback logic.

## Working Configuration

**claude-code-api `.env`**:
```env
CLAUDE_CODE__SERVER__PORT=8080
CLAUDE_CODE__CLAUDE__COMMAND=claude
CLAUDE_CODE__CLAUDE__TIMEOUT_SECONDS=600
RUST_LOG=claude_code_api=info
```

**url-preview code**:
```rust
let config = OpenAIConfig::new()
    .with_api_base("http://localhost:8080/v1")
    .with_api_key("not-needed");

let provider = OpenAIProvider::from_config(
    config,
    "claude-3-5-haiku-20241022".to_string() // Valid model!
);
```

## Testing Script

Save this as `test_url_preview.sh`:
```bash
#!/bin/bash
./test_url_preview_integration.sh
```

This will test the complete integration and show you exactly what's happening.