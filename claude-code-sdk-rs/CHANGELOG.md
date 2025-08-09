# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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