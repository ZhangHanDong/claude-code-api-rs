# Release 0.2.0

This release aligns the Rust SDK (cc-sdk) with the Python Agent SDK control protocol and adds quality-of-life features for agent tooling and streaming.

What's new
- Control protocol
  - `Query::set_model(Some|None)` and `Query::set_permission_mode(..)`
  - Accepts snake_case and camelCase keys for `can_use_tool` and `hook_callback`
  - `Query::stream_input(..)` now calls `end_input()` when finished
- Transport & CLI
  - `Transport::end_input()` (default no-op); `SubprocessTransport` closes stdin
  - Passes `--include-partial-messages`, `--fork-session`, `--setting-sources`, `--agents` when configured
  - Exposes `CLAUDE_AGENT_SDK_VERSION` in the environment
- Docs: Agent Tools & MCP guidance in SDK and root READMEs

Compatibility
- Minor API surface change: trait `Transport` gains a default `end_input()` method. Existing implementers typically do not need code changes, but we bump minor to be explicit.

Upgrade guide
1. Bump dependency: `cc-sdk = "0.2.0"`
2. If you provide a custom `Transport`, optionally override `end_input()` for graceful shutdown.
3. To enable partial assistant chunks, set `ClaudeCodeOptions::builder().include_partial_messages(true)`.
4. To define agents and settings sources, use `agents(..)` and `setting_sources(..)` on the builder.

