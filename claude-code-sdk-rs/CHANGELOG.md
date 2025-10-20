# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.0] - 2025-10-21

### 🎯 Major Release: Strongly-Typed Hooks & Production-Grade Quality

This release introduces a complete strongly-typed hooks system, eliminating all compiler warnings, and achieving production-grade code quality through modern Rust patterns.

### ⚠️ Breaking Changes

#### Strongly-Typed Hook System
- **`HookCallback` trait signature changed**:
  ```rust
  // Before (0.2.0)
  async fn execute(
      &self,
      input: &serde_json::Value,
      tool_use_id: Option<&str>,
      context: &HookContext,
  ) -> Result<serde_json::Value, SdkError>;

  // After (0.3.0)
  async fn execute(
      &self,
      input: &HookInput,
      tool_use_id: Option<&str>,
      context: &HookContext,
  ) -> Result<HookJSONOutput, SdkError>;
  ```

- **Hook Event Names**: Must use PascalCase format (e.g., `"PreToolUse"` not `"pre_tool_use"`)
  - CLI only recognizes PascalCase event names
  - See `docs/HOOK_EVENT_NAMES.md` for complete reference

### Added

#### Strongly-Typed Hook Input Types
- **`HookInput` enum** with discriminated variants:
  - `PreToolUse(PreToolUseHookInput)` - Before tool execution
  - `PostToolUse(PostToolUseHookInput)` - After tool execution
  - `UserPromptSubmit(UserPromptSubmitHookInput)` - User prompt submission
  - `Stop(StopHookInput)` - Session stop
  - `SubagentStop(SubagentStopHookInput)` - Subagent stop
  - `PreCompact(PreCompactHookInput)` - Before context compaction

#### Strongly-Typed Hook Output Types
- **`HookJSONOutput` enum**:
  - `Sync(SyncHookJSONOutput)` - Synchronous response
  - `Async(AsyncHookJSONOutput)` - Asynchronous response with callback ID

- **Output fields** with proper Rust naming:
  - `async_` → `"async"` (field name conversion)
  - `continue_` → `"continue"` (field name conversion)
  - Compile-time validation of all hook responses

#### Type Safety Benefits
- Compile-time verification of hook event types
- Automatic serialization/deserialization with proper error handling
- IDE autocomplete and type hints for all hook fields
- Eliminates runtime JSON parsing errors

### Changed

#### Code Quality Improvements (100% Warning Elimination)
- **Core Library**: 0 warnings (down from 26+)
- **Test Code**: 0 warnings (down from 8)
- **Example Code**: 0 warnings (down from ~25)
- **Total**: 0 warnings across entire codebase ✨

#### Modern Rust Patterns Adopted
- **Let Chains (RFC 2497)**: Flattened nested if-let statements
  ```rust
  // Before
  if let Some(tool_name) = data.get("tool_name") {
      if let Some(input) = data.get("input") {
          if let Some(callback) = callbacks.get(id) {
              // ...
          }
      }
  }

  // After
  if let Some(tool_name) = data.get("tool_name")
      && let Some(input) = data.get("input")
      && let Some(callback) = callbacks.get(id) {
      // ...
  }
  ```

- **Numeric Clamping**: Semantic `.clamp()` instead of `.min().max()`
  ```rust
  // Before: tokens.min(32000).max(1)
  // After: tokens.clamp(1, 32000)
  ```

- **Inline Format Arguments**: Modern format string syntax
  ```rust
  // Before: format!("Error: {}", error)
  // After: format!("Error: {error}")
  ```

- **Iterator Optimization**: O(1) `next_back()` instead of O(n) `last()`

### Improved

#### Documentation
- **New**: `docs/HOOK_EVENT_NAMES.md` - Hook event name reference with correct PascalCase formats
- **Enhanced**: All hook examples updated to use strongly-typed APIs
- **Added**: Comprehensive type documentation with field descriptions
- **Cleanup**: Removed 14 temporary process documents, organized all user-facing docs

