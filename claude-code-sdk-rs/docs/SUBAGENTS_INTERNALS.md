# Subagents 工作机制深度解析 (Rust SDK)

> 理解 Claude Code Subagents 的内部运作原理

**配套文档**:
- [SUBAGENTS_BEST_PRACTICES.md](./SUBAGENTS_BEST_PRACTICES.md) - 使用指南
- 本文档 - 工作机制

---

## 目录

- [核心工作机制](#核心工作机制)
- [调用流程详解](#调用流程详解)
- [并行 Subagents 机制](#并行-subagents-机制)
- [SDK 实现机制](#sdk-实现机制)
- [关键概念澄清](#关键概念澄清)
- [执行时序图](#执行时序图)
- [核心 Takeaways](#核心-takeaways)
- [未知部分](#未知部分)

---

## 核心工作机制

### 1. Subagent 不是"另一个进程"

```
误解 ❌：Subagent 是独立运行的 AI 实例
真相 ✅：Subagent 是主会话中的一个"角色切换"

主会话 (Session)
│
├─ 主 Claude (默认系统提示词)
│  └─ 用户对话
│
└─ Subagent 调用
   ├─ 临时切换系统提示词
   ├─ 临时限制工具权限
   ├─ 使用独立上下文窗口
   └─ 完成后返回结果给主会话
```

**类比**：
```
主 Claude = 通才医生
Subagent = 专科医生会诊

通才医生遇到专业问题时：
1. 调用专科医生（subagent）
2. 专科医生用专业知识处理
3. 结果返回给通才医生
4. 通才医生继续与患者对话

整个过程是同一个"医院"（进程），
只是角色和专业知识临时切换。
```

### 2. 上下文隔离的实现方式

```
技术实现：
┌─────────────────────────────────────┐
│ 主会话上下文窗口 (200K tokens)      │
│                                     │
│ User: "Review my code"              │
│ Claude: "I'll use code-reviewer"    │
│                                     │
│ ┌─────────────────────────────┐    │
│ │ Subagent 上下文 (独立窗口)  │    │
│ │                             │    │
│ │ System: "You are reviewer"  │    │
│ │ User: [代理的任务]          │    │
│ │ Tools: [Read, Grep only]    │    │
│ │                             │    │
│ │ → 执行审查                  │    │
│ │ → 返回结果                  │    │
│ └─────────────────────────────┘    │
│                                     │
│ Claude: "Review complete: ..."      │
└─────────────────────────────────────┘
```

**关键点**：
- ✅ Subagent 有**独立的消息历史**
- ✅ 主会话看不到 subagent 的中间步骤
- ✅ Subagent 只返回最终结果
- ✅ 上下文隔离防止污染主对话

---

## 调用流程详解

### 场景 1: 显式调用

```rust
// 用户输入
let mut messages = query(
    "Use the code-reviewer agent to review auth.rs",
    Some(options)
).await?;
```

**执行流程**：

```
┌─────────────────────────────────────────────────────┐
│ Step 1: 主 Claude 解析请求                          │
│   - 识别显式调用："code-reviewer agent"             │
│   - 加载 agent 定义                                 │
└──────────────────┬──────────────────────────────────┘
                   ↓
┌─────────────────────────────────────────────────────┐
│ Step 2: 创建 Subagent 上下文                        │
│   - 系统提示词: agent.prompt                        │
│   - 工具限制: agent.tools                           │
│   - 模型: agent.model 或继承                        │
│   - 初始消息: "Review auth.rs"                      │
└──────────────────┬──────────────────────────────────┘
                   ↓
┌─────────────────────────────────────────────────────┐
│ Step 3: Subagent 执行                               │
│   [在独立上下文中]                                  │
│   - Read auth.rs                                    │
│   - Grep for patterns                               │
│   - 分析代码                                        │
│   - 生成审查报告                                    │
└──────────────────┬──────────────────────────────────┘
                   ↓
┌─────────────────────────────────────────────────────┐
│ Step 4: 结果返回主会话                              │
│   - Subagent 完成                                   │
│   - 结果注入主会话上下文                            │
│   - 主 Claude 继续对话                              │
└─────────────────────────────────────────────────────┘
```

### 场景 2: 自动委托

```rust
// 用户输入（未明确指定 agent）
let mut messages = query(
    "I just modified auth.rs, can you check it?",
    Some(options)
).await?;
```

**执行流程**：

```
┌─────────────────────────────────────────────────────┐
│ Step 1: 主 Claude 推理                              │
│   - 分析请求："check" modified code                 │
│   - 匹配 agent descriptions                         │
│   - code-reviewer.description 包含:                 │
│     "Use PROACTIVELY after code changes"            │
│   - 决定：调用 code-reviewer                        │
└──────────────────┬──────────────────────────────────┘
                   ↓
┌─────────────────────────────────────────────────────┐
│ Step 2: 隐式委托                                    │
│   主 Claude: "Let me review your changes using      │
│              the code-reviewer agent..."            │
└──────────────────┬──────────────────────────────────┘
                   ↓
              [同上 Step 2-4]
```

---

## 并行 Subagents 机制

### 真实案例：9 个并行 Agents

```
用户的 Plan Mode 请求：
"Spawn 9 parallel sub-agents to process each section"

任务：从 48 个课程 README 中提取学习要点
```

**实际执行（推测）**：

```
┌─────────────────────────────────────────────────────┐
│ 主会话上下文                                        │
│                                                     │
│ User: "Spawn 9 agents: extractor-01, ..."          │
│ Claude: "Starting parallel extraction..."          │
│                                                     │
│ ┌──────────────┐ ┌──────────────┐ ┌─────────────┐ │
│ │ Subagent 1   │ │ Subagent 2   │ │ Subagent 3  │ │
│ │ (Section 01) │ │ (Section 02) │ │ (Section 03)│ │
│ │              │ │              │ │             │ │
│ │ - Read READMEs│ │ - Read READMEs│ │- Read ...  │ │
│ │ - Extract... │ │ - Extract... │ │- Extract...│ │
│ │ → Result 1   │ │ → Result 2   │ │→ Result 3  │ │
│ └──────────────┘ └──────────────┘ └─────────────┘ │
│                                                     │
│ [... Subagents 4-9 similarly ...]                  │
└─────────────────────────────────────────────────────┘
```

### 两阶段处理模式

```
Stage 1: 并行提取（9 agents）
  └─ 每个 agent 处理自己的 section
  └─ 输出临时结果

Stage 2: 串行去重（1 agent）
  └─ 读取所有临时结果
  └─ 去重并生成最终文件

这是处理大规模任务的优秀模式：
✅ Map phase: 并行处理独立任务
✅ Reduce phase: 串行汇总结果
```

---

## SDK 实现机制

### Rust SDK 内部流程

```rust
// 1. 用户定义 agents
let mut agents = HashMap::new();
agents.insert(
    "code-reviewer".to_string(),
    AgentDefinition {
        description: "Expert code reviewer...".to_string(),
        prompt: "You are a senior code reviewer...".to_string(),
        tools: Some(vec!["Read".into(), "Grep".into()]),
        model: Some("sonnet".to_string()),
    }
);

let options = ClaudeCodeOptions::builder()
    .agents(agents)
    .build();

// 2. SDK 内部序列化（在 subprocess.rs 中）
// claude-code-sdk-rs/src/transport/subprocess.rs:296-301

if let Some(ref agents) = self.options.agents {
    if !agents.is_empty() {
        // 序列化为 JSON
        if let Ok(json_str) = serde_json::to_string(agents) {
            // 传递给 CLI
            cmd.arg("--agents").arg(json_str);
            //     ^^^^^^^^^^  ^^^^^^^^^^^^
            //     CLI flag    JSON 序列化的 agents
        }
    }
}

// 3. CLI 接收并加载 agents
// 4. Claude 在对话中可以访问这些 agents

// 5. 当 Claude 调用 agent 时（推测）：
// 内部工具调用类似：
// {
//   "type": "tool_use",
//   "name": "Task",
//   "input": {
//     "subagent_type": "code-reviewer",
//     "prompt": "Review auth.rs"
//   }
// }

// 6. CLI 创建 subagent 上下文并执行
// 7. 结果通过 Message stream 返回
```

### 类型安全的优势

```rust
// ✅ Rust 的类型系统确保正确性

// 编译时检查
let agent = AgentDefinition {
    description: "...",
    prompt: "...",
    tools: Some(vec!["Read".into()]),  // Vec<String>
    model: Some("sonnet".into()),       // Option<String>
};

// 错误会在编译时捕获
let agent = AgentDefinition {
    tools: Some(vec![123]),  // ❌ 编译错误：类型不匹配
};

// 使用 builder pattern 进一步增强安全性
let options = ClaudeCodeOptions::builder()
    .agents(agents)  // HashMap<String, AgentDefinition>
    .build();
```

### 工具限制的实现

```rust
// 定义时限制工具
AgentDefinition {
    tools: Some(vec!["Read".into(), "Grep".into()]),
    // ...
}

// 执行时强制限制
// 当 subagent 尝试调用 "Bash" 时：
→ CLI 拒绝执行
→ 返回错误

// 这是硬限制，不是"建议"
```

---

## 关键概念澄清

### 1. Subagent ≠ 独立进程

```
❌ 误解：每个 subagent 是一个独立的 Claude 实例
✅ 真相：Subagent 是主会话中的"专家模式"

整个过程是同一个"进程"，只是角色和专业知识临时切换。
```

### 2. 上下文隔离的目的

```
为什么需要独立上下文？

问题：如果 subagent 共享主会话上下文
┌─────────────────────────────────────┐
│ 主会话: 100 轮对话历史               │
│ + Subagent: 分析 50 个文件           │
│ = 超出 200K token 限制 ❌            │
└─────────────────────────────────────┘

解决：独立上下文
┌─────────────────────────────────────┐
│ 主会话: 100 轮对话 (50K tokens)      │
│                                     │
│ Subagent: 独立窗口 (最多 200K)      │
│   - 只包含任务相关内容              │
│   - 完成后只返回结果摘要            │
│   - 详细过程丢弃                    │
└─────────────────────────────────────┘

优势：
✅ 主会话上下文保持精简
✅ Subagent 可以处理大量临时数据
✅ 支持更长的整体对话
```

### 3. 工具限制 = 硬限制

```rust
// 定义时限制
AgentDefinition {
    tools: Some(vec!["Read".into()]),
    // ...
}

// 执行时强制：
→ 只能调用 Read
→ 尝试调用 Bash 会被拒绝
→ 无法绕过
```

### 4. 模型选择

```rust
// 三种配置
AgentDefinition {
    model: Some("sonnet".into()),  // 固定使用 sonnet
    // ...
}

AgentDefinition {
    model: Some("inherit".into()),  // 继承主会话模型
    // ...
}

AgentDefinition {
    model: None,  // 使用默认 subagent 模型
    // ...
}
```

### 5. 返回结果 = 摘要

```
Subagent 内部可能：
- Read 50 个文件
- 运行 100 次 Grep
- 生成详细分析

但返回主会话的只是：
- 最终结论
- 关键发现
- 推荐行动
```

---

## 执行时序图

### 完整的调用流程

```
时间线（显式调用示例）:

T0: 用户输入
    let mut messages = query(
        "Use code-reviewer to review auth.rs",
        Some(options)
    ).await?;

T1: 主 Claude 处理
    - 识别 agent: code-reviewer
    - 准备 subagent 调用
    - 输出: "I'll review using code-reviewer..."

T2: 创建 Subagent 上下文
    System: "You are a senior code reviewer..."
    User: "Review auth.rs"
    Tools: [Read, Grep, Glob, Bash]
    Model: sonnet

T3: Subagent 执行
    - 调用 Read(auth.rs)
    - 调用 Grep 搜索模式
    - 分析代码
    - 生成报告

T4: Subagent 完成
    返回: "Found 3 issues: ..."

T5: 主会话继续
    Message::Assistant {
        content: [
            TextBlock {
                text: "Review complete. Found 3 issues:..."
            }
        ]
    }

T6: 后续处理
    while let Some(msg) = messages.next().await {
        match msg? {
            Message::Assistant { message } => { /* 处理 */ }
            Message::Result { .. } => break,
            _ => {}
        }
    }
```

---

## 核心 Takeaways

### 1. 本质
```
Subagent = 临时的专家模式
         + 独立上下文
         + 工具限制
         + 专业提示词
```

### 2. 上下文管理
```
主会话：长期对话历史（精简）
Subagent：临时任务上下文（详细）
结果：只返回摘要，不污染主会话
```

### 3. 并行能力
```
✅ 可能支持真正的并行执行
✅ 通过明确指示实现
✅ 适合大规模独立任务
⚠️  需要明确指示才能保证并行
```

### 4. 调用方式
```
显式：query("Use X agent to do Y", options)
隐式：依赖 description 匹配
Plan Mode：明确并行策略（推荐）
```

### 5. 类型安全
```
✅ Rust 的类型系统在编译时捕获错误
✅ HashMap<String, AgentDefinition> 保证正确性
✅ Option<Vec<String>> 明确可选性
✅ Result<T, E> 强制错误处理
```

### 6. 错误处理

```rust
// ✅ Rust 强制处理错误
let agents = match load_agents_from_file("config.json") {
    Ok(a) => a,
    Err(e) => {
        eprintln!("Failed to load agents: {}", e);
        HashMap::new()
    }
};

// ❌ 不能忽略错误（不像某些语言）
// let agents = load_agents_from_file("config.json");  // 编译错误
```

---

## Rust 特定优势

### 1. 零成本抽象

```rust
// Agent 定义没有运行时开销
let agent = AgentDefinition { ... };  // 栈分配，零成本

// HashMap 高效
let mut agents = HashMap::new();
agents.insert("reviewer".into(), agent);  // O(1) 插入
```

### 2. 所有权系统

```rust
// 编译时防止数据竞争
let options = ClaudeCodeOptions::builder()
    .agents(agents)  // agents 的所有权转移
    .build();

// agents 不能再使用（编译时错误）
// println!("{:?}", agents);  // ❌ 编译错误：所有权已转移
```

### 3. 并发安全

```rust
use tokio::spawn;

// ✅ 安全的并发处理
let handles: Vec<_> = sections
    .into_iter()
    .map(|section| {
        spawn(async move {
            // 每个 section 独立处理
            process_section(section).await
        })
    })
    .collect();

// 等待所有完成
for handle in handles {
    handle.await?;
}
```

### 4. 模式匹配

```rust
// ✅ 穷尽性检查
while let Some(msg) = messages.next().await {
    match msg? {
        Message::Assistant { message } => {
            // 处理 assistant 消息
        }
        Message::Result { .. } => {
            break;  // 结束
        }
        Message::User { .. } => {
            // 处理用户消息
        }
        Message::System { .. } => {
            // 处理系统消息
        }
        // 编译器确保所有分支都处理
    }
}
```

---

## 实践建议

### 1. 使用 const 定义工具集

```rust
const READONLY_TOOLS: &[&str] = &["Read", "Grep", "Glob"];
const EDITOR_TOOLS: &[&str] = &["Read", "Edit", "Write"];

fn create_readonly_agent(
    name: &str,
    description: &str,
    prompt: &str
) -> (String, AgentDefinition) {
    (
        name.to_string(),
        AgentDefinition {
            description: description.to_string(),
            prompt: prompt.to_string(),
            tools: Some(READONLY_TOOLS.iter().map(|s| s.to_string()).collect()),
            model: Some("sonnet".to_string()),
        }
    )
}
```

### 2. 使用 Builder Pattern

```rust
// ✅ 清晰、类型安全
let options = ClaudeCodeOptions::builder()
    .agents(agents)
    .permission_mode(PermissionMode::AcceptEdits)
    .model("sonnet")
    .max_turns(Some(10))
    .build();
```

### 3. 优雅的错误处理

```rust
// ✅ 使用 ? 操作符
async fn process() -> Result<(), Box<dyn std::error::Error>> {
    let mut messages = query("...", Some(options)).await?;

    while let Some(msg) = messages.next().await {
        match msg? {  // ? 传播错误
            Message::Assistant { message } => { /* ... */ }
            _ => {}
        }
    }

    Ok(())
}
```

### 4. 使用类型别名

```rust
use std::collections::HashMap;

type AgentMap = HashMap<String, AgentDefinition>;

fn load_agents() -> AgentMap {
    let mut agents = AgentMap::new();
    // ...
    agents
}
```

---

## 未知部分

以下是基于文档和代码推测，但未完全确认的细节：

### 1. 真正的并行机制
- 是 API 级并发还是 CLI 进程级并发？
- 最大并发数限制？

### 2. 上下文隔离的底层实现
- 独立上下文如何实现？
- 新的 API 调用还是上下文切换？

### 3. 成本计算
- Subagent 的 token 消耗如何计费？
- 是否与主会话分开计算？

### 4. 错误处理
- Subagent 失败时如何恢复？
- 是否重试？如何报告？

---

## 总结

**Subagents 的本质**：

> Subagents 是一种智能的"专家咨询"机制，通过独立上下文、工具限制和专业提示词，让 Claude 能够高效处理复杂任务，同时保持主会话的清晰和专注。

**Rust SDK 的优势**：

1. **类型安全**：编译时捕获错误
2. **零成本抽象**：高性能
3. **所有权系统**：防止数据竞争
4. **强制错误处理**：更可靠的代码

**关键理解**：

1. 不是多个 AI 实例，而是一个 AI 的多种专家模式
2. 独立上下文是为了资源隔离和主会话清晰
3. 工具限制是硬性执行的，不可绕过
4. 并行执行需要明确指示
5. Rust 的类型系统提供额外的安全保障

**与文档配合使用**：

- 本文档（INTERNALS）：理解工作原理
- BEST_PRACTICES 文档：学习如何使用

掌握这两方面，你将能充分发挥 subagents 的威力！

---

**最后更新**: 2025-10-10
**版本**: 1.0
**适用**: Rust SDK (cc-sdk v0.2.0+)
