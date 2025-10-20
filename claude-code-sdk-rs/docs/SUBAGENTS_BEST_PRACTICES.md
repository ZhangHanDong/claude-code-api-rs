# Subagents 最佳实践指南 (Rust SDK)

> 在 Rust Claude Code SDK 中使用 subagents 的完整指南

## 目录

- [核心策略](#核心策略)
- [定义方式对比](#定义方式对比)
- [最佳实践](#最佳实践)
- [高级技巧](#高级技巧)
- [完整示例](#完整示例)
- [常见陷阱](#常见陷阱)
- [快速决策树](#快速决策树)

---

## 核心策略

### 选择合适的定义方式

Rust SDK 提供了三种方式定义 subagents，各有优劣：

| 方式 | 优点 | 缺点 | 适用场景 |
|------|------|------|----------|
| **程序化定义** (`agents()`) | ✅ 动态生成<br>✅ 无文件依赖<br>✅ 版本控制在代码中<br>✅ 类型安全 | ❌ 不能跨项目共享<br>❌ 不能在 CLI 中直接使用 | CI/CD、测试、会话特定 |
| **文件系统定义** (`.claude/agents/`) | ✅ 团队共享<br>✅ CLI 可用<br>✅ 可复用 | ❌ 需要 `setting_sources`<br>❌ 文件依赖 | 团队协作、跨项目复用 |
| **混合使用** | ✅ 灵活性最高 | ❌ 管理复杂度 | 大型项目 |

---

## 最佳实践

### 实践 1: 程序化定义 - 推荐用于 SDK

```rust
use cc_sdk::{ClaudeCodeOptions, AgentDefinition, PermissionMode, query};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut agents = HashMap::new();

    // ✅ 最佳实践：单一职责，详细 prompt，限制工具

    // 代码审查专家
    agents.insert(
        "code-reviewer".to_string(),
        AgentDefinition {
            description: "Expert code review specialist. \
                         Use PROACTIVELY after code changes. \
                         Focuses on security, performance, and best practices.".to_string(),
            prompt: r#"You are a senior code reviewer with 10+ years of experience.

WHEN INVOKED:
1. Run `git diff` to see recent changes
2. Focus ONLY on modified files
3. Begin review immediately - no preamble

REVIEW CHECKLIST (priority order):
🔴 CRITICAL (must fix):
   - Security vulnerabilities (SQL injection, XSS, etc.)
   - Exposed secrets/API keys
   - Data races/concurrency issues

⚠️  WARNINGS (should fix):
   - Poor error handling
   - Missing input validation
   - Performance bottlenecks

💡 SUGGESTIONS (consider):
   - Code readability improvements
   - Better naming conventions
   - Refactoring opportunities

OUTPUT FORMAT:
For each issue, provide:
- Location (file:line)
- Severity (Critical/Warning/Suggestion)
- Issue description
- Code example of the fix
- Rationale

Always be constructive and specific."#.to_string(),
            tools: Some(vec![
                "Read".to_string(),
                "Grep".to_string(),
                "Glob".to_string(),
                "Bash".to_string(),
            ]),
            model: Some("sonnet".to_string()),
        }
    );

    // 测试自动化专家
    agents.insert(
        "test-runner".to_string(),
        AgentDefinition {
            description: "Test automation expert. \
                         Use PROACTIVELY when code changes detected. \
                         Runs tests and fixes failures automatically.".to_string(),
            prompt: r#"You are a test automation specialist.

WORKFLOW:
1. Detect changed files (git diff)
2. Identify affected test files
3. Run relevant tests: cargo test <test_name> --verbose
4. If failures:
   a. Analyze error messages
   b. Check if bug in code or test
   c. Fix the root cause
   d. Re-run to verify
5. Report results concisely

RULES:
- Preserve original test intent
- Don't skip failing tests
- Add new tests for uncovered cases
- Use #[test] for unit tests
- Use #[tokio::test] for async tests"#.to_string(),
            tools: Some(vec![
                "Read".to_string(),
                "Edit".to_string(),
                "Bash".to_string(),
                "Grep".to_string(),
            ]),
            model: Some("sonnet".to_string()),
        }
    );

    // 调试专家
    agents.insert(
        "debugger".to_string(),
        AgentDefinition {
            description: "Debugging specialist for errors and failures. \
                         Use when encountering exceptions, test failures, or unexpected behavior.".to_string(),
            prompt: r#"You are a debugging expert specializing in root cause analysis.

DEBUGGING PROTOCOL:
1. Capture full error (message + stack trace)
2. Identify reproduction steps
3. Form hypothesis
4. Test hypothesis with strategic logging (tracing/log crate)
5. Implement minimal fix
6. Verify solution

TECHNIQUES:
- Binary search for fault localization
- Debug printing with context (dbg! macro)
- Check recent changes (git log -p)
- Inspect variable states at failure point
- Use rust-analyzer for type checking

OUTPUT:
- Root cause (not symptoms)
- Evidence supporting diagnosis
- Minimal code fix
- How to prevent recurrence"#.to_string(),
            tools: Some(vec![
                "Read".to_string(),
                "Edit".to_string(),
                "Bash".to_string(),
                "Grep".to_string(),
                "Glob".to_string(),
            ]),
            model: Some("opus".to_string()), // 复杂调试用更强模型
        }
    );

    // 使用 builder pattern 创建选项
    let options = ClaudeCodeOptions::builder()
        .agents(agents)
        .permission_mode(PermissionMode::AcceptEdits)
        .model("sonnet")
        .build();

    // 显式调用特定 agent
    let mut messages = query(
        "Use the code-reviewer agent to review recent changes",
        Some(options)
    ).await?;

    // 处理响应
    while let Some(msg) = messages.next().await {
        println!("{:?}", msg?);
    }

    Ok(())
}
```

---

### 实践 2: 文件系统定义 + SDK 加载

#### 步骤 1: 创建 `.claude/agents/code-reviewer.md`

```markdown
---
name: code-reviewer
description: Expert code review specialist. Use PROACTIVELY after code changes.
tools: Read, Grep, Glob, Bash
model: sonnet
---

You are a senior code reviewer...

[Same prompt as above]
```

#### 步骤 2: SDK 中加载文件系统 agents

```rust
use cc_sdk::{ClaudeCodeOptions, SettingSource};

let options = ClaudeCodeOptions::builder()
    .setting_sources(vec![
        SettingSource::User,
        SettingSource::Project,
        SettingSource::Local,
    ])
    .cwd("/path/to/project")
    .build();
```

**优先级**（高到低）:
1. 程序化定义 (`agents()`)
2. Project agents (`.claude/agents/`)
3. User agents (`~/.claude/agents/`)

---

### 实践 3: 混合策略 - 动态 + 静态

```rust
let mut agents = HashMap::new();
agents.insert(
    "experiment-agent".to_string(),
    AgentDefinition {
        description: "Experimental agent for this session only".to_string(),
        prompt: "Try new approaches...".to_string(),
        tools: Some(vec!["Read".to_string()]),
        model: Some("haiku".to_string()), // 实验用便宜模型
    }
);

let options = ClaudeCodeOptions::builder()
    .setting_sources(vec![SettingSource::User, SettingSource::Project])
    .agents(agents)  // 动态 agent 会覆盖同名的文件系统 agent
    .build();
```

---

## 高级技巧

### 技巧 1: 类型安全的 Agent 定义

```rust
// ✅ 最佳实践：使用常量定义工具列表
const READONLY_TOOLS: &[&str] = &["Read", "Grep", "Glob"];
const EDITOR_TOOLS: &[&str] = &["Read", "Edit", "Write"];
const FULL_TOOLS: &[&str] = &["Read", "Edit", "Write", "Bash", "Grep", "Glob"];

fn create_readonly_agent(name: &str, description: &str, prompt: &str) -> (String, AgentDefinition) {
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

// 使用
let mut agents = HashMap::new();
agents.insert(
    create_readonly_agent(
        "analyzer",
        "Code structure analyzer",
        "Analyze code structure and patterns..."
    )
);
```

### 技巧 2: Agent 工厂函数

```rust
use cc_sdk::AgentDefinition;

struct AgentBuilder {
    name: String,
    description: String,
    prompt: String,
    tools: Vec<String>,
    model: String,
}

impl AgentBuilder {
    fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
            prompt: String::new(),
            tools: Vec::new(),
            model: "sonnet".to_string(),
        }
    }

    fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    fn prompt(mut self, prompt: impl Into<String>) -> Self {
        self.prompt = prompt.into();
        self
    }

    fn readonly_tools(mut self) -> Self {
        self.tools = vec!["Read".into(), "Grep".into(), "Glob".into()];
        self
    }

    fn editor_tools(mut self) -> Self {
        self.tools = vec!["Read".into(), "Edit".into(), "Write".into()];
        self
    }

    fn model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    fn build(self) -> (String, AgentDefinition) {
        (
            self.name.clone(),
            AgentDefinition {
                description: self.description,
                prompt: self.prompt,
                tools: Some(self.tools),
                model: Some(self.model),
            }
        )
    }
}

// 使用示例
let mut agents = HashMap::new();
let (name, agent) = AgentBuilder::new("code-reviewer")
    .description("Expert code reviewer")
    .prompt("You are a senior code reviewer...")
    .readonly_tools()
    .model("opus")
    .build();
agents.insert(name, agent);
```

### 技巧 3: 模型选择策略

```rust
enum TaskComplexity {
    Simple,
    Medium,
    Complex,
}

impl TaskComplexity {
    fn recommended_model(&self) -> &str {
        match self {
            TaskComplexity::Simple => "haiku",
            TaskComplexity::Medium => "sonnet",
            TaskComplexity::Complex => "opus",
        }
    }
}

// 使用
let debugger_agent = AgentDefinition {
    model: Some(TaskComplexity::Complex.recommended_model().to_string()),
    // ...
};
```

### 技巧 4: 从配置文件加载 Agents

```rust
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Deserialize, Serialize)]
struct AgentConfig {
    agents: HashMap<String, AgentDefinition>,
}

fn load_agents_from_file(path: &str) -> Result<HashMap<String, AgentDefinition>, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path)?;
    let config: AgentConfig = serde_json::from_str(&content)?;
    Ok(config.agents)
}

// 使用
let agents = load_agents_from_file("agents_config.json")?;
let options = ClaudeCodeOptions::builder()
    .agents(agents)
    .build();
```

---

## 完整示例：多 Agent 协作系统

```rust
use cc_sdk::{
    ClaudeCodeOptions, AgentDefinition, PermissionMode,
    InteractiveClient, Message,
};
use std::collections::HashMap;
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 定义专业 agent 团队
    let mut agents = HashMap::new();

    // 1. 分析师 - 只读
    agents.insert(
        "analyzer".to_string(),
        AgentDefinition {
            description: "Code structure analyzer. Use for understanding codebase architecture.".to_string(),
            prompt: r#"Analyze code structure and patterns.

Focus on:
- File organization
- Module dependencies
- Design patterns used
- Potential refactoring opportunities

Output: Structured analysis with recommendations."#.to_string(),
            tools: Some(vec!["Read".into(), "Grep".into(), "Glob".into()]),
            model: Some("sonnet".to_string()),
        }
    );

    // 2. 实施者 - 读写
    agents.insert(
        "implementer".to_string(),
        AgentDefinition {
            description: "Implementation specialist. Use for writing new code or features.".to_string(),
            prompt: r#"Implement features based on specifications.

Follow:
- Clean code principles
- SOLID principles
- Existing project conventions
- Write tests alongside code
- Use idiomatic Rust (Result, Option, iterators)

Always ask for clarification if specs are unclear."#.to_string(),
            tools: Some(vec!["Read".into(), "Write".into(), "Edit".into(), "Bash".into()]),
            model: Some("sonnet".to_string()),
        }
    );

    // 3. 测试员 - 读写执行
    agents.insert(
        "tester".to_string(),
        AgentDefinition {
            description: "Test specialist. Use PROACTIVELY after code changes.".to_string(),
            prompt: r#"Write and run comprehensive tests.

Coverage:
- Unit tests (cargo test)
- Integration tests (tests/)
- Edge cases
- Error conditions

Aim for >80% coverage."#.to_string(),
            tools: Some(vec!["Read".into(), "Write".into(), "Bash".into()]),
            model: Some("sonnet".to_string()),
        }
    );

    // 4. 审查员 - 只读
    agents.insert(
        "reviewer".to_string(),
        AgentDefinition {
            description: "Code reviewer. Use before committing changes.".to_string(),
            prompt: r#"Review Rust code for quality and security.

Rust-specific checks:
- Lifetime management
- Ownership & borrowing
- Error handling (Result, Option)
- Unsafe code justification
- Performance (allocations, clones)
- API design (builder pattern, traits)

General checks:
- Security vulnerabilities
- Code readability
- Test coverage"#.to_string(),
            tools: Some(vec!["Read".into(), "Grep".into(), "Glob".into(), "Bash".into()]),
            model: Some("opus".to_string()),
        }
    );

    let options = ClaudeCodeOptions::builder()
        .agents(agents)
        .permission_mode(PermissionMode::AcceptEdits)
        .max_turns(Some(20))
        .build();

    let mut client = InteractiveClient::new(options)?;
    client.connect().await?;

    // 工作流：分析 -> 实施 -> 测试 -> 审查
    client.send_message(r#"
Please help me add a new authentication feature:

1. Use the analyzer agent to understand current auth structure
2. Use the implementer agent to add JWT token support
3. Use the tester agent to write comprehensive tests
4. Use the reviewer agent to review all changes

Work through each step systematically.
    "#.to_string()).await?;

    // 接收并打印响应
    let mut stream = client.receive_response_stream().await;
    while let Some(result) = stream.next().await {
        match result? {
            Message::Assistant { message } => {
                for block in message.content {
                    match block {
                        cc_sdk::ContentBlock::Text(text) => {
                            println!("{}", text.text);
                        }
                        _ => {}
                    }
                }
            }
            Message::Result { .. } => break,
            _ => {}
        }
    }

    client.disconnect().await?;
    Ok(())
}
```

---

## 常见陷阱与解决方案

### 陷阱 1: Description 不够具体

```rust
// ❌ 不好：太泛化
AgentDefinition {
    description: "Code helper".to_string(),
    // ...
}

// ✅ 好：明确触发条件
AgentDefinition {
    description: "Use PROACTIVELY after git commits to review changes".to_string(),
    // ...
}
```

### 陷阱 2: 忘记处理 Result

```rust
// ❌ 不好：unwrap 可能 panic
let agents = load_agents_from_file("config.json").unwrap();

// ✅ 好：优雅的错误处理
let agents = match load_agents_from_file("config.json") {
    Ok(a) => a,
    Err(e) => {
        eprintln!("Failed to load agents: {}", e);
        HashMap::new() // 使用默认值
    }
};
```

### 陷阱 3: 工具权限过大

```rust
// ❌ 危险：授予 Bash 但无需求
AgentDefinition {
    description: "Code formatter".to_string(),
    tools: Some(vec!["Read".into(), "Edit".into(), "Bash".into()]),
    // ...
}

// ✅ 安全：最小权限
AgentDefinition {
    description: "Code formatter".to_string(),
    tools: Some(vec!["Read".into(), "Edit".into()]),
    // ...
}
```

---

## 快速决策树

```
需要定义 subagent？
│
├─ 仅用于当前会话/脚本？
│  └─ 使用程序化定义 (.agents())
│
├─ 需要团队共享？
│  └─ 使用文件系统 (.claude/agents/) + setting_sources
│
├─ 需要临时覆盖团队 agent？
│  └─ 使用混合策略
│
└─ 快速测试原型？
   └─ 使用程序化定义，成功后移到文件系统
```

---

## 黄金法则

**在 Rust SDK 中使用 subagents 的核心原则**：

1. ✅ **单一职责**：每个 agent 只做一件事
2. ✅ **详细 prompt**：包含 WHEN/WHAT/HOW/OUTPUT
3. ✅ **最小权限**：只授予必要工具
4. ✅ **明确 description**：使用 "PROACTIVELY" 等关键词
5. ✅ **适当模型**：根据任务复杂度选择
6. ✅ **类型安全**：利用 Rust 的类型系统
7. ✅ **错误处理**：使用 Result 而非 panic
8. ✅ **显式调用**：重要任务明确指定 agent

**Rust SDK 特有优势**：
- 编译时类型检查
- 零成本抽象
- 并发安全
- 高性能

---

## Rust 特定最佳实践

### 1. 使用 const 定义工具集

```rust
const READONLY_TOOLS: &[&str] = &["Read", "Grep", "Glob"];
const FULL_TOOLS: &[&str] = &["Read", "Edit", "Write", "Bash", "Grep", "Glob"];

fn to_string_vec(tools: &[&str]) -> Vec<String> {
    tools.iter().map(|s| s.to_string()).collect()
}
```

### 2. 使用 Builder Pattern

```rust
let options = ClaudeCodeOptions::builder()
    .agents(agents)
    .permission_mode(PermissionMode::AcceptEdits)
    .model("sonnet")
    .max_turns(Some(10))
    .build();
```

### 3. 使用 macro 减少重复

```rust
macro_rules! agent {
    ($name:expr, $desc:expr, $prompt:expr, readonly) => {
        ($name.to_string(), AgentDefinition {
            description: $desc.to_string(),
            prompt: $prompt.to_string(),
            tools: Some(vec!["Read".into(), "Grep".into(), "Glob".into()]),
            model: Some("sonnet".to_string()),
        })
    };
    ($name:expr, $desc:expr, $prompt:expr, editor) => {
        ($name.to_string(), AgentDefinition {
            description: $desc.to_string(),
            prompt: $prompt.to_string(),
            tools: Some(vec!["Read".into(), "Edit".into(), "Write".into()]),
            model: Some("sonnet".to_string()),
        })
    };
}

// 使用
let mut agents = HashMap::new();
agents.insert(agent!("analyzer", "Code analyzer", "Analyze code...", readonly));
agents.insert(agent!("formatter", "Code formatter", "Format code...", editor));
```

---

## 参考资源

- [Rust Claude Code SDK 文档](https://docs.rs/cc-sdk)
- [Claude Code SDK GitHub](https://github.com/ZhangHanDong/claude-code-api-rs)
- [Claude Code Subagents 官方文档](https://docs.anthropic.com/en/docs/claude-code/subagents)
- [Rust 异步编程](https://rust-lang.github.io/async-book/)

---

**最后更新**: 2025-10-10
**版本**: 1.0
**适用 SDK 版本**: cc-sdk v0.2.0+
