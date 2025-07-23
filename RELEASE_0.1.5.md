# Release v0.1.5 - Major Architecture Upgrade

## 🎉 Overview

This release marks a significant milestone in the Claude Code API project. We've restructured the entire codebase into a Cargo workspace and built the API server on top of a robust, feature-complete SDK.

## 🏗️ Major Changes

### Workspace Structure
The project is now organized as a Cargo workspace with two main crates:
- **`claude-code-api`** - The OpenAI-compatible REST API server
- **`claude-code-sdk-rs`** - A powerful Rust SDK for Claude Code CLI integration

### Built on claude-code-sdk-rs
The API server now leverages the SDK internally, providing:
- Better code organization and reusability
- Consistent Claude Code CLI interaction patterns
- Easier maintenance and feature additions

## ✨ New Features

### Performance Optimizations
- **Connection Pooling**: OptimizedClient with connection pool support for 5-10x faster responses
- **Pre-warming**: Connections are pre-warmed on server startup for reduced first-request latency
- **Multiple Client Modes**:
  - OneShot: Simple, stateless queries
  - Interactive: Maintains conversation context
  - Batch: Process multiple queries concurrently

### Enhanced OpenAI Compatibility
- **Proper Conversation History**: Chat completions now correctly handle full message history
- **Accurate Token Counting**: Token usage reflects the entire conversation context
- **Improved Streaming**: Better streaming response support

### New Testing Tools
- Manual interactive test tools for both SDK and OpenAI API modes
- OpenAI-compatible server example with proper history support
- Comprehensive testing scripts for all operation modes
- Performance benchmarking examples

## 🐛 Bug Fixes

- Fixed OpenAI chat completions to properly process conversation history (previously only used last message)
- Corrected token counting to include all messages in the conversation
- Improved error handling in connection pool management
- Fixed various edge cases in interactive mode

## 📚 Documentation

- Updated README to reflect the new architecture
- Added comprehensive guides for:
  - SDK usage and integration
  - Performance optimization
  - Interactive mode testing
  - OpenAI API compatibility

## 💻 For Developers

### Using the SDK Directly
```toml
[dependencies]
cc-sdk = "0.1.5"
```

```rust
use cc_sdk::{query, OptimizedClient, ClientMode};

// Simple query
let response = query("Hello Claude").await?;

// Advanced usage with connection pooling
let client = OptimizedClient::new(options, ClientMode::Interactive)?;
```

### API Server Improvements
The API server now uses `OptimizedClient` by default, providing better performance out of the box.

## 🙏 Acknowledgments

This release represents a major architectural improvement. Special thanks to all contributors and users who provided feedback on performance issues and API compatibility.

## 📋 Migration Guide

For most users, this is a drop-in replacement. However, if you're using advanced features:

1. **Interactive Sessions**: Now more stable with the new SDK architecture
2. **Performance**: Enable connection pooling in your configuration for best results
3. **OpenAI Compatibility**: Conversation history now works correctly - ensure your client sends full message arrays

## 🚀 What's Next

- Further performance optimizations
- Additional OpenAI API endpoints
- Enhanced MCP support
- Request caching implementation

---

For detailed changes, see the [CHANGELOG.md](CHANGELOG.md)