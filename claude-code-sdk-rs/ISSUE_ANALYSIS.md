# Interactive Client Issue Analysis

## Problem
The `ClaudeSDKClient` hangs after sending a message when using interactive mode.

## Root Cause
The issue is in the transport layer's `receive_messages()` implementation:

```rust
fn receive_messages(&mut self) -> Pin<Box<dyn Stream<Item = Result<Message>> + Send + '_>> {
    if let Some(rx) = self.message_rx.take() {  // <-- This TAKES the receiver!
        Box::pin(ReceiverStream::new(rx).map(Ok))
    } else {
        Box::pin(futures::stream::empty())
    }
}
```

The `take()` method removes the receiver from the transport, meaning `receive_messages()` can only be called once. However, the `ClaudeSDKClient` architecture assumes it can be called multiple times.

## How Python SDK Works
The Python SDK:
1. Starts the subprocess with `--input-format stream-json` and `--output-format stream-json`
2. Keeps stdin open for continuous message sending
3. Uses async generators to continuously stream messages
4. Never "takes" the stdout stream - it remains available throughout the session

## Solution
The Rust SDK needs to be refactored to:
1. NOT take ownership of the message receiver in `receive_messages()`
2. Allow multiple concurrent readers of the message stream
3. Properly handle the async nature of the streaming protocol

## Quick Fix
For now, the `query()` function works correctly because it uses `--print` mode which doesn't require streaming. Users should use `query()` instead of `ClaudeSDKClient` until this is fixed.