# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.4.0] - 2025-12-17

### 🎯 Major Release: Python SDK v0.1.14 Full Parity & Auto-Download

This release achieves **full feature parity** with the official Python `claude-agent-sdk` v0.1.14, including automatic CLI download capability.

### ✨ Highlights

- **100% Python SDK v0.1.14 Feature Parity** - All configuration options synchronized
- **Automatic CLI Download** - No manual installation required
- **Claude Sonnet 4.5 Support** - Latest model with enhanced capabilities
- **File Checkpointing** - Rewind file changes to any point in conversation
- **Structured Output** - JSON schema validation for responses

### 📊 Python SDK vs Rust SDK Feature Comparison

| Feature | Python SDK | Rust SDK | Status |
|---------|-----------|----------|--------|
| **Core Features** ||||
| Simple query API | `query()` | `query()` | ✅ Parity |
| Interactive client | `ClaudeSDKClient` | `ClaudeSDKClient` | ✅ Parity |
| Streaming messages | ✅ | ✅ | ✅ Parity |
| **Configuration** ||||
| `tools` (base tool set) | ✅ | ✅ | ✅ Parity |
| `allowed_tools` | ✅ | ✅ | ✅ Parity |
| `disallowed_tools` | ✅ | ✅ | ✅ Parity |
| `permission_mode` | ✅ | ✅ | ✅ Parity |
| `system_prompt` | ✅ | ✅ | ✅ Parity |
| `max_turns` | ✅ | ✅ | ✅ Parity |
| `max_thinking_tokens` | ✅ | ✅ | ✅ Parity |
| `max_output_tokens` | ✅ | ✅ | ✅ Parity |
| `max_budget_usd` | ✅ | ✅ | ✅ Parity |
| `model` | ✅ | ✅ | ✅ Parity |
| `fallback_model` | ✅ | ✅ | ✅ Parity |
| `betas` (SDK beta features) | ✅ | ✅ | ✅ Parity |
| `output_format` (structured) | ✅ | ✅ | ✅ Parity |
| `sandbox` | ✅ | ✅ | ✅ Parity |
| `plugins` | ✅ | ✅ | ✅ Parity |
| `setting_sources` | ✅ | ✅ | ✅ Parity |
| `agents` | ✅ | ✅ | ✅ Parity |
| **File Operations** ||||
| `enable_file_checkpointing` | ✅ | ✅ | ✅ Parity |
| `rewind_files()` | ✅ | ✅ | ✅ Parity |
| **MCP Support** ||||
| Stdio servers | ✅ | ✅ | ✅ Parity |
| SSE servers | ✅ | ✅ | ✅ Parity |
| HTTP servers | ✅ | ✅ | ✅ Parity |
| SDK (in-process) servers | ✅ | ✅ | ✅ Parity |
| **Control Protocol** ||||
| Permission callbacks | ✅ | ✅ | ✅ Parity |
| Hook callbacks | ✅ | ✅ | ✅ Parity |
| Interrupt | ✅ | ✅ | ✅ Parity |
| Set model | ✅ | ✅ | ✅ Parity |
| Set permission mode | ✅ | ✅ | ✅ Parity |
| **CLI Management** ||||
| Bundled CLI | ✅ (in wheel) | ✅ (auto-download) | ✅ Equivalent |
| `user` (OS setuid) | ✅ | ❌ Reserved | ⚠️ Not implemented |

### Added

#### New Configuration Options (Python SDK v0.1.14 Sync)

- **Tools Configuration** (`tools` option) - Control the base set of available tools
  - `ToolsConfig::list(vec!["Read", "Edit"])` - Enable specific tools only
  - `ToolsConfig::none()` - Disable all built-in tools
  - `ToolsConfig::claude_code_preset()` - Use claude_code preset
  - **Note**: Distinct from `allowed_tools` which only controls auto-approval permissions

- **File Checkpointing & Rewind** - Track and rewind file changes during sessions
  - `enable_file_checkpointing(true)` - Enable file state tracking
  - `ClaudeSDKClient::rewind_files(user_message_id)` - Rewind to any checkpoint
  - `SDKControlRewindFilesRequest` type for control protocol

- **SDK Beta Features** (`betas` option) - Enable Anthropic API beta features
  - `SdkBeta::Context1M` - Extended context window (1M tokens)
  - Builder: `.add_beta(SdkBeta::Context1M)`

- **Budget Control** (`max_budget_usd`) - Set spending limits for sessions
  - Session automatically terminates when budget exceeded
  - Example: `.max_budget_usd(5.0)` for $5 limit

- **Fallback Model** (`fallback_model`) - Automatic model failover
  - Used when primary model is unavailable
  - Example: `.fallback_model("claude-3-5-sonnet-20241022")`

- **Structured Output** (`output_format`) - JSON schema validation for responses
  - Returns validated `structured_output` in `Message::Result`
  - Supports `structuredOutput` alias for CLI compatibility
  - Example: `.output_format(json!({"type": "json_schema", "schema": {...}}))`

- **Sandbox Configuration** (`sandbox`) - Bash command isolation
  - `SandboxSettings` with full network/filesystem control
  - `SandboxNetworkConfig` for proxy and socket configuration
  - `SandboxIgnoreViolations` for exception handling
  - Automatically merged into `--settings` CLI argument

- **Plugin Support** (`plugins`) - Custom plugin configuration
  - `SdkPluginConfig::Local { path }` for local plugins
  - Passed to CLI as `--plugin-dir` arguments
  - Builder: `.add_plugin(SdkPluginConfig::Local { path: "...".into() })`

- **User Identifier** (`user`) - Reserved field for user tracking
  - **Note**: In Python SDK this maps to OS-level `subprocess.Popen(user=...)` which is not implemented in Rust SDK due to platform/privilege requirements
  - Field is reserved for future use

