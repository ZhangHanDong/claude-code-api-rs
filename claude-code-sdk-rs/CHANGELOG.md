# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.10] - 2025-01-15

### Added
- Comprehensive streaming documentation explaining message-level vs character-level streaming
- Clarification about Claude CLI limitations and output format support
- Documentation comparing Python and Rust SDK streaming capabilities

### Clarified
- Both Python and Rust Claude Code SDKs have identical streaming capabilities
- Streaming refers to message-level streaming (complete JSON messages), not character-level
- Claude CLI only supports `stream-json` format which outputs complete JSON objects
- True character-level streaming requires direct Anthropic API, not Claude CLI

### Technical Analysis
- Added test scripts demonstrating that `text` format doesn't support streaming
- Documented that all three Claude CLI output formats (`text`, `json`, `stream-json`) don't support character-level streaming
- Created comprehensive explanation document (`STREAMING_EXPLANATION.md`)

## [0.1.9] - 2025-01-15

### Fixed
- **Critical Environment Variable Fix**: Intelligent handling of `CLAUDE_CODE_MAX_OUTPUT_TOKENS`
  - Discovered maximum safe value is 32000 (values above cause Claude CLI to crash)
  - SDK now automatically caps values > 32000 to the maximum safe value
  - Invalid non-numeric values are replaced with safe default (8192)
  - Prevents "Error: Invalid env var" crashes that exit with code 1

### Added
- Documentation for environment variables (`docs/ENVIRONMENT_VARIABLES.md`)
- Better warning logs when invalid environment variable values are detected

### Improved
- More robust environment variable validation before spawning Claude CLI
- Smarter handling instead of simply removing problematic variables

## [0.1.8] - 2025-01-15

### Added
- **Streaming Output Support** - New methods for streaming responses
  - `receive_messages_stream()` - Returns a stream of messages for async iteration
  - `receive_response_stream()` - Convenience method that streams until Result message
  - Full parity with Python SDK's streaming capabilities
- Comprehensive streaming example (`examples/streaming_output.rs`)
- Process lifecycle management with Arc<Mutex> for shared ownership
- Automatic cleanup task that kills Claude CLI process when stream is dropped
- Better logging for process lifecycle events (debug, info, warn levels)

### Fixed
- Fixed async execution order in comprehensive test to avoid future not being awaited
- Improved process cleanup in `query_print_mode` to prevent zombie processes
- Added automatic process termination when stream is dropped to prevent resource leaks
- **Critical Fix**: Automatically handle problematic `CLAUDE_CODE_MAX_OUTPUT_TOKENS` environment variable
  - Maximum safe value is 32000 - values above this cause Claude CLI to exit with error code 1
  - SDK now automatically caps values > 32000 to 32000
  - Invalid non-numeric values are replaced with safe default (8192)
  - Prevents immediate process termination due to invalid settings

### Changed
- Refactored test execution to await futures immediately instead of collecting them
- Enhanced error handling with more detailed warning messages for process management

## [0.1.7] - 2025-01-14

### Added
- Support for `plan` permission mode to match Python SDK
- Support for `extra_args` field in `ClaudeCodeOptions` for passing arbitrary CLI flags
- New `ThinkingContent` content block type with `thinking` and `signature` fields
- New `CliJsonDecodeError` error type for better JSON decode error handling
- Builder methods: `extra_args()` and `add_extra_arg()` for flexible CLI argument passing
- Full parity with Python SDK version 0.0.20
- Documentation for 2025 Claude models (Opus 4.1, Sonnet 4)
- Example code for model selection and fallback strategies

### Changed
- `PermissionMode` enum now includes `Plan` variant
- Enhanced error handling with more specific error types
- Updated documentation to reflect 2025 model availability

### Model Support
- **Opus 4.1**: Use `"opus-4.1"` or `"claude-opus-4-1-20250805"`
- **Sonnet 4**: Use `"sonnet-4"` or `"claude-sonnet-4-20250514"`
- **Aliases**: `"opus"` and `"sonnet"` for latest versions

## [0.1.6] - 2025-01-23

### Added
- Support for `settings` field in `ClaudeCodeOptions` for `--settings` CLI parameter
- Support for `add_dirs` field in `ClaudeCodeOptions` for `--add-dir` CLI parameter
- New builder methods: `settings()`, `add_dirs()`, and `add_dir()`
- Full parity with Python SDK version 0.0.19

### Changed
- Updated subprocess transport to pass new CLI arguments

## [0.1.5] - 2025-01-22

### Added
- New `InteractiveClient` (renamed from `SimpleInteractiveClient`) with full Python SDK parity
- `send_message()` method - send messages without waiting for response
- `receive_response()` method - receive messages until Result message (matches Python SDK)
- `interrupt()` method - cancel ongoing operations
- Broadcast channel support in transport layer for multiple message receivers
- Comprehensive examples and documentation in English, Chinese, and Japanese
- Backward compatibility alias for `SimpleInteractiveClient`

### Fixed
- Critical deadlock issue in original `ClaudeSDKClient` where receiver task held transport lock
- Transport layer `receive_messages()` now creates new receivers instead of taking ownership
- Interactive mode hang issue completely resolved

### Changed
- Renamed `SimpleInteractiveClient` to `InteractiveClient` for clarity
- Updated README to reflect working interactive mode
- Transport layer now uses broadcast channels instead of mpsc channels
- Improved error messages and logging

## [0.1.4] - 2025-01-21

### Added
- Initial implementation of `SimpleInteractiveClient` to work around deadlock issues
- Support for `permission_prompt_tool_name` field for Python SDK compatibility
- Comprehensive test suite for interactive functionality

### Fixed
- Parameter mapping consistency with Python SDK
- Query function print mode parameters

### Changed
- Default client recommendation to use `SimpleInteractiveClient`

## [0.1.3] - 2025-01-20

### Added
- Basic interactive client implementation
- Streaming message support
- Full configuration options matching Python SDK

### Known Issues
- Interactive client has deadlock issues when sending messages

## [0.1.2] - 2025-01-19

### Added
- Support for all ClaudeCodeOptions fields
- Message parsing for all message types
- Error handling framework

## [0.1.1] - 2025-01-18

### Added
- Basic subprocess transport implementation
- Simple query function

## [0.1.0] - 2025-01-17

### Added
- Initial release
- Basic SDK structure
- Type definitions matching Python SDK