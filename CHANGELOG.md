# Changelog

All notable changes to this project will be documented in this file.

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
