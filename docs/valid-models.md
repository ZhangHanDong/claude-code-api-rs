# Valid Model Names for Claude Code API

## Overview

Claude Code CLI uses specific model names that may differ from the standard Claude API model names. This document lists the valid model names that can be used with claude-code-api.

## Valid Model Names (2025)

### Recommended Aliases
These aliases automatically use the latest version of each model family:
- `opus` - Latest Opus model (currently 4.1)
- `sonnet` - Latest Sonnet model (currently 4)

### Full Model Names
- `claude-opus-4-1-20250805` - Claude Opus 4.1 (most capable)
- `claude-sonnet-4-20250514` - Claude Sonnet 4 (balanced performance)
- `claude-3-5-sonnet-20241022` - Claude 3.5 Sonnet (previous generation)
- `claude-3-5-haiku-20241022` - Claude 3.5 Haiku (fastest)

## Invalid Model Names

The following model names will result in 404 "not_found_error":
- `opus-4.1` - Short alias not supported (use `opus` instead)
- `opus-4` - Short alias not supported (use `opus` instead)
- `sonnet-4` - Short alias not supported (use `sonnet` instead)
- `claude-3-opus-20240229` - Outdated/invalid format

## Recommended Usage

For best compatibility:
1. Use the full model names listed above
2. Use aliases `opus` or `sonnet` for convenience
3. Avoid using outdated model name formats

## Examples

### ✅ Valid Requests

```bash
# Using recommended alias (BEST PRACTICE)
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "opus",
    "messages": [{"role": "user", "content": "Hello"}]
  }'

# Using full model name
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-opus-4-1-20250805",
    "messages": [{"role": "user", "content": "Hello"}]
  }'
```

### ❌ Invalid Requests

If you use an invalid model name like `opus-4.1` or `sonnet-4`, you'll get:
```json
{
  "error": {
    "type": "invalid_request_error",
    "message": "system: Invalid model name"
  }
}
```