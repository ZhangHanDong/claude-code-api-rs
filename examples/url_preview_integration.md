# url-preview Integration Example

This example shows how [url-preview](https://github.com/ZhangHanDong/url-preview) v0.6.0 integrates with claude-code-api to extract structured data from web pages using Claude.

## Overview

url-preview is a Rust library that extracts structured data from web pages using LLMs. With version 0.6.0, it added support for Claude through claude-code-api, allowing users to leverage Claude's capabilities for web content extraction.

## How url-preview Uses claude-code-api

### 1. Configuration

url-preview uses the OpenAI provider with a custom base URL pointing to claude-code-api:

```rust
use url_preview::{LLMExtractor, OpenAIProvider};
use async_openai::config::OpenAIConfig;

// Configure to use claude-code-api instead of OpenAI
let config = OpenAIConfig::new()
    .with_api_base("http://localhost:8080/v1")
    .with_api_key("not-needed"); // claude-code-api doesn't require an API key

// Use a valid Claude model
let provider = OpenAIProvider::from_config(
    config,
    "claude-3-5-haiku-20241022".to_string()
);
```

### 2. Extracting Structured Data

url-preview sends requests in OpenAI's tools format, which claude-code-api translates for Claude:

```rust
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct ProductInfo {
    title: String,
    price: f64,
    description: String,
    availability: bool,
}

// Extract product information from a webpage
let extractor = LLMExtractor::new(Arc::new(provider));
let fetcher = Fetcher::new();

let result = extractor.extract::<ProductInfo>(
    "https://example.com/product",
    &fetcher
).await?;

println!("Product: {}", result.data.title);
println!("Price: ${}", result.data.price);
```

### 3. Behind the Scenes

1. **url-preview** fetches the webpage content
2. **url-preview** sends an OpenAI-compatible request with tools format to claude-code-api
3. **claude-code-api** translates this to Claude's format
4. **Claude** analyzes the content and returns structured JSON
5. **claude-code-api** formats the response as tool_calls
6. **url-preview** extracts the structured data from tool_calls

## Example Request Flow

### url-preview sends:
```json
{
  "model": "claude-3-5-haiku-20241022",
  "messages": [
    {
      "role": "system",
      "content": "You are a helpful assistant that extracts structured data from web content."
    },
    {
      "role": "user",
      "content": "Extract product information from: [webpage content]"
    }
  ],
  "tools": [{
    "type": "function",
    "function": {
      "name": "extract_data",
      "description": "Extract structured data from the content",
      "parameters": {
        "type": "object",
        "properties": {
          "title": {"type": "string"},
          "price": {"type": "number"},
          "description": {"type": "string"},
          "availability": {"type": "boolean"}
        }
      }
    }
  }]
}
```

### claude-code-api returns:
```json
{
  "choices": [{
    "message": {
      "tool_calls": [{
        "id": "call_xxx",
        "type": "function",
        "function": {
          "name": "extract_data",
          "arguments": "{\"title\":\"Example Product\",\"price\":29.99,\"description\":\"A great product\",\"availability\":true}"
        }
      }]
    }
  }]
}
```

## Benefits of This Integration

1. **No API Key Required** - Unlike OpenAI, claude-code-api uses your local Claude CLI
2. **Cost Effective** - Uses your existing Claude subscription
3. **Privacy** - Data stays local, no external API calls
4. **Flexibility** - Switch between Claude and OpenAI with just a config change
5. **Tool Calling Support** - Full compatibility with OpenAI's tools format

## Running the Example

1. Start claude-code-api:
   ```bash
   cargo run --bin ccapi
   ```

2. In the url-preview project:
   ```bash
   cargo run --example claude_api_working --features llm
   ```

## Common Issues

- **Model Name**: Make sure to use valid model names like `claude-3-5-haiku-20241022`, not `claude-3-opus-20240229`
- **Tool Calls**: claude-code-api returns `tool_calls` array, not the deprecated `function_call`
- **Timeout**: For complex pages, increase timeout: `CLAUDE_CODE__CLAUDE__TIMEOUT_SECONDS=600`

## Learn More

- [url-preview Documentation](https://github.com/ZhangHanDong/url-preview)
- [claude-code-api Documentation](https://github.com/ZhangHanDong/claude-code-api-rs)
- [OpenAI Tools Format](https://platform.openai.com/docs/guides/function-calling)