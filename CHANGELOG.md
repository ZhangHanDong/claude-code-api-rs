# Changelog

All notable changes to this project will be documented in this file.

## [0.6.0] - 2026-02-25

### Added

#### WebSocket Bridge (Plan A)

- **WebSocket session management** — full lifecycle for CLI↔client bridging
  - `POST /v1/sessions` — create a session, spawns CLI with `--sdk-url ws://...`
  - `GET /v1/sessions` — list all active sessions
  - `GET /v1/sessions/{id}` — get session details
  - `DELETE /v1/sessions/{id}` — kill CLI process and clean up
- **WebSocket endpoints** for real-time bidirectional communication:
  - `/ws/cli/{session_id}` — CLI process connects here (NDJSON over WebSocket)
  - `/ws/session/{session_id}` — external clients connect here
- **WsBridge** — core message router between CLI and clients
  - Routes `system/init`, `assistant`, `result`, `stream_event`, `control_request` from CLI to clients
  - Routes `user_message`, `permission_response`, `interrupt`, `set_model`, `set_permission_mode` from clients to CLI
  - Message history replay on client reconnection
  - Pending permission tracking and forwarding
  - Message queuing when CLI is not yet connected
- **WsCliLauncher** — spawns Claude Code CLI with correct flags:
  - `--sdk-url ws://127.0.0.1:{port}/ws/cli/{session_id}`
  - `--print --output-format stream-json --input-format stream-json --verbose`
  - Process lifecycle monitoring with stdout/stderr logging
  - Cross-platform process termination (SIGTERM on Unix, taskkill on Windows)
- **NDJSON helpers** — `parse_ndjson()` and `to_ndjson()` with 5 unit tests
- Added `"ws"` feature to axum dependency for WebSocket upgrade support

#### SDK WebSocket Transport (Plan B)

- **`websocket` feature flag** in `cc-sdk` for WebSocket transport
- **`WebSocketTransport`** — implements `Transport` trait over WebSocket
- **`WebSocketConfig`** — reconnection, ping, buffer, and auth settings
- **`SdkError::WebSocketError`** — new feature-gated error variant
- 6 new unit tests for WebSocket transport
- See `claude-code-sdk-rs/CHANGELOG.md` for full SDK details

### Notes
- All 79 SDK tests pass (73 existing + 6 new WebSocket)
- All 23 API tests pass (18 existing + 5 new NDJSON)
- Zero regressions from existing functionality

## [0.1.10] - 2025-01-15

### SDK Updates
- **claude-code-sdk-rs v0.1.10**: Streaming documentation and clarifications
  - Added comprehensive documentation explaining message-level vs character-level streaming
  - Clarified that both Python and Rust SDKs have identical capabilities
  - Documented Claude CLI limitations with output formats
  - Created technical analysis and test scripts

### API Updates
- **Model Support**: Confirmed support for all 2025 Claude models
  - Claude 4 Series: Opus 4.1, Opus 4, Sonnet 4
  - Claude 3.7 Series: Sonnet 3.7
  - Claude 3.5 Series: Haiku 3.5
  - Claude 3 Series: Haiku 3
- **Streaming Enhancement**: Added text chunking handler (simulation for better UX)

### Documentation
- Added `STREAMING_EXPLANATION.md` with detailed technical explanation
- Created test scripts for validating streaming behavior
- Updated SDK documentation with streaming clarifications

### Changed
- Bumped workspace version to 0.1.10

---

## [0.1.9] - 2025-01-15

### SDK Updates
- **claude-code-sdk-rs v0.1.9**: Critical environment variable fixes
  - Intelligent handling of `CLAUDE_CODE_MAX_OUTPUT_TOKENS` (max safe value: 32000)
  - Prevents Claude CLI crashes due to invalid environment variable values
  - Added comprehensive environment variable documentation

### Changed
- Bumped workspace version to 0.1.9

---

## [0.1.8] - 2025-01-15

### SDK Updates
- **claude-code-sdk-rs v0.1.8**: Streaming output support and process improvements
  - **NEW**: Full streaming output support matching Python SDK capabilities
  - Added `receive_messages_stream()` and `receive_response_stream()` methods
  - Comprehensive streaming example demonstrating usage patterns
  - Fixed async execution order in comprehensive test suite
  - Improved process cleanup to prevent zombie processes
  - Added automatic process termination when stream is dropped
  - Enhanced process lifecycle management with Arc<Mutex>
  - Better logging for debugging process lifecycle events

### API Updates
- **Model List Update**: Complete update based on official Anthropic API documentation
  - **Claude 4 Series (2025)**:
    - Claude Opus 4.1 (claude-opus-4-1-20250805)
    - Claude Opus 4 (claude-opus-4-20250514)
    - Claude Sonnet 4 (claude-sonnet-4-20250514)
  - **Claude 3.7 Series (2025)**:
    - Claude Sonnet 3.7 (claude-3-7-sonnet-20250219)
    - Claude Sonnet 3.7 Latest (claude-3-7-sonnet-latest)
  - **Claude 3.5 Series (2024)**:
    - Claude Haiku 3.5 (claude-3-5-haiku-20241022)
    - Claude Haiku 3.5 Latest (claude-3-5-haiku-latest)
  - **Claude 3 Series (2024)**:
    - Claude Haiku 3 (claude-3-haiku-20240307)
  - Default model changed to claude-sonnet-4-20250514
  - Total of 8 models available via /v1/models endpoint

### Changed
- Bumped workspace version to 0.1.8
- Updated test suite to reflect new model count (8 models total)

### Improved
- Enhanced Claude CLI detection to check both 'claude' and 'claude-code' commands
- Added more search paths including macOS Homebrew locations
- Improved error diagnostics with detailed stderr capture and analysis
- Better error messages for common issues:
  - Claude CLI not installed
  - Authentication failures  
  - Model availability issues
  - Node.js/npm not installed

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
## 0.2.0 - 2025-10-07

Highlights
- Transport trait extended with default `end_input()`; `SubprocessTransport` closes stdin to signal end-of-input.
- Control protocol parity with Python Agent SDK:
  - `Query::set_model(Some|None)` and `Query::set_permission_mode(..)`
  - snake_case/camelCase field compatibility for `can_use_tool` and `hook_callback`
  - Query startup forwards non-control SDK messages from transport
- CLI argument passthrough:
  - `include_partial_messages` → `--include-partial-messages`
  - `fork_session` → `--fork-session`
  - `setting_sources` → `--setting-sources user,project,local`
  - `agents` → `--agents <json>`
- Diagnostics: sets `CLAUDE_AGENT_SDK_VERSION` env with crate version.

Notes
- Minor version bump due to trait extension; `end_input()` has a default no-op impl.
- Consider switching to Auto/Control protocol formats once the CLI stabilizes.
