# url-preview Integration Guide

This guide explains how to integrate claude-code-api with the url-preview library.

## Overview

url-preview expects OpenAI-compatible API responses with `tool_calls` format (not the legacy `function_call` format). claude-code-api has been updated to support this.

## Key Issues and Solutions

### 1. Invalid Model Names

❌ **Problem**: url-preview examples use `claude-3-opus-20240229` which is invalid for Claude CLI.

✅ **Solution**: Use valid model names:
- `claude-3-5-haiku-20241022` (fastest)
- `claude-3-5-sonnet-20241022` (balanced)
- `opus` (most capable, alias for latest Opus)
- `claude-opus-4-20250514` (specific version)

### 2. Function Call Format

❌ **Problem**: url-preview expects `tool_calls` array, not `function_call` object.

✅ **Solution**: claude-code-api now detects when `tools` are used (vs `functions`) and returns the correct format:

```json
{
  "choices": [{
    "message": {
      "tool_calls": [{
        "id": "call_xxx",
        "type": "function",
        "function": {
          "name": "extract_data",
          "arguments": "{...}"
        }
      }]
    }
  }]
}
```

## Working Example

```rust
use url_preview::{LLMExtractor, LLMExtractorConfig, OpenAIProvider, Fetcher};
use std::sync::Arc;

// Configure for claude-code-api
let config = async_openai::config::OpenAIConfig::new()
    .with_api_base("http://localhost:8080/v1")
    .with_api_key("not-needed");

// Use a VALID model name
let provider = Arc::new(
    OpenAIProvider::from_config(config, "claude-3-5-haiku-20241022".to_string())
);

// Extract data
let extractor = LLMExtractor::new(provider);
let fetcher = Fetcher::new();
let result = extractor.extract::<YourSchema>(url, &fetcher).await?;
```

## Running the Integration

1. Start claude-code-api:
   ```bash
   RUST_LOG=info cargo run --bin ccapi
   ```

2. In url-preview project, update the model name in examples:
   ```rust
   // Change from:
   "claude-3-opus-20240229"
   // To:
   "opus" // or "claude-3-5-sonnet-20241022"
   ```

3. Run url-preview example:
   ```bash
   cargo run --example claude_api_working --features llm
   ```

## Troubleshooting

### "Invalid model name" Error
- Check the model name against the valid list above
- Use `opus` or `sonnet` aliases for simplicity

### "No function call or valid JSON in response" Error
- Make sure you're using the latest claude-code-api with tool_calls support
- Verify the API is returning tool_calls when tools are requested

### Timeout Errors
- Increase timeout in .env: `CLAUDE_CODE__CLAUDE__TIMEOUT_SECONDS=600`
- Use simpler schemas or smaller content

## API Request Format

url-preview sends requests like this:
```json
{
  "model": "claude-3-5-haiku-20241022",
  "messages": [...],
  "tools": [{
    "type": "function",
    "function": {
      "name": "extract_data",
      "description": "...",
      "parameters": {...}
    }
  }],
  "tool_choice": "required"
}
```

claude-code-api will:
1. Detect the `tools` array (not `functions`)
2. Process Claude's JSON response
3. Return it in `tool_calls` format (not `function_call`)

This ensures compatibility with url-preview's OpenAI provider implementation.