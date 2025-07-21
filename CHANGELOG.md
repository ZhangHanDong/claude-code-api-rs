# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2024-07-21

### Added
- Initial release of Claude Code API
- OpenAI-compatible API endpoints (`/v1/chat/completions`, `/v1/models`)
- Conversation management with session persistence
- Multimodal support for processing images with text
- Response caching system with LRU eviction
- MCP (Model Context Protocol) integration
- File access control with configurable permissions
- Streaming response support via Server-Sent Events
- Comprehensive error handling with automatic retries
- Statistics API for monitoring usage
- Health check endpoint
- Request ID tracking for all requests
- CORS support for web applications
- Environment variable and file-based configuration
- Startup scripts for common scenarios
- Comprehensive documentation in English and Chinese

### Technical Details
- Built with Rust, Axum web framework, and Tokio async runtime
- Process reuse for improved performance
- In-memory conversation storage with automatic cleanup
- SHA256-based response caching
- Temporary file management for image processing
- Compatible with OpenAI client libraries

### Documentation
- Complete README in English and Chinese
- Detailed guides for multimodal usage, file access, and MCP
- API endpoint documentation
- Configuration examples
- Troubleshooting guide

[0.1.0]: https://github.com/yourusername/claude-code-api/releases/tag/v0.1.0