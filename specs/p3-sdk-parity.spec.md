spec: task
name: "P3: 官方 SDK 对等 — 对齐 Python SDK + CC 源码合约"
inherits: project
tags: [parity, python-sdk, cc-source, v0.8.0]
depends: [p2-runtime-mcp-and-thinking]
estimate: 5d
---

## 意图

将 Rust SDK (v0.7.0) 与官方 Python SDK (v0.1.55) + Claude Code v2.1.88 源码合约对齐。当前 Rust SDK 在权限、控制协议、Hook 事件、客户端 API、配置选项、会话管理等方面存在显著缺口。

## 参考基线

| 来源 | 版本 | 用途 |
|------|------|------|
| Python SDK | v0.1.55 (`claude-agent-sdk-python`) | 官方 API surface 参考 |
| CC 源码 | v2.1.88 (`restored-src/src/entrypoints/sdk/coreSchemas.ts`) | 完整类型合约 |
| CC v2.1.91 | bundle 逆向 | 新增字段（auto mode、staleReadFileStateHint） |

## 差距总览

### 统计

| 维度 | Python SDK | Rust SDK v0.7.0 | 缺失数 |
|------|-----------|-----------------|--------|
| 权限模式 | 5 | 4 | 1 (`dontAsk`) |
| Hook 事件 | 10 | 5 | 5 |
| 客户端方法 | 13 | 5 | 8 |
| 控制请求子类型 | 11 | 6 | 5 |
| 配置选项字段 | 30+ | 18 | 12+ |
| Session API | 7 函数 | 4 函数 | 3 |
| 消息专用类型 | 3 (Task*) | 0 | 3 |

---

## P0: 用户会撞到的问题

### P0-1: 补齐 `dontAsk` 权限模式

**Python SDK**: `PermissionMode = "default" | "acceptEdits" | "plan" | "bypassPermissions" | "dontAsk"`

**Rust SDK 缺失**: `DontAsk` — 不提示，未预批准则直接拒绝

```rust
// types.rs
pub enum PermissionMode {
    Default,
    AcceptEdits,
    Plan,
    BypassPermissions,
    DontAsk,  // ← 新增
}
```

**建议**: 同时添加 `#[non_exhaustive]` 为未来 `Auto` 模式预留。

### P0-2: 补齐 `can_use_tool` 权限回调

**Python SDK** 核心权限机制:
```python
CanUseTool = Callable[
    [str, dict[str, Any], ToolPermissionContext],
    Awaitable[PermissionResult]
]
```

**Rust SDK**: 有 `ToolPermissionContext` 和 `PermissionResult` 类型定义，但 `ClaudeAgentOptions` 中**无回调字段**。用户无法在 SDK 层拦截权限请求。

```rust
// 建议添加到 ClaudeCodeOptions
pub can_use_tool: Option<Arc<dyn CanUseTool>>,

#[async_trait]
pub trait CanUseTool: Send + Sync {
    async fn check(
        &self,
        tool_name: &str,
        input: &serde_json::Value,
        context: &ToolPermissionContext,
    ) -> Result<PermissionResult>;
}
```

### P0-3: 补齐客户端控制方法

**Python `ClaudeSDKClient`** 有 13 个方法，Rust SDK 缺 8 个:

| 方法 | 优先级 | 用途 |
|------|--------|------|
| `set_permission_mode()` | P0 | 运行时切换权限模式 |
| `set_model()` | P0 | 运行时切换模型 |
| `get_context_usage()` | P0 | 获取 token 分布和缓存命中 |
| `get_mcp_status()` | P1 | 查询 MCP 服务器状态 |
| `reconnect_mcp_server()` | P1 | 重连失败的 MCP 服务器 |
| `toggle_mcp_server()` | P1 | 启用/禁用 MCP 服务器 |
| `stop_task()` | P1 | 停止后台任务 |
| `get_server_info()` | P2 | 获取初始化信息 |

**实现方式**: 每个方法对应一个控制请求子类型，通过 `internal_query.rs` 的控制协议发送。

