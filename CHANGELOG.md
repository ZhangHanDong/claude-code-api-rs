# Changelog

All notable changes to this project will be documented in this file.

## [0.1.5] - 2025-01-22

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
