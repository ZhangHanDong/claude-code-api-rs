# Valid Model Names for Claude Code API

## Overview

Claude Code CLI uses specific model names that may differ from the standard Claude API model names. This document lists the valid model names that can be used with claude-code-api.

## Valid Model Names (2025)

### Full Model Names
- `claude-opus-4-1-20250805` - Claude Opus 4.1 (most capable, latest)
- `claude-sonnet-4-20250514` - Claude Sonnet 4 (balanced performance)
- `claude-3-5-sonnet-20241022` - Claude 3.5 Sonnet (previous generation)
- `claude-3-5-haiku-20241022` - Claude 3.5 Haiku (fastest)

### Model Aliases (Shortcuts)
- `opus-4.1` - Alias for Claude Opus 4.1
- `opus-4` - Alias for Claude Opus 4.x series
- `opus` - Alias for the latest Opus model (currently 4.1)
- `sonnet-4` - Alias for Claude Sonnet 4
- `sonnet` - Alias for the latest Sonnet model (currently 4)
- `haiku` - Alias for the latest Haiku model

## Invalid Model Names

The following model names will result in "Invalid model name" errors:
- `claude-3-opus-20240229` - Outdated/invalid format
- `haiku` - Alias not supported standalone
- `claude-haiku-4-20250514` - Not available

## Recommended Usage

For best compatibility:
1. Use the full model names listed above
2. Use aliases `opus` or `sonnet` for convenience
3. Avoid using outdated model name formats

## Example

```bash
# Valid requests
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-3-5-sonnet-20241022",
    "messages": [{"role": "user", "content": "Hello"}]
  }'

# Using alias
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "opus",
    "messages": [{"role": "user", "content": "Hello"}]
  }'
```

## Error Example

If you use an invalid model name like `claude-3-opus-20240229`, you'll get:
```json
{
  "error": {
    "type": "invalid_request_error",
    "message": "system: Invalid model name"
  }
}
```