### P0-4: 补齐 Hook 事件

**Python SDK 有但 Rust SDK 缺失的 5 个事件**:

| 事件 | Hook Input 类型 | 关键字段 |
|------|----------------|---------|
| `SubagentStart` | `SubagentStartHookInput` | `agent_id`, `agent_type` |
| `SubagentStop` | `SubagentStopHookInput` | `agent_id`, `agent_type`, `agent_transcript_path` |
| `PreCompact` | `PreCompactHookInput` | `trigger` ("manual"/"auto"), `custom_instructions` |
| `Notification` | `NotificationHookInput` | `message`, `title`, `notification_type` |
| `PermissionRequest` | `PermissionRequestHookInput` | `tool_name`, `tool_input`, `permission_suggestions` |

**Rust SDK 多出的事件** (Python 没有):
- `SessionStart` / `SessionEnd` — 保留，CC 源码合约中存在

### P0-5: `cache_read_input_tokens` / `cache_creation_input_tokens` 解析

**Python SDK** 的 `ResultMessage.usage` 和 `ContextUsageResponse.apiUsage` 包含:
```python
apiUsage: {
    input_tokens: int,
    output_tokens: int,
    cache_creation_input_tokens: int,
    cache_read_input_tokens: int
}
```

**Rust SDK** 的 `TokenUsageTracker` 仅追踪 `input_tokens` / `output_tokens`，丢失缓存信息。

---

## P1: 配置完整性

### P1-1: 配置选项补齐

| 字段 | Python 类型 | 说明 |
|------|-----------|------|
| `disallowed_tools` | `list[str]` | 工具黑名单 |
| `continue_conversation` | `bool` | 继续上一会话 |
| `resume` | `str | None` | 恢复指定会话 ID |
| `fork_session` | `bool` | 分叉会话 |
| `max_turns` | `int | None` | 最大回合数限制 |
| `betas` | `list[SdkBeta]` | Beta 特性列表（如 `context-1m-2025-08-07`） |
| `setting_sources` | `list[SettingSource]` | 配置层来源（user/project/local） |
| `enable_file_checkpointing` | `bool` | 文件检查点 |
| `task_budget` | `TaskBudget | None` | 任务预算 |
| `include_partial_messages` | `bool` | 包含部分流消息 |
| `stderr` | `Callable | None` | CLI stderr 回调 |
| `extra_args` | `dict[str, str | None]` | 任意 CLI 参数透传 |
| `system_prompt` preset/file | `SystemPromptPreset | SystemPromptFile` | 预设或文件路径模式 |

### P1-2: `AgentDefinition` 字段补齐

**Python SDK 完整定义**:
```python
@dataclass
class AgentDefinition:
    description: str
    prompt: str
    tools: list[str] | None
    disallowedTools: list[str] | None      # ← Rust 缺失
    model: str | None
    skills: list[str] | None               # ← Rust 缺失
    memory: "user"|"project"|"local"|None   # ← Rust 缺失
    mcpServers: list[...] | None
    initialPrompt: str | None              # ← Rust 缺失
    maxTurns: int | None                   # ← Rust 缺失
    background: bool | None                # ← Rust 缺失
    effort: "low"|"medium"|"high"|"max"|int|None  # ← Rust 缺失
    permissionMode: PermissionMode | None
```

### P1-3: MCP 全生命周期控制请求

补齐控制请求:
- `mcp_status` → `McpStatusResponse`
- `mcp_set_servers` → 动态添加/移除 MCP 服务器
- `mcp_reconnect` → 重连单个服务器
- `mcp_toggle` → 启用/禁用服务器

### P1-4: `ContextUsageResponse` 类型

**Python SDK** 返回完整的上下文分布:
```python
class ContextUsageResponse(TypedDict):
    categories: list[ContextUsageCategory]
    totalTokens: int
    maxTokens: int
    percentage: float
    model: str
    isAutoCompactEnabled: bool
    autoCompactThreshold: int | None
    memoryFiles: list[...]
    mcpTools: list[...]
    messageBreakdown: dict | None
    apiUsage: dict | None  # 含 cache_read/cache_creation
```