- **Max Thinking Tokens** - Now properly passed to CLI as `--max-thinking-tokens`
  - Only passed when value is non-zero to avoid breaking older CLI versions
  - Enables extended thinking budget for complex reasoning tasks

#### Automatic CLI Download

- **Auto-download Claude Code CLI** when not found
  - New `auto_download_cli` option in `ClaudeCodeOptions`
  - Downloads via npm or official install script as fallback
  - Caches in platform-specific location:
    - macOS: `~/Library/Caches/cc-sdk/cli/`
    - Linux: `~/.cache/cc-sdk/cli/`
    - Windows: `%LOCALAPPDATA%\cc-sdk\cli\`
  - Enabled by default via `auto-download` feature flag
  - Can be disabled with `default-features = false`
  - New `SubprocessTransport::new_async()` method for async CLI resolution

- **CLI Discovery Priority**:
  1. System PATH (`claude`, `claude-code`)
  2. SDK cache directory (auto-downloaded)
  3. Common installation locations (npm, yarn, homebrew, etc.)
  4. Auto-download if `auto_download_cli` is enabled

#### Claude Sonnet 4.5 Support

- Added support for the latest Claude Sonnet 4.5 model (`claude-sonnet-4-5-20250929`)
- Updated `ModelRecommendation::balanced_model()` to return Sonnet 4.5
- Added `ModelRecommendation::latest_sonnet()` helper function
- Updated default recommendations to use Sonnet 4.5 for balanced/general/normal/standard tasks
- New example: `examples/sonnet_4_5_example.rs`

#### Account Information Retrieval

- New `get_account_info()` method for ClaudeSDKClient
- Multiple fallback methods: env var → config file → /status command
- New documentation: `docs/ACCOUNT_INFO.md`

### Changed

#### CLI Parameter Alignment (Python SDK Parity)

- `--tools` now uses comma-separated format or empty string (not JSON array)
- `--json-schema` used for structured output (extracted from `output_format.schema`)
- `--sandbox` settings merged into `--settings` JSON object
- `--plugin-dir` used for plugins (not `--plugins` JSON)
- `--setting-sources` always passed (even when empty)
- `--max-thinking-tokens` passed when > 0

#### Control Protocol Improvements

- `send_control_request` now returns `Err` when response has `subtype=error`
- Returns only `response` (or legacy `data`) payload, not full wrapper
- Added snake_case ↔ camelCase field aliases for all control types:
  - `tool_name` / `toolName`
  - `callback_id` / `callbackId`
  - `tool_use_id` / `toolUseId`
  - `user_message_id` / `userMessageId`
  - `server_name` / `mcpServerName` / `mcp_server_name`
  - `permission_suggestions` / `permissionSuggestions`
  - `blocked_path` / `blockedPath`

#### Message Parsing

- `structured_output` in `Message::Result` now supports `structuredOutput` alias
- Null values properly handled (ignored, not set to `Some(null)`)

### New Types

```rust
// Tools configuration
pub enum ToolsConfig { List(Vec<String>), Preset(ToolsPreset) }
pub struct ToolsPreset { pub preset_type: String, pub preset: String }

// SDK Beta features
pub enum SdkBeta { Context1M }

// Sandbox configuration
pub struct SandboxSettings { /* full network/filesystem controls */ }
pub struct SandboxNetworkConfig { /* proxy, socket settings */ }
pub struct SandboxIgnoreViolations { /* exception paths */ }

// Plugin configuration
pub enum SdkPluginConfig { Local { path: String } }

// Control protocol
pub struct SDKControlRewindFilesRequest { pub user_message_id: String }
```

### New Builder Methods

```rust
ClaudeCodeOptions::builder()
    .tools(ToolsConfig::list(vec!["Read", "Edit"]))
    .betas(vec![SdkBeta::Context1M])
    .max_budget_usd(5.0)
    .fallback_model("claude-3-5-sonnet-20241022")
    .output_format(json!({"type": "json_schema", "schema": {...}}))
    .enable_file_checkpointing(true)
    .sandbox(SandboxSettings { ... })
    .plugins(vec![SdkPluginConfig::Local { path: "...".into() }])
    .user("user-id")
    .auto_download_cli(true)
    .build()
```

### Examples

- `examples/test_auto_download.rs` - CLI auto-download testing
- `examples/sonnet_4_5_example.rs` - Sonnet 4.5 features
- `examples/account_info.rs` - Account information retrieval
- `examples/session_with_account_info.rs` - Session with account verification
- `examples/with_dotenv.rs` - Environment variable configuration

### Documentation

- `docs/ACCOUNT_INFO.md` - Account information guide
- `docs/ENVIRONMENT_VARIABLES.md` - Environment variables reference
- `docs/models-guide.md` - Updated with Sonnet 4.5
- `docs/SONNET_4_5_GUIDE.md` - Sonnet 4.5 specific guide

### Migration from 0.3.0

1. **No breaking changes** - All existing code continues to work
2. **New features are opt-in** - Enable via builder methods
3. **CLI auto-download** - Enabled by default, disable with `default-features = false`

### Notes

- Achieves **100% feature parity** with Python claude-agent-sdk v0.1.14
- Only `user` (OS setuid) is not implemented due to platform/privilege requirements
- All tests pass: `cargo test -p cc-sdk`
- Zero compiler warnings

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
- **Result Type**: Fixed confusion between `nexus_claude::Result<T>` and `std::result::Result<T, E>`

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
   use nexus_claude::{HookCallback, HookInput, HookJSONOutput, SyncHookJSONOutput};

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
   use nexus_claude::{
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
