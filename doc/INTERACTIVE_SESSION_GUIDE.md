# Interactive Session Guide

## Overview

The Interactive Session feature allows the Claude Code API to reuse the same Claude process for multiple requests within the same conversation. This significantly improves performance by avoiding the overhead of creating new processes for each request.

## Benefits

1. **Faster Response Times**: After the initial request, subsequent requests in the same conversation are much faster (typically 1-3 seconds vs 5-15 seconds)
2. **Context Preservation**: The Claude process maintains context between requests, enabling true multi-turn conversations
3. **Resource Efficiency**: Reduces system resource usage by reusing processes instead of creating new ones

## Configuration

Interactive sessions are enabled by default. You can control this feature through configuration:

### In Configuration Files

```toml
[claude]
# Enable interactive session management
use_interactive_sessions = true
```

### Via Environment Variables

```bash
export CLAUDE_CODE__CLAUDE__USE_INTERACTIVE_SESSIONS=true
```

## How It Works

1. When a request includes a `conversation_id`, the system checks if an interactive session already exists for that ID
2. If a session exists, the message is sent to the existing Claude process
3. If no session exists, a new interactive Claude process is started and associated with the conversation ID
4. Sessions are automatically cleaned up after 30 minutes of inactivity

## API Usage

Simply include a `conversation_id` in your requests to use the same session:

```bash
# First request - creates a new session
curl -X POST http://localhost:8000/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-3-5-sonnet-20241022",
    "conversation_id": "my-session-123",
    "messages": [{
      "role": "user",
      "content": "Hello! Remember the number 42."
    }]
  }'

# Second request - reuses the same session
curl -X POST http://localhost:8000/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-3-5-sonnet-20241022",
    "conversation_id": "my-session-123",
    "messages": [{
      "role": "user",
      "content": "What number did I ask you to remember?"
    }]
  }'
```

## Session Management

### Session Lifetime

- Sessions are kept alive for 30 minutes after the last activity
- Inactive sessions are automatically cleaned up to free resources
- You can manually close a session by making a DELETE request to `/v1/sessions/{conversation_id}`

### Session Limits

- Maximum concurrent sessions is controlled by `claude.max_concurrent_sessions` configuration
- Default: 20 concurrent sessions

### Monitoring

You can check active sessions through the stats endpoint:

```bash
curl http://localhost:8000/stats
```

## Performance Comparison

| Request Type | Without Interactive Sessions | With Interactive Sessions |
|--------------|------------------------------|---------------------------|
| First Request | 5-15 seconds | 5-15 seconds |
| Subsequent Requests | 5-15 seconds | 1-3 seconds |

## Best Practices

1. **Use Consistent Conversation IDs**: Always use the same conversation ID for related requests
2. **Generate Unique IDs**: Use UUIDs or similar to ensure conversation IDs don't collide
3. **Handle Session Expiry**: Be prepared to handle cases where a session has expired and needs recreation
4. **Close Long Sessions**: For very long conversations, consider closing and recreating sessions periodically

## Troubleshooting

### Session Not Reusing

If sessions aren't being reused:
1. Check that `use_interactive_sessions` is set to `true`
2. Verify you're using the same `conversation_id` across requests
3. Ensure the session hasn't expired (30 minute timeout)
4. Check logs for any errors creating or maintaining sessions

### High Memory Usage

If memory usage is high:
1. Reduce `max_concurrent_sessions`
2. Decrease session timeout in configuration
3. Manually close sessions when conversations end

## Testing

Use the provided test script to verify interactive sessions are working:

```bash
./script/test_interactive_session.sh
```

This script will:
- Create a session and ask Claude to remember information
- Make a second request to verify the session was reused
- Check that different conversation IDs create separate sessions