---

## P2: Session + 消息增强

### P2-1: Session API 补齐

| 函数 | Python | Rust | 差距 |
|------|--------|------|------|
| `get_session_info()` | ✅ | ❌ | 新增 |
| `delete_session()` | ✅ | ❌ | 新增 |
| `fork_session()` | ✅ `→ ForkSessionResult` | ❌ | 新增 |

### P2-2: Task 消息专用类型

Python SDK 将 `SystemMessage` 的 task 子类型拆分为专用类型:

```rust
// 建议添加到 Message enum
pub enum Message {
    // ... 现有变体
    TaskStarted { task_id: String, description: String, ... },
    TaskProgress { task_id: String, usage: TaskUsage, ... },
    TaskNotification { task_id: String, status: TaskNotificationStatus, ... },
}
```

### P2-3: 其他客户端方法

- `rewind_files(user_message_id)` — 完善文件检查点回滚
- `stop_task(task_id)` — 停止后台任务
- `get_server_info()` — 获取 `SDKControlInitializeResponse`

---

## P3: 代码质量

### P3-1: 清理 dead code

- 删除 `client.rs`、`client_v2.rs`、`client_final.rs`（标注编译失败）
- 移除未激活的 `#[allow(dead_code)]` 代码或提取到 feature gate
- 清理 process pool 中已注释/未使用的逻辑

### P3-2: 消除 unsafe 环境变量

```rust
// 当前: client 初始化中
unsafe { std::env::set_var("CLAUDE_CODE_...", value) }

// 改为: 通过 CLI 参数传递
command.arg("--env").arg(format!("{}={}", key, value))
// 或通过 Command::env()
command.env(key, value)
```

### P3-3: `#[non_exhaustive]` 保护

对以下枚举添加 `#[non_exhaustive]`，避免未来扩展破坏下游:
- `PermissionMode`
- `Message`
- `SdkError`
- `HookEvent`（如果是枚举）

---

## 验收标准

### P0 验收

- [ ] `PermissionMode::DontAsk` 可编译且序列化为 `"dontAsk"`
- [ ] `can_use_tool` 回调能拦截工具权限请求并返回 allow/deny
- [ ] `client.set_permission_mode()` / `client.set_model()` 发送控制请求并收到响应
- [ ] `client.get_context_usage()` 返回 `ContextUsageResponse` 含 token 分布
- [ ] 5 个新 Hook 事件有对应的输入类型且能正确路由回调
- [ ] `TokenUsageTracker` 追踪 `cache_read_input_tokens` 和 `cache_creation_input_tokens`
- [ ] 所有新增类型在 `lib.rs` 中 re-export

### P1 验收

- [ ] `ClaudeCodeOptions` 包含所有 Python SDK 对应字段
- [ ] `AgentDefinition` 字段与 Python SDK 对齐
- [ ] MCP 4 个控制请求可正常发送/接收
- [ ] `ContextUsageResponse` 类型与 Python SDK 匹配

### P2 验收

- [ ] `delete_session()` / `fork_session()` 通过测试
- [ ] Task 消息拆分为专用类型，现有代码不受影响
- [ ] `stop_task()` 可停止后台任务

### P3 验收

- [ ] 零 `#[allow(dead_code)]`（或全部有 feature gate 理由）
- [ ] 零 `unsafe` 环境变量操作
- [ ] 关键枚举有 `#[non_exhaustive]`

## 边界

### 允许修改
- `claude-code-sdk-rs/src/**`
- `claude-code-sdk-rs/Cargo.toml`

### 禁止
- 不破坏现有公开 API（只新增，不修改签名）
- 不引入 Python 依赖
- 不直接调用 Anthropic API（始终通过 CLI 子进程）

## 排除范围

- 不实现 Rust 原生的 Anthropic Messages API 客户端
- 不实现 WebSocket transport 的 P0-P2 功能（保持 feature gate）
- 不对 `claude-code-api`（API 服务器）做对齐更新（另开 spec）
