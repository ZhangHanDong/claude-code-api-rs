# OpenAI Tool Calling Support

This document describes the OpenAI tool calling support in claude-code-api.

## Overview

Claude Code API supports OpenAI's modern `tools` format for AI tool integrations. The deprecated `functions` format is no longer supported.

## How It Works

When a request includes `tools` parameter, the API will:

1. Pass the user's message to Claude Code
2. Analyze Claude's response for JSON content
3. If JSON is detected that matches a requested tool, format it as a tool_calls response
4. Return the response in OpenAI's expected format

## Request Format

```json
{
  "model": "claude-3-5-haiku-20241022",
  "messages": [
    {
      "role": "user",
      "content": "Please preview this URL: https://example.com"
    }
  ],
  "tools": [
    {
      "type": "function",
      "function": {
        "name": "url_preview",
        "description": "Preview a URL and extract its content",
        "parameters": {
          "type": "object",
          "properties": {
            "url": {
              "type": "string",
              "description": "The URL to preview"
            }
          },
          "required": ["url"]
        }
      }
    }
  ],
  "tool_choice": "auto"
}
```

## Response Format

When a tool call is detected, the response will include:

```json
{
  "id": "chatcmpl-...",
  "object": "chat.completion",
  "created": 1234567890,
  "model": "claude-3-5-haiku-20241022",
  "choices": [
    {
      "index": 0,
      "message": {
        "role": "assistant",
        "content": null,
        "tool_calls": [
          {
            "id": "call_xxx",
            "type": "function",
            "function": {
              "name": "url_preview",
              "arguments": "{\"url\": \"https://example.com\"}"
            }
          }
        ]
      },
      "finish_reason": "stop"
    }
  ],
  "usage": {
    "prompt_tokens": 0,
    "completion_tokens": 50,
    "total_tokens": 50
  }
}
```

## Integration Examples

### url-preview Integration

The tool calling support was designed to work seamlessly with libraries like url-preview:

```rust
use url_preview::{LLMExtractor, OpenAIProvider};
use async_openai::config::OpenAIConfig;

// Configure for claude-code-api
let config = OpenAIConfig::new()
    .with_api_base("http://localhost:8080/v1")
    .with_api_key("not-needed");

let provider = OpenAIProvider::from_config(
    config, 
    "claude-3-5-haiku-20241022".to_string()
);
```

### Python Example

```python
from openai import OpenAI

client = OpenAI(
    base_url="http://localhost:8080/v1",
    api_key="not-needed"
)

response = client.chat.completions.create(
    model="claude-3-5-haiku-20241022",
    messages=[{"role": "user", "content": "Get weather in Beijing"}],
    tools=[{
        "type": "function",
        "function": {
            "name": "get_weather",
            "description": "Get weather for a location",
            "parameters": {
                "type": "object",
                "properties": {
                    "location": {"type": "string"}
                },
                "required": ["location"]
            }
        }
    }]
)

# Access tool calls
if response.choices[0].message.tool_calls:
    tool_call = response.choices[0].message.tool_calls[0]
    print(f"Tool: {tool_call.function.name}")
    print(f"Args: {tool_call.function.arguments}")
```

## JSON Detection

The API uses multiple strategies to detect JSON in Claude's responses:

1. Direct JSON parsing of the entire response
2. JSON within markdown code blocks (```json)
3. Embedded JSON objects within text

## Valid Model Names

Use these model names with claude-code-api:
- `claude-3-5-haiku-20241022` (fastest)
- `claude-3-5-sonnet-20241022` (balanced)
- `opus` (most capable, alias)
- `sonnet` (alias)

## Limitations

- Tool calling is only supported in non-streaming mode
- The API does not execute tools; it only formats responses as tool calls
- Complex nested tool parameters may require additional validation

## Testing

Use the provided test script to verify tool calling:

```bash
./test_tool_format.sh
```