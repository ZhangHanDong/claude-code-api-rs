# Release Notes - v0.1.10

## Release Date: 2025-01-15

## Summary
This release focuses on documentation and clarification of streaming capabilities in the Claude Code SDK and API. We've added comprehensive documentation explaining the differences between message-level and character-level streaming, and clarified the limitations imposed by Claude CLI.

## Key Highlights

### 📚 Documentation Improvements
- **Streaming Clarification**: Added detailed documentation explaining that both Python and Rust Claude Code SDKs support message-level streaming (not character-level)
- **Technical Analysis**: Created comprehensive technical explanation of Claude CLI's output format limitations
- **Test Scripts**: Added validation scripts demonstrating streaming behavior

### 🔧 Technical Details

#### Streaming Capabilities Explained
- Both Python and Rust SDKs use `--output-format stream-json`
- This provides **message-level streaming** (complete JSON messages)
- **Not character-level streaming** due to Claude CLI architectural constraints
- For true character streaming, direct Anthropic API is required (not Claude CLI)

#### Output Format Support
| Format | Description | Streaming | SDK Support |
|--------|-------------|-----------|-------------|
| `text` | Plain text output | No | ❌ |
| `json` | Complete JSON | No | ❌ |
| `stream-json` | Message streaming | Yes (messages) | ✅ |

### 🤖 Model Support
Confirmed support for all 2025 Claude models:
- Claude 4 Series: Opus 4.1, Opus 4, Sonnet 4
- Claude 3.7 Series: Sonnet 3.7
- Claude 3.5 Series: Haiku 3.5
- Claude 3 Series: Haiku 3

### 🛠 API Enhancements
- Added text chunking handler for simulated character streaming (UX improvement)
- Enhanced streaming handler in `claude-code-api`

## Breaking Changes
None - This is a documentation and clarification release

## Migration Guide
No migration needed. The SDK functionality remains the same, with better documentation.

## Installation

### Rust SDK
```toml
[dependencies]
cc-sdk = "0.1.10"
```

### API Server
```bash
cargo install claude-code-api --version 0.1.10
```

## What's Next
- Investigating direct Anthropic API integration for true character-level streaming
- Exploring additional output format support options
- Performance optimizations for message processing

## Contributors
- Environment variable fixes and streaming analysis
- Model list updates based on official Anthropic documentation
- Community feedback on streaming behavior

## Resources
- [Streaming Explanation](./STREAMING_EXPLANATION.md)
- [SDK Documentation](./claude-code-sdk-rs/README.md)
- [API Documentation](./README.md)

---

For questions or issues, please open an issue on [GitHub](https://github.com/ZhangHanDong/claude-code-api-rs).