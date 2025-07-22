# Release Notes - v0.1.5

## 🎉 Major Release: Full Interactive Mode with Python SDK Parity!

This release fixes the critical deadlock issue in the interactive client and achieves near-complete feature parity with the Python SDK.

## ✨ New Features

### InteractiveClient (formerly SimpleInteractiveClient)
- **Fully working interactive mode** - No more hangs or deadlocks!
- **`send_message()`** - Send messages without waiting for response
- **`receive_response()`** - Receive messages until Result message (Python SDK parity)
- **`interrupt()`** - Cancel ongoing operations
- **Stateful conversations** - Maintains context across multiple exchanges

### Transport Layer Improvements
- Broadcast channel support for multiple message receivers
- Fixed `receive_messages()` to create new receivers instead of taking ownership
- Improved error handling and logging

## 🐛 Bug Fixes
- **Fixed critical deadlock** in original `ClaudeSDKClient`
- **Resolved hang issue** where interactive mode would freeze after sending messages
- **Fixed transport layer** single-use receiver problem

## 📚 Documentation
- Comprehensive README in English, Chinese, and Japanese
- Full API documentation with examples
- Migration guide from SimpleInteractiveClient to InteractiveClient
- Detailed comparison with Python SDK

## 💔 Breaking Changes
- None! Full backward compatibility maintained
- `SimpleInteractiveClient` is now an alias for `InteractiveClient`

## 🔄 Migration
If you were using `SimpleInteractiveClient`:
```rust
// Old
use cc_sdk::SimpleInteractiveClient;

// New (both work)
use cc_sdk::InteractiveClient;
// or keep using SimpleInteractiveClient (it's an alias)
```

## 📦 Installation
```toml
[dependencies]
claude-code-sdk = "0.1.5"
```

## 🙏 Acknowledgments
Thanks to all users who reported the interactive mode issues and provided feedback!

## 📈 What's Next
- Session ID support for managing multiple conversations
- Streaming input support for advanced use cases
- Additional error types for fine-grained error handling

---

**Full Changelog**: https://github.com/your-username/claude-code-sdk-rs/blob/main/CHANGELOG.md
