# Response to: Python SDK Streaming Support

Thank you for bringing up this question about streaming support. After investigating both the Python and Rust Claude Code SDKs, I can clarify the situation:

## TL;DR
Both Python and Rust Claude Code SDKs have **identical streaming capabilities**. They both support **message-level streaming**, not character-level streaming.

## What "Streaming" Means in Claude Code SDKs

When we say the SDKs support "streaming", we mean:
- ✅ **Message streaming**: Receiving complete messages as they're generated
- ❌ **Character streaming**: NOT receiving character-by-character output

## Technical Details

### Both SDKs Use the Same Approach
Looking at the source code:

**Python SDK** (`subprocess_cli.py:90`):
```python
cmd = [self._cli_path, "--output-format", "stream-json", "--verbose"]
```

**Rust SDK** (`subprocess.rs:106`):
```rust
cmd.arg("--output-format").arg("stream-json");
```

Both SDKs:
1. Use Claude CLI with `--output-format stream-json`
2. Receive complete JSON messages from the CLI
3. Parse these messages into structured objects
4. Cannot provide character-level streaming because Claude CLI doesn't support it

### Why Claude CLI Doesn't Support Character-Level Streaming

The Claude CLI offers three output formats:
- `text`: Complete response after processing (non-streaming)
- `json`: Complete JSON after processing (non-streaming)  
- `stream-json`: **Streams complete JSON messages** (message-level streaming)

The `stream-json` format streams **complete, valid JSON objects** because:
- JSON must be valid and complete to be parsed
- Tool use requires structured message format
- Error handling is more reliable with complete messages

### The Anthropic Python SDK is Different

The documentation you referenced about Python streaming refers to the **Anthropic Python SDK**, which is a different product that:
- Directly calls the Anthropic API
- Bypasses Claude CLI entirely
- Can support true character-level streaming via Server-Sent Events (SSE)

The Claude Code SDKs (both Python and Rust) work through Claude CLI, which has different architectural constraints.

## Current Streaming Capabilities

Both Claude Code SDKs provide:
- ✅ Real-time message delivery as they're generated
- ✅ Streaming multiple assistant responses in a conversation
- ✅ Lower latency compared to waiting for complete responses
- ❌ Character-by-character streaming (limited by Claude CLI)

## Example of Current Streaming Behavior

```python
# Python SDK
async for message in client.receive_messages():
    # Receives complete messages, not individual characters
    if isinstance(message, AssistantMessage):
        print(message.content)  # Prints complete message at once
```

```rust
// Rust SDK  
while let Some(message) = client.receive_messages_stream().next().await {
    // Receives complete messages, not individual characters
    if let Message::Assistant(msg) = message? {
        println!("{}", msg.content);  // Prints complete message at once
    }
}
```

## Conclusion

Both Python and Rust Claude Code SDKs have the same streaming capabilities. The limitation is not in the SDKs but in Claude CLI itself. If you need true character-level streaming, you would need to use the Anthropic API directly rather than through Claude CLI.

The API server (`claude-code-api`) does attempt to simulate character-level streaming by chunking messages, but this is just a workaround - the underlying data still arrives as complete messages from Claude CLI.

I hope this clarifies the situation! Both SDKs are working as designed and have feature parity.