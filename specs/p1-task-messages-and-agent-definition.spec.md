spec: task
name: "P1 Task 消息与 AgentDefinition 增强"
depends: [p0-quick-wins]
tags: [parity, p1, python-sdk, subagent]
estimate: 1d
---

## 意图

添加 Task 生命周期消息类型（TaskStarted、TaskProgress、TaskNotification），
让用户在使用 Agent 工具时能感知子代理的执行进度和结果。同时扩展 `AgentDefinition`
增加 `skills`、`memory`、`mcpServers` 字段，与 Python SDK v0.1.48+ 对齐。
还需在 Hook 输入类型上添加 `agent_id` 和 `agent_type` 字段。

## 决策

- Task 消息通过 `Message::System` 的 `subtype` 字段区分，提供 typed helper 方法
- `TaskStartedMessage` struct: `task_id: String`, `description: String`, `uuid: String`, `session_id: String`, `tool_use_id: Option<String>`, `task_type: Option<String>`
- `TaskProgressMessage` struct: `task_id: String`, `description: String`, `usage: TaskUsage`, `uuid: String`, `session_id: String`, `tool_use_id: Option<String>`, `last_tool_name: Option<String>`
- `TaskNotificationMessage` struct: `task_id`, `status` (枚举: `Completed`, `Failed`, `Stopped`), `output_file: String`, `summary: String`, `uuid: String`, `session_id: String`, `tool_use_id: Option<String>`, `usage: Option<TaskUsage>`
- `TaskUsage` struct (与 Python SDK 对齐): `total_tokens: u64`, `tool_uses: u64`, `duration_ms: u64`
- `AgentDefinition` 新增: `skills: Option<Vec<String>>`, `memory: Option<String>`, `mcp_servers: Option<Vec<serde_json::Value>>`
- 所有 Hook 输入的 base trait/struct 添加 `agent_id: Option<String>`, `agent_type: Option<String>`
- 使用 `#[serde(default)]` 保持向后兼容

## 边界

### 允许修改
- claude-code-sdk-rs/src/types.rs
- claude-code-sdk-rs/tests/**

### 禁止
- 不要修改 `Message` 枚举的 tag 策略（保持 `#[serde(tag = "type")]`）
- 不要在 `Message` 枚举中为 Task 添加顶级变体（它们是 System 消息的子类型）
- 不要移除 `AgentDefinition` 现有字段

## 验收标准

场景: 解析 TaskStarted 系统消息
  测试: test_parse_task_started
  假设 收到 System 类型的 NDJSON:
    | 字段 | 值 |
    | subtype | task_started |
    | data.task_id | task-abc-123 |
    | data.description | Searching codebase |
    | data.task_type | agent |
  当 解析为 `Message::System`
  那么 调用 `as_task_started()` 返回 `Some(TaskStartedMessage)`
  并且 `task_id` 等于 "task-abc-123"

场景: 解析 TaskProgress 系统消息
  测试: test_parse_task_progress
  假设 收到 subtype 为 "task_progress" 的 System 消息
  当 调用 `as_task_progress()`
  那么 返回 `Some(TaskProgressMessage)`
  并且 `usage.input_tokens` 可被读取

场景: 解析 TaskNotification 完成状态
  测试: test_parse_task_notification_completed
  假设 收到 subtype 为 "task_notification" 的 System 消息
  并且 data 包含 `"status": "completed"` 和 `"summary": "Found 3 matches"`
  当 调用 `as_task_notification()`
  那么 `status` 等于 `TaskStatus::Completed`
  并且 `summary` 等于 `Some("Found 3 matches".to_string())`

场景: TaskNotification 失败状态
  测试: test_parse_task_notification_failed
  假设 收到 task_notification 且 `"status": "failed"`
  当 调用 `as_task_notification()`
  那么 `status` 等于 `TaskStatus::Failed`

场景: AgentDefinition 序列化含 skills 和 memory
  测试: test_agent_definition_with_skills_memory
  假设 创建 `AgentDefinition` 并设置:
    | 字段 | 值 |
    | skills | ["code-review", "commit"] |
    | memory | project |
    | mcp_servers | ["filesystem"] |
  当 序列化为 JSON
  那么 JSON 包含 `"skills": ["code-review", "commit"]`
  并且 JSON 包含 `"memory": "project"`

场景: AgentDefinition 无新字段时向后兼容
  测试: test_agent_definition_backward_compat
  假设 JSON 只包含 `description`, `prompt`, `tools`, `model`
  当 反序列化为 `AgentDefinition`
  那么 `skills` 等于 `None`
  并且 `memory` 等于 `None`
  并且 `mcp_servers` 等于 `None`

场景: Hook 输入包含 agent_id 和 agent_type
  测试: test_hook_input_agent_fields
  假设 收到 PreToolUse hook 输入 JSON 且包含 `"agent_id": "sub-1"` 和 `"agent_type": "Explore"`
  当 反序列化为 `PreToolUseHookInput`
  那么 `agent_id` 等于 `Some("sub-1".to_string())`
  并且 `agent_type` 等于 `Some("Explore".to_string())`

场景: Hook 输入无 agent 字段时兼容
  测试: test_hook_input_no_agent_fields
  假设 收到 PreToolUse hook 输入 JSON 且不包含 `agent_id` 和 `agent_type`
  当 反序列化为 `PreToolUseHookInput`
  那么 `agent_id` 等于 `None`
  并且 `agent_type` 等于 `None`

场景: TaskUsage 字段反序列化
  测试: test_task_usage_deserialization
  假设 JSON 包含 `{"total_tokens": 700, "tool_uses": 5, "duration_ms": 12000}`
  当 反序列化为 `TaskUsage`
  那么 `total_tokens` 等于 "700"
  并且 `tool_uses` 等于 "5"
  并且 `duration_ms` 等于 "12000"

场景: serde(default) 保持向后兼容
  测试: test_serde_default_backward_compat
  假设 System 消息的 data 字段缺少 `usage` 键
  当 解析 TaskProgressMessage
  那么 `usage` 使用默认值而非报错

## 排除范围

- 会话历史 API
- Runtime MCP 控制
- `stop_task()` 交互式客户端方法（P2）
