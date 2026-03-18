spec: task
name: "P0 快速胜利: RateLimitEvent, stop_reason, effort"
tags: [parity, p0, python-sdk]
estimate: 0.5d
---

## 意图

为 cc-sdk 添加三个 Python SDK 已有但 Rust 缺失的小功能，实现生产环境必备的
可观测性和配置能力。这三个功能改动量小但价值高：`RateLimitEvent` 让用户感知
限流状态，`stop_reason` 让用户知道响应结束原因，`effort` 让用户控制推理深度。

## 决策

- `RateLimitEvent` 作为 `Message` 枚举的新变体，包含 `rate_limit_info: RateLimitInfo`, `uuid: String`, `session_id: String`
- `RateLimitInfo` struct 完整字段: `status: RateLimitStatus`, `resets_at: Option<i64>`, `rate_limit_type: Option<RateLimitType>`, `utilization: Option<f64>`, `overage_status: Option<RateLimitStatus>`, `overage_resets_at: Option<i64>`, `overage_disabled_reason: Option<String>`, `raw: serde_json::Value`
- `RateLimitStatus` 枚举: `Allowed`, `AllowedWarning`, `Rejected`
- `RateLimitType` 枚举: `FiveHour`, `SevenDay`, `SevenDayOpus`, `SevenDaySonnet`, `Overage`
- `stop_reason` 添加到 `Message::Result` 变体中，类型为 `Option<String>`
- `effort` 添加到 `ClaudeCodeOptions`，类型为 `Option<Effort>` 枚举（`Low`, `Medium`, `High`, `Max`）
- `effort` 通过 `--effort` CLI flag 传递
- 未知的 `Message` type 不 panic，返回新增的 `Message::Unknown` 变体（前向兼容）
- `AssistantMessage` 添加: `model: Option<String>` (Option 因 Rust 反序列化兼容), `usage: Option<serde_json::Value>`, `error: Option<AssistantMessageError>`, `parent_tool_use_id: Option<String>`
- `AssistantMessageError` 枚举: `AuthenticationFailed`, `BillingError`, `RateLimit`, `InvalidRequest`, `ServerError`, `Unknown`
- `StreamEvent` 作为 `Message` 枚举的新变体，包含 `uuid: String`, `session_id: String`, `event: serde_json::Value`, `parent_tool_use_id: Option<String>`

## 边界

### 允许修改
- claude-code-sdk-rs/src/types.rs
- claude-code-sdk-rs/src/query.rs
- claude-code-sdk-rs/src/internal_query.rs
- claude-code-sdk-rs/src/transport/subprocess.rs
- claude-code-sdk-rs/tests/**

### 禁止
- 不要修改 `Transport` trait 的签名
- 不要修改 WebSocket 模块
- 不要破坏现有 `Message` 枚举的反序列化兼容性

## 验收标准

场景: 反序列化 RateLimitEvent JSON
  测试: test_deserialize_rate_limit_event
  假设 收到如下 NDJSON:
    | 字段 | 值 |
    | type | rate_limit |
    | rate_limit_info.status | allowed_warning |
    | rate_limit_info.utilization | 0.85 |
    | rate_limit_info.resets_at | 1742400000 |
  当 解析为 Message 枚举
  那么 匹配 `Message::RateLimit` 变体
  并且 `rate_limit_info.status` 等于 `RateLimitStatus::AllowedWarning`
  并且 `rate_limit_info.utilization` 等于 `Some(0.85)`

场景: RateLimitEvent rejected 状态
  测试: test_rate_limit_rejected
  假设 收到 `status` 为 "rejected" 的 RateLimitEvent
  当 解析为 Message 枚举
  那么 `rate_limit_info.status` 等于 `RateLimitStatus::Rejected`
  并且 `rate_limit_info.rate_limit_type` 可被读取

场景: ResultMessage 包含 stop_reason
  测试: test_result_message_stop_reason
  假设 收到 Result 类型的 NDJSON 且包含 `"stop_reason": "max_turns"`
  当 解析为 `Message::Result`
  那么 `stop_reason` 等于 `Some("max_turns".to_string())`

场景: ResultMessage 无 stop_reason 时向后兼容
  测试: test_result_message_no_stop_reason
  假设 收到 Result 类型的 NDJSON 且不包含 `stop_reason` 字段
  当 解析为 `Message::Result`
  那么 `stop_reason` 等于 `None`

场景: effort 参数传递到 CLI
  测试: test_effort_cli_flag
  假设 `ClaudeCodeOptions` 设置 `effort` 为 `Effort::High`
  当 构建 CLI 参数列表
  那么 参数列表包含 `"--effort"` 和 `"high"`

场景: effort 不设置时无 CLI flag
  测试: test_effort_none_no_flag
  假设 `ClaudeCodeOptions` 未设置 `effort`
  当 构建 CLI 参数列表
  那么 参数列表不包含 `"--effort"`

场景: AssistantMessage 包含 model 和 usage
  测试: test_assistant_message_model_usage
  假设 收到 Assistant 类型的 NDJSON 且包含 `"model": "claude-sonnet-4-6"` 和 `"usage": {"input_tokens": 100}`
  当 解析为 `Message::Assistant`
  那么 `message.model` 等于 `Some("claude-sonnet-4-6".to_string())`
  并且 `message.usage` 不为 `None`

场景: 未知 Message type 不 panic
  测试: test_unknown_message_type_no_panic
  假设 收到 `"type": "future_new_type"` 的 NDJSON
  当 解析为 Message 枚举
  那么 不 panic
  并且 匹配 `Message::Unknown` 变体或被安全跳过

场景: RateLimitEvent 包含 uuid 和 session_id
  测试: test_rate_limit_event_uuid_session_id
  假设 收到 RateLimitEvent JSON 且包含 `"uuid": "evt-123"` 和 `"session_id": "sess-456"`
  当 解析为 `Message::RateLimit`
  那么 `uuid` 等于 "evt-123"
  并且 `session_id` 等于 "sess-456"

场景: RateLimitInfo overage 字段反序列化
  测试: test_rate_limit_info_overage_fields
  假设 RateLimitInfo JSON 包含 `"overage_status": "rejected"` 和 `"overage_disabled_reason": "plan_limit"`
  当 反序列化为 `RateLimitInfo`
  那么 `overage_status` 等于 `Some(RateLimitStatus::Rejected)`
  并且 `overage_disabled_reason` 等于 `Some("plan_limit".to_string())`

场景: AssistantMessage error 字段反序列化
  测试: test_assistant_message_error
  假设 收到 Assistant NDJSON 且包含 `"error": "rate_limit"`
  当 解析为 `Message::Assistant`
  那么 `message.error` 等于 `Some(AssistantMessageError::RateLimit)`

场景: AssistantMessage parent_tool_use_id 反序列化
  测试: test_assistant_message_parent_tool_use_id
  假设 收到 Assistant NDJSON 且包含 `"parent_tool_use_id": "tool-789"`
  当 解析为 `Message::Assistant`
  那么 `message.parent_tool_use_id` 等于 `Some("tool-789".to_string())`

场景: StreamEvent 反序列化
  测试: test_stream_event_deserialization
  假设 收到 `"type": "stream_event"` 的 NDJSON:
    | 字段 | 值 |
    | uuid | evt-001 |
    | session_id | sess-002 |
    | event.type | content_block_delta |
  当 解析为 Message 枚举
  那么 匹配 `Message::StreamEvent` 变体
  并且 `uuid` 等于 "evt-001"

## 排除范围

- 会话历史 API（list_sessions 等）
- Task 消息类型
- Runtime MCP 控制
- ThinkingConfig 枚举重构