#### Error Messages
- Hook input parsing errors now include detailed type information
- Better error messages for hook event name mismatches
- Clear distinction between format errors and logic errors

#### Examples
- **`examples/hooks_typed.rs`**: New comprehensive strongly-typed hooks example
  - ToolUseLogger: Logs all tool usage
  - ToolBlocker: Blocks specific tools with validation
  - PromptEnhancer: Modifies user prompts
- **`examples/control_protocol_demo.rs`**: Updated to use new hook types
- All examples verified with zero warnings

### Fixed

- **Hook Event Names**: Corrected examples to use PascalCase (`"PreToolUse"` not `"pre_tool_use"`)
- **Documentation Links**: Fixed all broken relative paths in documentation
- **Test Data Format**: Updated tests to include required `hook_event_name` discriminator field
- **Result Type**: Fixed confusion between `cc_sdk::Result<T>` and `std::result::Result<T, E>`

### Technical Details

#### Hook Type System Architecture
- Uses `#[serde(tag = "hook_event_name")]` for discriminated union
- Field name conversion with `#[serde(rename = "...")]`
- Comprehensive test coverage with 15+ unit tests
- End-to-end integration tests with mock transport

#### Code Quality Metrics
```
Compilation:  0 errors,  0 warnings
Clippy (lib): 0 warnings
Clippy (tests): 0 warnings
Test Suite:   45/45 passed
Coverage:     All hook types tested
```

### Migration Guide

#### Updating Hook Callbacks

1. **Change trait signature**:
   ```rust
   use cc_sdk::{HookCallback, HookInput, HookJSONOutput, SyncHookJSONOutput};

   #[async_trait]
   impl HookCallback for MyHook {
       async fn execute(
           &self,
           input: &HookInput,  // Changed from &serde_json::Value
           tool_use_id: Option<&str>,
           context: &HookContext,
       ) -> Result<HookJSONOutput, SdkError> {  // Changed return type
           match input {
               HookInput::PreToolUse(pre_tool_use) => {
                   // Access typed fields
                   println!("Tool: {}", pre_tool_use.tool_name);

                   Ok(HookJSONOutput::Sync(SyncHookJSONOutput {
                       continue_: Some(true),
                       ..Default::default()
                   }))
               }
               _ => Ok(HookJSONOutput::Sync(SyncHookJSONOutput::default()))
           }
       }
   }
   ```

2. **Update hook registration** to use PascalCase event names:
   ```rust
   // Before
   hooks.insert("pre_tool_use".to_string(), ...);

   // After
   hooks.insert("PreToolUse".to_string(), ...);
   ```

3. **Add new imports**:
   ```rust
   use cc_sdk::{
       HookInput, HookJSONOutput,
       SyncHookJSONOutput, AsyncHookJSONOutput,
       PreToolUseHookInput, PostToolUseHookInput,
       // ... other hook types as needed
   };
   ```

#### Benefits of Migration
- ✅ Compile-time type safety
- ✅ Better IDE support with autocomplete
- ✅ Eliminate runtime JSON parsing errors
- ✅ Clear documentation of available fields
- ✅ Easier testing with concrete types

### Documentation

- **Hook Event Names**: `docs/HOOK_EVENT_NAMES.md`
- **Typed Hooks Example**: `examples/hooks_typed.rs`
- **Control Protocol**: `examples/control_protocol_demo.rs`
- **API Documentation**: All public types fully documented

### Notes

- This is a **major version bump** due to breaking changes in `HookCallback` trait
- All existing hook implementations must be updated to use typed interfaces
- Python SDK parity maintained with strongly-typed implementation
- Zero warnings achieved across entire codebase
- Production-grade code quality standards met

## [0.2.0] - 2025-10-07

