spec: task
name: "P2 运行时 MCP 控制与 ThinkingConfig"
depends: [p0-quick-wins, p1-task-messages-and-agent-definition]
tags: [parity, p2, python-sdk, mcp]
estimate: 1d
---

## 意图

为 `InteractiveClient` 添加运行时 MCP 服务器管理方法，让用户在会话进行中
动态添加/移除/重连/切换 MCP 服务器。同时将 `max_thinking_tokens` 重构为
类型化的 `ThinkingConfig` 枚举，支持 adaptive 模式。

## 决策

- MCP 控制方法通过 SDK Control Protocol 发送，复用现有 `send_control_request` 机制
- `get_mcp_status()` 返回 `Vec<McpServerStatus>`
- `McpServerStatus` struct: `name`, `status` (枚举), `server_info`, `error`, `tools`
- `McpToolInfo` struct: `name`, `description`, `annotations` (read_only, destructive, open_world)
- `add_mcp_server(name, config)`, `remove_mcp_server(name)`, `reconnect_mcp_server(name)`, `toggle_mcp_server(name, enabled)` 均为 `InteractiveClient` 方法
- `ThinkingConfig` 枚举: `Adaptive`, `Enabled { budget_tokens: i32 }`, `Disabled`
- `ClaudeCodeOptions` 新增 `thinking: Option<ThinkingConfig>`，保留 `max_thinking_tokens` 但标记 `#[deprecated]`
- 如果同时设置 `thinking` 和 `max_thinking_tokens`，`thinking` 优先

## 边界

### 允许修改
- claude-code-sdk-rs/src/types.rs
- claude-code-sdk-rs/src/interactive.rs
- claude-code-sdk-rs/src/internal_query.rs
- claude-code-sdk-rs/src/query.rs
- claude-code-sdk-rs/tests/**

### 禁止
- 不要修改 `SdkMcpServer`（in-process MCP）的 API
- 不要移除 `max_thinking_tokens` 字段（仅 deprecate）
- 不要修改 `Transport` trait

## 验收标准

场景: get_mcp_status 返回服务器状态
  测试: test_get_mcp_status_parse
  假设 控制协议返回包含两个 MCP 服务器状态的 JSON
  当 解析 `McpStatusResponse`
  那么 返回 `Vec<McpServerStatus>` 长度为 "2"
  并且 每个状态包含 `name` 和 `status` 字段

场景: McpServerStatus 状态枚举反序列化
  测试: test_mcp_status_enum_variants
  假设 JSON 包含 `"status": "needs-auth"`
  当 反序列化为 `McpConnectionStatus`
  那么 匹配 `McpConnectionStatus::NeedsAuth`

场景: add_mcp_server 发送正确的控制请求
  测试: test_add_mcp_server_control_request
  假设 调用 `add_mcp_server("fs", McpServerConfig::Stdio { command: "npx", args: ["-y", "@anthropic/mcp-fs"] })`
  当 构建控制请求 JSON
  那么 JSON 包含 `"type": "addMcpServer"` 和 `"name": "fs"`

场景: toggle_mcp_server 启用/禁用
  测试: test_toggle_mcp_server
  假设 调用 `toggle_mcp_server("fs", false)`
  当 构建控制请求 JSON
  那么 JSON 包含 `"type": "toggleMcpServer"` 和 `"enabled": false`

场景: ThinkingConfig::Adaptive 序列化
  测试: test_thinking_config_adaptive
  假设 设置 `thinking` 为 `ThinkingConfig::Adaptive`
  当 序列化为 JSON
  那么 JSON 等于 `{"type": "adaptive"}`

场景: ThinkingConfig::Enabled 带 budget
  测试: test_thinking_config_enabled
  假设 设置 `thinking` 为 `ThinkingConfig::Enabled { budget_tokens: 10000 }`
  当 序列化为 JSON
  那么 JSON 等于 `{"type": "enabled", "budget_tokens": 10000}`

场景: ThinkingConfig 优先于 max_thinking_tokens
  测试: test_thinking_config_priority
  假设 同时设置 `thinking: Adaptive` 和 `max_thinking_tokens: 5000`
  当 构建 CLI 参数
  那么 使用 `thinking` 的配置
  但是 不使用 `max_thinking_tokens` 的值

场景: ThinkingConfig::Disabled 传递
  测试: test_thinking_config_disabled_cli
  假设 设置 `thinking` 为 `ThinkingConfig::Disabled`
  当 构建 CLI 参数
  那么 参数列表包含禁用思考的标志

场景: remove_mcp_server 发送正确控制请求
  测试: test_remove_mcp_server_control_request
  假设 调用 `remove_mcp_server("fs")`
  当 构建控制请求 JSON
  那么 JSON 包含 `"type": "removeMcpServer"` 和 `"name": "fs"`

场景: get_mcp_status CLI 未连接时返回错误
  测试: test_get_mcp_status_not_connected
  假设 `InteractiveClient` 未调用 `connect()`
  当 调用 `get_mcp_status()`
  那么 返回 `Err(SdkError::InvalidState)`

## 排除范围

- MCP 服务器的健康检查轮询
- MCP 工具调用的拦截/代理
- 自动重连策略（由用户控制）
