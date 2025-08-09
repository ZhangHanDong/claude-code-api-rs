# Changelog

All notable changes to this project will be documented in this file.

## [0.1.6] - 2025-01-23

### Added
- **SDK Updates**: Updated claude-code-sdk-rs to version 0.1.6 with:
  - Support for `settings` field in `ClaudeCodeOptions` for `--settings` CLI parameter
  - Support for `add_dirs` field in `ClaudeCodeOptions` for `--add-dir` CLI parameter
  - New builder methods: `settings()`, `add_dirs()`, and `add_dir()`
  - Full parity with Python SDK version 0.0.19

### Changed
- Bumped workspace version to 0.1.6

## [0.1.5] - 2025-01-23

### Integrations
- **url-preview v0.6.0** - First external project to integrate claude-code-api for LLM-powered web content extraction

### Major Changes
- **Restructured as Workspace**: Reorganized project into a Cargo workspace with two crates:
  - `claude-code-api` - The OpenAI-compatible REST API server
  - `claude-code-sdk-rs` - The underlying Rust SDK for Claude Code CLI integration
- **Built on claude-code-sdk-rs**: The API server now uses the robust SDK internally for all Claude interactions

### Added
- **Performance Optimizations via SDK**:
  - Connection pooling with `OptimizedClient` for 5-10x faster responses
  - Multiple client modes: OneShot, Interactive, and Batch processing
  - Pre-warming of connection pools for reduced first-request latency
  - Batch processing endpoints for high-throughput scenarios
- **Enhanced OpenAI Compatibility**:
  - Better conversation history handling in chat completions
  - Proper token counting in responses
  - Streaming response improvements
- **New Examples and Testing Tools**:
  - Manual interactive test tools for both SDK and OpenAI API modes
  - OpenAI-compatible server with proper history support
  - Comprehensive testing scripts for all modes
  - Performance benchmarking examples
- **Tool Calling Support**:
  - Full OpenAI tools format compatibility (removed deprecated functions format)
  - Automatic detection and formatting of tool calls in Claude's responses
  - Returns proper `tool_calls` array format
  - Seamless integration with modern AI tools like url-preview
- **Extended Timeout Support**:
  - Configurable timeout via CLAUDE_CODE__CLAUDE__TIMEOUT_SECONDS
  - Default timeout increased to 600 seconds for long-running tasks
  - Proper session cleanup on timeout to prevent EPIPE errors

### Changed
- **Architecture**: API server now leverages `cc-sdk` for all Claude operations
- **Performance**: Default configuration now uses optimized clients with connection pooling
- **Documentation**: Updated to reflect the workspace structure and SDK integration

### Fixed
- OpenAI chat completions now properly handle full conversation history
- Token counting now accurately reflects the entire conversation context
- Improved error handling and recovery in connection pool management
- Fixed timeout errors for long-running tasks (WebSearch, etc.)
- Fixed EPIPE errors by properly closing sessions on timeout
- Resolved all compilation warnings

## [0.1.5] - 2025-01-22 (SDK Release)

### Fixed
- Fixed interactive mode hang issue in Rust SDK with new SimpleInteractiveClient implementation
- Resolved deadlock in ClaudeSDKClient where receiver task held transport lock preventing sends
- Fixed transport layer to support multiple message receivers using broadcast channels

### Added
- New SimpleInteractiveClient that works correctly for stateful conversations
- Broadcast channel implementation in transport layer for multiple receivers
- Working examples and tests for interactive mode

### Changed
- Updated README to reflect working interactive mode with SimpleInteractiveClient
- Modified receive_messages() to create new receivers instead of taking ownership

## [0.1.4] - 2025-01-22

### Fixed
- Fixed critical issue where default `use_interactive_sessions` was set to true, causing timeouts
- Fixed interactive session mode issues with Claude CLI
- Resolved compilation errors with Handler trait bounds
- Fixed timeout issues with process pool mode
- Improved error handling for Claude process communication

### Changed
- **BREAKING**: Changed default `use_interactive_sessions` from true to false
- Temporarily disabled interactive session mode due to Claude CLI limitations
- Using process pool mode by default for better stability
- Improved logging for debugging process communication

### Added
- Documentation for interactive session concurrency issues
- Better error messages for timeout scenarios

## [0.1.3] - 2025-01-21

### Fixed
- Fixed default log level to ensure consistent logging output for both command aliases
- Changed default log filter from module-specific to global "info" level

## [0.1.2] - 2025-01-21

### Added
- Added `ccapi` as a shorter command alias for `claude-code-api`
- Both `claude-code-api` and `ccapi` commands are now available after installation

## [0.1.1] - 2025-01-21

### Fixed
- Fixed critical issue where Claude CLI was interpreting prompts as file paths
- Changed stdin handling from `Stdio::null()` to `Stdio::piped()` to properly send input to Claude
- Messages are now correctly sent via stdin instead of command line arguments

### Changed
- Improved error handling for stdin write operations

## [0.1.0] - 2025-01-21

### Initial Release
- High-performance OpenAI-compatible API gateway for Claude Code CLI
- Process pooling for improved efficiency
- Response caching with SHA256
- Multimodal support (text and images)
- MCP (Model Context Protocol) support
- Streaming responses via SSE
- Session management
- File access control
- Comprehensive error handling with retries
