spec: task
name: "P1 会话历史 API"
depends: [p0-quick-wins]
tags: [parity, p1, python-sdk, session]
estimate: 1d
---

## 意图

实现会话历史管理功能，让用户能程序化地列出、查询、重命名和标记会话。
这些功能通过调用 Claude Code CLI 的 `--session-*` 子命令实现，不涉及
直接读取文件系统。对 agent-teams 项目的跨会话协调尤其重要。

## 决策

- `list_sessions()` 调用 CLI `claude sessions list --json` 命令，支持 `include_worktrees: bool` 参数（默认 true）
- `get_session_messages()` 调用 CLI `claude sessions messages --json` 命令
- `rename_session()` 调用 CLI `claude sessions rename` 命令
- `tag_session()` 调用 CLI `claude sessions tag` 命令，含 Unicode 清洗
- 返回类型为独立 struct（`SessionInfo`, `SessionMessage`），放在新模块 `src/sessions.rs`
- `SessionInfo` 字段: `session_id`, `summary`, `last_modified` (i64 ms), `file_size` (u64), `custom_title`, `first_prompt`, `git_branch`, `cwd`
- `SessionMessage` 字段: `msg_type` ("user"/"assistant"), `uuid`, `session_id`, `message` (serde_json::Value)
- 这些函数既作为独立顶层函数导出，也作为 `InteractiveClient` 方法提供
- 使用 `tokio::process::Command` 调用 CLI，不依赖现有 Transport

## 边界

### 允许修改
- claude-code-sdk-rs/src/sessions.rs（新文件）
- claude-code-sdk-rs/src/lib.rs（添加 mod 和 pub use）
- claude-code-sdk-rs/src/interactive.rs（添加方法）
- claude-code-sdk-rs/tests/**

### 禁止
- 不要直接读取 `~/.claude/` 下的会话文件
- 不要修改现有的 `Transport` trait
- 不要在 `ClaudeCodeOptions` 中添加会话历史相关字段

## 验收标准

场景: list_sessions 返回会话列表
  测试: test_list_sessions_parse
  假设 CLI 返回包含两个会话的 JSON 数组
  当 调用 `list_sessions(None, None)`
  那么 返回 `Vec<SessionInfo>` 长度为 "2"
  并且 每个 `SessionInfo` 的 `session_id` 不为空

场景: list_sessions 带 directory 过滤
  测试: test_list_sessions_with_directory
  假设 指定 directory 为 "/tmp/project"
  当 构建 CLI 命令
  那么 命令参数包含 "--directory" 和 "/tmp/project"

场景: list_sessions 带 limit
  测试: test_list_sessions_with_limit
  假设 指定 limit 为 "10"
  当 构建 CLI 命令
  那么 命令参数包含 "--limit" 和 "10"

场景: get_session_messages 返回消息列表
  测试: test_get_session_messages_parse
  假设 CLI 返回包含 user 和 assistant 消息的 JSON 数组
  当 调用 `get_session_messages("session-uuid", None, None, 0)`
  那么 返回 `Vec<SessionMessage>`
  并且 第一条消息的 `msg_type` 为 "user"

场景: get_session_messages 带 offset 和 limit
  测试: test_get_session_messages_with_pagination
  假设 指定 offset 为 "5" 和 limit 为 "10"
  当 构建 CLI 命令
  那么 命令参数包含 "--offset" "5" "--limit" "10"

场景: rename_session 调用成功
  测试: test_rename_session
  假设 session_id 为 "abc-123" 且 title 为 "My Session"
  当 调用 `rename_session("abc-123", "My Session")`
  那么 CLI 命令包含 "rename" "abc-123" 和 "My Session"

场景: tag_session Unicode 清洗
  测试: test_tag_session_unicode_sanitize
  假设 tag 包含不可见 Unicode 字符 "\u{200B}tag\u{FEFF}"
  当 调用 `tag_session("abc-123", Some("tag"))`
  那么 传递给 CLI 的 tag 已移除零宽字符

场景: tag_session 清除标签
  测试: test_tag_session_clear
  假设 tag 为 `None`
  当 调用 `tag_session("abc-123", None)`
  那么 CLI 命令包含 "--clear" flag

场景: CLI 不存在时返回错误
  测试: test_session_api_cli_not_found
  假设 CLI 可执行文件不在 PATH 中
  当 调用 `list_sessions(None, None)`
  那么 返回 `Err(SdkError::CliNotFound)`

## 排除范围

- 会话内容的全文搜索
- 会话的删除/清理
- 会话导出（Markdown/JSON）
