# Repository Guidelines

## Project Structure & Module Organization
- Workspace crates: `claude-code-api/` (server) and `claude-code-sdk-rs/` (SDK).
- API crate modules: `src/api/`, `src/core/`, `src/models/`, `src/utils/`, `src/middleware/`.
- Shared assets/docs: `docs/`, `doc/`, `examples/`, `migrations/`, `config/`, `script/`.
- Integration tests live in `tests/`; unit tests live alongside modules in `src/...`.

## Architecture Overview
- Server: Axum + Tokio; routes in `src/api/*` call services in `src/core/*`.
- Process pool: `core/process_pool.rs` manages Claude CLI processes; interactive mode in `core/interactive_session.rs`.
- SDK: `claude-code-sdk-rs` provides CLI orchestration and streaming; API composes it.
- Middleware: request ID and error mapping in `src/middleware/*`; caching and conversations under `core/*`.

## Build, Test, and Development Commands
- Build all: `cargo build` (use `--release` for production).
- Run API server: `RUST_LOG=info cargo run -p claude-code-api --bin claude-code-api` (alias: `--bin ccapi`).
- Quick start script: `script/run.sh` (builds release and runs).
- Test all crates: `cargo test` (or per crate: `cargo test -p claude-code-api`).
- Lint/format: `cargo fmt --all` and `cargo clippy --all-targets --all-features -D warnings`.
- Docker: `docker build -t claude-code-api . && docker run --rm -p 8080:8080 --env-file .env claude-code-api`.

## Coding Style & Naming Conventions
- Rust edition: 2024; follow `rustfmt` defaults (4‑space indent, max width default).
- Naming: modules/files `snake_case`, types `PascalCase`, functions/vars `snake_case`, constants `SCREAMING_SNAKE_CASE`.
- Errors: prefer `anyhow::Result` at app edges; library code defines errors with `thiserror`.
- Logging: use `tracing` with `RUST_LOG` (e.g., `RUST_LOG=info`).

## Testing Guidelines
- Frameworks: `tokio::test` for async, `axum-test` for HTTP routes.
- Locations: unit tests in-module (`mod tests`), integration tests in `tests/` (e.g., `tests/api_tests.rs`).
- Conventions: name tests by behavior (e.g., `test_chat_completion_validation`). Ensure fast, deterministic tests.
- Run: `cargo test` or `cargo test -p claude-code-api`.

## Commit & Pull Request Guidelines
- Commits: concise, imperative subject (e.g., "Add models listing"), body explains motivation and approach; reference issues (`Closes #123`).
- PRs: include summary, linked issues, test plan (commands/curl), config/env changes, and screenshots/logs if touching HTTP endpoints.
- Add or update tests for user-visible changes.

## Security & Configuration Tips
- Do not commit secrets. Use `.env` (see `.env.example`) and `config/local.toml` for local overrides.
- Common env prefix: `CLAUDE_CODE__...` (see README). Validate setup with `script/check_config.sh`.
- When changing defaults, document corresponding keys in `README.md` and `config/`.

## Agent-Specific Instructions
- Keep patches minimal and scoped; avoid unrelated refactors or drive-by fixes.
- Follow this AGENTS.md for style, structure, and command examples.
- Reference files with explicit paths (e.g., `claude-code-api/src/main.rs:1`).
- Prefer `rg` for search, `cargo` for build/test, and existing scripts in `script/`.

## Agent Tools & MCP
- Enable tools in SDK: set `ClaudeCodeOptions.allowed_tools`/`disallowed_tools`, choose `permission_mode` (e.g., `AcceptEdits`).
- Runtime approvals: implement `CanUseTool` and return `PermissionResult::{Allow,Deny}`; SDK会按控制协议回传 `{allow, input?/reason?}`。
- MCP servers: configure via SDK `options.mcp_servers` (stdio/http/sse/sdk) or API `config/` + `script/start_with_mcp.sh`.
- Programmatic agents: use builder `agents(...)` 与 `setting_sources(...)`，SDK 会透传 `--agents` 与 `--setting-sources`。
- Streaming: `stream_input(...)` 发送后自动 `end_input()`；可启用 `include_partial_messages(true)` 获取部分块。