### Added
- `Query::set_model(Some|None)` control request
- `ClaudeCodeOptions::builder().include_partial_messages(..)`, `fork_session(..)`, `setting_sources(..)`, `agents(..)`
- `Transport::end_input()` (default no-op); `SubprocessTransport` closes stdin
- Message forwarding from `transport.receive_messages()` into `Query` stream

### Changed
- Control protocol parsing accepts snake_case and camelCase keys for `can_use_tool` and `hook_callback`
- Subprocess command includes `--include-partial-messages`, `--fork-session`, `--setting-sources`, `--agents` when configured
- Sets `CLAUDE_AGENT_SDK_VERSION` env to crate version

### Notes
- Trait extension is backwards compatible due to default method; bump minor version to reflect API surface change

## [0.1.11] - 2025-01-16

### 🎯 Major Feature: Control Protocol Format Compatibility

This release introduces configurable control protocol format support, ensuring compatibility with both current and future Claude CLI versions while maintaining full feature parity with the Python SDK.

### Added
- **Control Protocol Format Configuration** - New `ControlProtocolFormat` enum with three modes:
  - `Legacy` (default) - Uses `sdk_control_request/response` format for maximum compatibility
  - `Control` - Uses new unified `type=control` format for future CLI versions
  - `Auto` - Currently defaults to Legacy, will auto-negotiate in future releases
  
- **Flexible Configuration Methods**:
  - Programmatic: `ClaudeCodeOptions::builder().control_protocol_format(ControlProtocolFormat::Legacy)`
  - Environment variable: `CLAUDE_CODE_CONTROL_FORMAT=legacy|control` (overrides programmatic setting)
  
- **Full Control Protocol Implementation**:
  - Permission callbacks (`CanUseTool` trait)
  - Hook callbacks (`HookCallback` trait) 
  - MCP server support (SDK type)
  - Debug stderr output redirection
  - Initialization protocol with server info retrieval

- **Documentation**:
  - `CONTROL_PROTOCOL_COMPATIBILITY.md` - Comprehensive compatibility guide
  - `examples/control_format_demo.rs` - Interactive demonstration of format configuration
  - `examples/control_protocol_demo.rs` - Full control protocol features showcase

### Improved
- **Concurrency Optimizations**:
  - Control handler uses `take_sdk_control_receiver()` to avoid holding locks across await points
  - Client message reception uses `subscribe_messages()` for lock-free streaming
  - Broadcast channel for multi-subscriber message distribution
  
- **Dual-Stack Message Reception**:
  - Automatically handles new format: `{"type":"control","control":{...}}`
  - Maintains compatibility with legacy: `{"type":"sdk_control_request","request":{...}}`
  - Supports system control variants: `{"type":"system","subtype":"sdk_control:*"}`

- **Initialization Strategy**:
  - Non-blocking `initialize()` sends request without waiting
  - Server info cached in client, accessible via `get_server_info()`
  - Prevents Query/Client message competition

### Fixed
- **Lock Contention Issues**: Eliminated all instances of holding locks across await boundaries
- **Message Routing**: Control messages properly routed to SDK control channel without blocking main stream
- **Error Aggregation**: stderr output properly aggregated and forwarded as system messages

### Technical Details
- **Architecture**: Single shared `Arc<Mutex<SubprocessTransport>>` between Client and Query
- **Compatibility**: Default Legacy format ensures zero-breaking changes for existing users
- **Migration Path**: Smooth transition to new format when CLI support is available

### Migration Guide
1. **No action required** - Default behavior maintains full compatibility
2. **Testing new format**: Set `CLAUDE_CODE_CONTROL_FORMAT=control` (requires updated CLI)
3. **Future-proofing**: Use `ControlProtocolFormat::Auto` for automatic format negotiation

### Notes
- This release achieves feature parity with Python SDK while providing additional configuration flexibility
- All control protocol features (permissions, hooks, MCP) are fully functional with current CLI versions
- The dual-stack reception ensures forward compatibility with future CLI updates

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
