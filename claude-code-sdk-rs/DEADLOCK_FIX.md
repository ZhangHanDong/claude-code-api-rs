# Deadlock Issue in ClaudeSDKClient

## Problem
The `start_message_receiver` method spawns a task that:
1. Locks the transport mutex
2. Calls `receive_messages()` which returns a stream
3. Holds the lock while waiting for messages from the stream

Meanwhile, `send_user_message` needs to lock the same transport to send messages, causing a deadlock.

## Solution
The transport needs to be redesigned to allow concurrent sending and receiving without holding locks for extended periods.

## Current Workaround
For now, users should use the `query()` function which works correctly because it uses a different transport instance for each query.