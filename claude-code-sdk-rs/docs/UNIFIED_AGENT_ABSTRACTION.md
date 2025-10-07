# Unified Agent Abstraction Layer Design

## Overview

Design document for a unified abstraction layer that allows the Claude Code SDK to support multiple AI agent backends (Claude Code CLI, Codex, Direct API, etc.) with dynamic provider switching based on task requirements.

**Last Updated**: 2025-10-07
**Status**: Research Phase Complete - Implementation Ready

## Motivation

1. **Multi-Backend Support**: Support both Claude Code CLI and Codex systems
2. **Dynamic Switching**: Route different tasks to different providers based on requirements
3. **Unified Interface**: Single API for all agent interactions
4. **Future-Proof**: Easy to add new providers (Direct API, custom agents, etc.)

## Research Summary

### Key Findings from Codex SDK Investigation

After analyzing the Codex Rust codebase (`/Users/zhangalex/Work/Projects/ai/codex/codex-rs`), the following architectural patterns were identified:

**1. Submission Queue / Event Queue (SQ/EQ) Pattern**
- **Submission Queue**: User submits operations to agent (`Op::UserInput`, `Op::Interrupt`, `Op::Shutdown`, etc.)
- **Event Queue**: Agent emits events to user (`EventMsg::AgentMessage`, `EventMsg::TokenCount`, etc.)
- 25+ event types including streaming deltas, tool calls, approvals, and background tasks
- Fully async bidirectional communication using channels

**2. Conversation Management**
```rust
ConversationManager {
    conversations: HashMap<ConversationId, Arc<CodexConversation>>,
    auth_manager: Arc<AuthManager>,
}
```
- Multi-thread conversation support (unlimited concurrent sessions)
- Persistent rollout storage in `~/.codex/sessions`
- Fork/resume capabilities for conversations

**3. TypeScript SDK Architecture**
```typescript
class Codex {
  startThread(options: ThreadOptions): Thread  // New conversation
  resumeThread(id: string): Thread             // Resume from rollout
}

class Thread {
  async run(input: string): Promise<Turn>                    // Buffered execution
  async runStreamed(input: string): Promise<StreamedTurn>    // Streaming events
}
```
- Wraps CLI binary via subprocess (stdin/stdout)
- JSONL event streaming with `--experimental-json` flag
- Simplified API: `run()` for simple use, `runStreamed()` for advanced control

### Claude Code SDK vs Codex - Critical Differences

| Feature | Claude Code SDK | Codex |
|---------|----------------|-------|
| **Communication** | Control Protocol (JSONL) | SQ/EQ Pattern (async channels) |
| **Session Model** | Single session per client | Multi-thread with persistent rollout |
| **API Style** | `send_request()` → `receive_messages()` | `submit(Op)` → `next_event()` |
| **Streaming** | `Stream<Message>` | `AsyncGenerator<Event>` |
| **State** | ClientState + SessionData | Rollout-based persistence |
| **Interrupts** | Control request | `Op::Interrupt` submission |
| **Token Tracking** | Built-in BudgetManager | `EventMsg::TokenCount` in stream |
| **Persistence** | None (ephemeral) | Full rollout with resume support |
| **Concurrency** | 1 session per client | Unlimited concurrent threads |

## Architecture Analysis

### Codex Architecture (from /Users/zhangalex/Work/Projects/ai/codex)

**Communication Pattern**:
- **Submission Queue (SQ)**: User → Agent requests
- **Event Queue (EQ)**: Agent → User events
- Async bidirectional communication (Actor model)

**Key Components**:
```rust
pub struct Codex {
    tx_sub: Sender<Submission>,    // Submit requests
    rx_event: Receiver<Event>,      // Receive events
}

pub struct Submission {
    id: String,
    op: Op,  // UserInput, UserTurn, Interrupt, etc.
}

pub enum Op {
    UserInput { items: Vec<InputItem> },
    UserTurn { items, cwd, approval_policy, sandbox_policy, model, ... },
    Interrupt,
    ExecApproval { id, decision },
    PatchApproval { id, decision },
    GetPath,
    ListMcpTools,
    Review { review_request },
    Shutdown,
}

pub struct Event {
    msg: EventMsg,
    // AgentMessage, TokenCount, ExecApproval, etc.
}
```

**ModelClient Abstraction**:
```rust
pub struct ModelClient {
    config: Arc<Config>,
    auth_manager: Option<Arc<AuthManager>>,
    provider: ModelProviderInfo,  // Provider abstraction!
}

impl ModelClient {
    pub async fn stream(&self, prompt: &Prompt) -> Result<ResponseStream> {
        match self.provider.wire_api {
            WireApi::Responses => // Anthropic Messages API
            WireApi::Chat => // OpenAI Chat Completions
        }
    }
}
```

### Claude Code SDK Current Architecture

**Communication Pattern**:
- CLI subprocess stdin/stdout
- JSON message streaming (stream-json format)
- Control Protocol for permissions/hooks/MCP

**Key Components**:
```rust
pub struct ClaudeSDKClient {
    transport: Arc<Mutex<SubprocessTransport>>,
    query_handler: Option<Arc<Mutex<Query>>>,
    budget_manager: BudgetManager,
}
```

## Unified Abstraction Design

### Core Trait Hierarchy

```rust
/// Top-level provider abstraction
#[async_trait]
pub trait AgentProvider: Send + Sync {
    /// Provider type identifier
    fn provider_type(&self) -> ProviderType;

    /// Create a new session
    async fn create_session(
        &self,
        config: SessionConfig
    ) -> Result<Box<dyn AgentSession>>;

    /// Get provider capabilities
    fn capabilities(&self) -> ProviderCapabilities;

    /// Check if provider can handle a specific task type
    fn can_handle_task(&self, task: &TaskDescriptor) -> bool {
        task.matches_capabilities(&self.capabilities())
    }

    /// Get estimated cost for a task (for routing decisions)
    async fn estimate_cost(&self, task: &TaskDescriptor) -> Result<CostEstimate>;
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ProviderType {
    ClaudeCode,    // CLI-based Claude Code
    Codex,         // Codex system
    DirectAPI,     // Direct Anthropic API (future)
    Custom(String), // User-defined providers
}

#[derive(Debug, Clone)]
pub struct ProviderCapabilities {
    // Core capabilities
    pub supports_tools: bool,
    pub supports_mcp: bool,
    pub supports_hooks: bool,
    pub supports_interrupts: bool,
    pub supports_streaming: bool,

    // Advanced capabilities
    pub supports_file_editing: bool,
    pub supports_shell_exec: bool,
    pub supports_web_search: bool,
    pub supports_code_review: bool,

    // Model support
    pub available_models: Vec<String>,
    pub supports_reasoning: bool,

    // Limits
    pub max_context_tokens: Option<u64>,
    pub max_output_tokens: Option<u64>,
    pub rate_limit: Option<RateLimit>,
}

/// Unified session interface
#[async_trait]
pub trait AgentSession: Send + Sync {
    /// Send user input
    async fn send_input(&mut self, input: UserInput) -> Result<()>;

    /// Receive agent events as a stream
    fn receive_events(&mut self) -> Pin<Box<dyn Stream<Item = Result<AgentEvent>> + Send + '_>>;

    /// Interrupt current task
    async fn interrupt(&mut self) -> Result<()>;

    /// Get usage statistics
    async fn get_usage(&self) -> UsageStats;

    /// Get session metadata
    fn metadata(&self) -> &SessionMetadata;

    /// Close session
    async fn close(self: Box<Self>) -> Result<()>;
}
```

### Dynamic Provider Routing

```rust
/// Task descriptor for routing decisions
#[derive(Debug, Clone)]
pub struct TaskDescriptor {
    /// Task type
    pub task_type: TaskType,

    /// Required capabilities
    pub required_capabilities: Vec<Capability>,

    /// Preferred model characteristics
    pub model_preference: ModelPreference,

    /// Cost constraints
    pub max_cost: Option<f64>,

    /// Performance requirements
    pub max_latency: Option<Duration>,

    /// Priority level
    pub priority: Priority,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TaskType {
    /// Simple Q&A, no tools needed
    SimpleQuery,

    /// Code generation/editing
    CodeGeneration,

    /// File editing tasks
    FileEditing,

    /// Shell command execution
    ShellExecution,

    /// Code review
    CodeReview,

    /// Complex multi-step tasks
    ComplexWorkflow,

    /// Custom task type
    Custom(String),
}

#[derive(Debug, Clone)]
pub enum ModelPreference {
    /// Fastest response
    Speed,

    /// Best quality
    Quality,

    /// Lowest cost
    Cost,

    /// Balanced
    Balanced,

    /// Specific model
    Specific(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Low,
    Normal,
    High,
    Critical,
}

/// Provider router that selects the best provider for a task
pub struct ProviderRouter {
    providers: Vec<Box<dyn AgentProvider>>,
    routing_strategy: RoutingStrategy,
}

#[derive(Debug, Clone)]
pub enum RoutingStrategy {
    /// Always use the first available provider
    FirstAvailable,

    /// Choose cheapest provider that meets requirements
    CostOptimized,

    /// Choose fastest provider
    LatencyOptimized,

    /// Balance cost and performance
    Balanced,

    /// Custom routing logic
    Custom(Arc<dyn Fn(&TaskDescriptor, &[&dyn AgentProvider]) -> Option<usize> + Send + Sync>),
}

impl ProviderRouter {
    pub fn new(providers: Vec<Box<dyn AgentProvider>>) -> Self {
        Self {
            providers,
            routing_strategy: RoutingStrategy::Balanced,
        }
    }

    pub fn with_strategy(mut self, strategy: RoutingStrategy) -> Self {
        self.routing_strategy = strategy;
        self
    }

    /// Select best provider for a task
    pub async fn select_provider(&self, task: &TaskDescriptor) -> Result<&dyn AgentProvider> {
        match &self.routing_strategy {
            RoutingStrategy::FirstAvailable => {
                self.select_first_capable(task)
            }
            RoutingStrategy::CostOptimized => {
                self.select_cheapest(task).await
            }
            RoutingStrategy::LatencyOptimized => {
                self.select_fastest(task).await
            }
            RoutingStrategy::Balanced => {
                self.select_balanced(task).await
            }
            RoutingStrategy::Custom(f) => {
                let providers_ref: Vec<&dyn AgentProvider> =
                    self.providers.iter().map(|p| p.as_ref()).collect();

                let idx = f(task, &providers_ref)
                    .ok_or_else(|| SdkError::NoProviderAvailable {
                        task_type: format!("{:?}", task.task_type),
                    })?;

                Ok(self.providers[idx].as_ref())
            }
        }
    }

    /// Create session with automatic provider selection
    pub async fn create_session_for_task(
        &self,
        task: TaskDescriptor,
        config: SessionConfig,
    ) -> Result<(Box<dyn AgentSession>, ProviderType)> {
        let provider = self.select_provider(&task).await?;
        let provider_type = provider.provider_type();
        let session = provider.create_session(config).await?;

        Ok((session, provider_type))
    }

    // Selection strategies

    fn select_first_capable(&self, task: &TaskDescriptor) -> Result<&dyn AgentProvider> {
        self.providers
            .iter()
            .find(|p| p.can_handle_task(task))
            .map(|p| p.as_ref())
            .ok_or_else(|| SdkError::NoProviderAvailable {
                task_type: format!("{:?}", task.task_type),
            })
    }

    async fn select_cheapest(&self, task: &TaskDescriptor) -> Result<&dyn AgentProvider> {
        let mut candidates = Vec::new();

        for provider in &self.providers {
            if provider.can_handle_task(task) {
                let cost = provider.estimate_cost(task).await?;
                candidates.push((provider.as_ref(), cost.total_usd));
            }
        }

        candidates.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        candidates
            .first()
            .map(|(p, _)| *p)
            .ok_or_else(|| SdkError::NoProviderAvailable {
                task_type: format!("{:?}", task.task_type),
            })
    }

    async fn select_fastest(&self, task: &TaskDescriptor) -> Result<&dyn AgentProvider> {
        // For now, use simple heuristic: Codex is typically faster for complex tasks,
        // Claude Code CLI is faster for simple queries

        let is_simple = matches!(task.task_type, TaskType::SimpleQuery);

        self.providers
            .iter()
            .find(|p| {
                p.can_handle_task(task) &&
                if is_simple {
                    p.provider_type() == ProviderType::ClaudeCode
                } else {
                    p.provider_type() == ProviderType::Codex
                }
            })
            .or_else(|| {
                self.providers.iter().find(|p| p.can_handle_task(task))
            })
            .map(|p| p.as_ref())
            .ok_or_else(|| SdkError::NoProviderAvailable {
                task_type: format!("{:?}", task.task_type),
            })
    }

    async fn select_balanced(&self, task: &TaskDescriptor) -> Result<&dyn AgentProvider> {
        // Score each provider based on cost, capabilities, and expected performance
        let mut scored_providers = Vec::new();

        for provider in &self.providers {
            if !provider.can_handle_task(task) {
                continue;
            }

            let cost_est = provider.estimate_cost(task).await?;
            let caps = provider.capabilities();

            // Simple scoring: lower is better
            let mut score = cost_est.total_usd * 100.0; // Cost weight

            // Penalize if missing preferred capabilities
            if task.task_type == TaskType::CodeReview && !caps.supports_code_review {
                score += 10.0;
            }

            // Bonus for exact model match
            if let ModelPreference::Specific(ref model) = task.model_preference {
                if caps.available_models.contains(model) {
                    score -= 5.0;
                }
            }

            scored_providers.push((provider.as_ref(), score));
        }

        scored_providers.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        scored_providers
            .first()
            .map(|(p, _)| *p)
            .ok_or_else(|| SdkError::NoProviderAvailable {
                task_type: format!("{:?}", task.task_type),
            })
    }
}

#[derive(Debug, Clone)]
pub struct CostEstimate {
    pub total_usd: f64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub confidence: f64, // 0.0-1.0
}
```

### Unified Message Types

```rust
/// Unified user input
#[derive(Debug, Clone)]
pub enum UserInput {
    /// Simple text input
    Text(String),

    /// Text with additional context
    TextWithContext {
        text: String,
        context: Vec<ContextItem>,
    },

    /// Tool/command approval
    ToolApproval {
        request_id: String,
        approved: bool,
        modified_input: Option<serde_json::Value>,
        reason: Option<String>,
    },

    /// Patch approval
    PatchApproval {
        request_id: String,
        decision: ReviewDecision,
    },

    /// Request conversation history
    GetHistory,

    /// Request MCP tools list
    ListMcpTools,
}

#[derive(Debug, Clone)]
pub enum ReviewDecision {
    Accept,
    Reject,
    Modify { changes: String },
}

/// Context items attached to input
#[derive(Debug, Clone)]
pub enum ContextItem {
    File {
        path: PathBuf,
        content: String,
        language: Option<String>,
    },
    GitDiff {
        content: String,
    },
    Environment {
        key: String,
        value: String,
    },
    UserInstructions {
        text: String,
    },
}

/// Unified agent event
#[derive(Debug, Clone)]
pub enum AgentEvent {
    // Text output
    TextDelta {
        text: String,
    },
    TextComplete {
        text: String,
    },

    // Thinking process
    ThinkingDelta {
        content: String,
    },
    ThinkingComplete,

    // Tool calls
    ToolUse {
        tool_name: String,
        tool_id: String,
        input: serde_json::Value,
    },
    ToolResult {
        tool_id: String,
        output: String,
        is_error: bool,
    },

    // Permission/approval requests
    PermissionRequest {
        request_id: String,
        tool_name: String,
        input: serde_json::Value,
        suggestions: Vec<PermissionSuggestion>,
    },

    ExecApprovalRequest {
        request_id: String,
        command: String,
        cwd: PathBuf,
        parsed_command: Option<String>,
    },

    PatchApprovalRequest {
        request_id: String,
        patch_content: String,
        files_affected: Vec<PathBuf>,
    },

    // Token usage
    UsageUpdate {
        input_tokens: u64,
        output_tokens: u64,
        cost_usd: f64,
    },

    // Session events
    SessionStarted {
        session_id: String,
        provider: ProviderType,
    },

    TurnComplete {
        stop_reason: StopReason,
    },

    // Background events
    BackgroundTask {
        task_type: String,
        message: String,
    },

    // Errors
    Error {
        message: String,
        error_type: ErrorType,
        recoverable: bool,
    },
}

#[derive(Debug, Clone)]
pub struct PermissionSuggestion {
    pub suggestion_type: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StopReason {
    EndTurn,
    MaxTokens,
    StopSequence,
    ToolUse,
    UserInterrupt,
    Error,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ErrorType {
    RateLimit,
    Authentication,
    InvalidRequest,
    ModelOverloaded,
    NetworkError,
    InternalError,
    UsageLimitReached,
}

/// Session metadata
#[derive(Debug, Clone)]
pub struct SessionMetadata {
    pub session_id: String,
    pub provider_type: ProviderType,
    pub created_at: std::time::SystemTime,
    pub model: String,
    pub capabilities: ProviderCapabilities,
}

/// Usage statistics
#[derive(Debug, Clone, Default)]
pub struct UsageStats {
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_cost_usd: f64,
    pub turns_completed: usize,
}
```

### Provider Implementations

#### ClaudeCodeProvider

```rust
pub struct ClaudeCodeProvider {
    options: ClaudeCodeOptions,
}

impl ClaudeCodeProvider {
    pub fn new(options: ClaudeCodeOptions) -> Self {
        Self { options }
    }
}

#[async_trait]
impl AgentProvider for ClaudeCodeProvider {
    fn provider_type(&self) -> ProviderType {
        ProviderType::ClaudeCode
    }

    async fn create_session(&self, config: SessionConfig) -> Result<Box<dyn AgentSession>> {
        let mut client = ClaudeSDKClient::new(self.options.clone());

        if let Some(prompt) = config.initial_prompt {
            client.connect(Some(prompt)).await?;
        } else {
            client.connect(None).await?;
        }

        let metadata = SessionMetadata {
            session_id: uuid::Uuid::new_v4().to_string(),
            provider_type: ProviderType::ClaudeCode,
            created_at: std::time::SystemTime::now(),
            model: self.options.model.clone().unwrap_or_else(|| "sonnet".to_string()),
            capabilities: self.capabilities(),
        };

        Ok(Box::new(ClaudeCodeSession {
            client,
            config,
            metadata,
        }))
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            supports_tools: true,
            supports_mcp: true,
            supports_hooks: true,
            supports_interrupts: true,
            supports_streaming: true,
            supports_file_editing: true,
            supports_shell_exec: true,
            supports_web_search: true,
            supports_code_review: false, // CLI doesn't have dedicated review mode
            available_models: vec![
                "claude-3-5-haiku-20241022".to_string(),
                "sonnet".to_string(),
                "opus".to_string(),
            ],
            supports_reasoning: true,
            max_context_tokens: Some(200_000),
            max_output_tokens: Some(8192),
            rate_limit: None, // Managed by Anthropic
        }
    }

    fn can_handle_task(&self, task: &TaskDescriptor) -> bool {
        let caps = self.capabilities();

        // Check required capabilities
        for required in &task.required_capabilities {
            match required {
                Capability::Tools if !caps.supports_tools => return false,
                Capability::MCP if !caps.supports_mcp => return false,
                Capability::Hooks if !caps.supports_hooks => return false,
                Capability::CodeReview if !caps.supports_code_review => return false,
                _ => {}
            }
        }

        // Check model availability
        if let ModelPreference::Specific(ref model) = task.model_preference {
            if !caps.available_models.iter().any(|m| m == model) {
                return false;
            }
        }

        true
    }

    async fn estimate_cost(&self, task: &TaskDescriptor) -> Result<CostEstimate> {
        use crate::model_recommendation::estimate_cost_multiplier;

        let model = match &task.model_preference {
            ModelPreference::Specific(m) => m.clone(),
            ModelPreference::Speed | ModelPreference::Cost => "claude-3-5-haiku-20241022".to_string(),
            ModelPreference::Quality => "opus".to_string(),
            ModelPreference::Balanced => "sonnet".to_string(),
        };

        let multiplier = estimate_cost_multiplier(&model);

        // Rough estimates based on task type
        let (est_input, est_output) = match task.task_type {
            TaskType::SimpleQuery => (500, 200),
            TaskType::CodeGeneration => (2000, 1000),
            TaskType::FileEditing => (3000, 1500),
            TaskType::ShellExecution => (1000, 500),
            TaskType::CodeReview => (5000, 2000),
            TaskType::ComplexWorkflow => (10000, 5000),
            TaskType::Custom(_) => (2000, 1000),
        };

        // Haiku baseline: ~$0.001 per 1K tokens
        let base_cost_per_1k = 0.001;
        let total_cost = ((est_input + est_output) as f64 / 1000.0) * base_cost_per_1k * multiplier;

        Ok(CostEstimate {
            total_usd: total_cost,
            input_tokens: est_input,
            output_tokens: est_output,
            confidence: 0.6, // Medium confidence for estimates
        })
    }
}

struct ClaudeCodeSession {
    client: ClaudeSDKClient,
    config: SessionConfig,
    metadata: SessionMetadata,
}

#[async_trait]
impl AgentSession for ClaudeCodeSession {
    async fn send_input(&mut self, input: UserInput) -> Result<()> {
        match input {
            UserInput::Text(text) => {
                self.client.send_request(text, None).await?;
            }
            UserInput::TextWithContext { text, context } => {
                // Format context items as part of the message
                let mut full_message = String::new();

                for item in context {
                    match item {
                        ContextItem::File { path, content, .. } => {
                            full_message.push_str(&format!("\n<file path=\"{:?}\">\n{}\n</file>\n", path, content));
                        }
                        ContextItem::GitDiff { content } => {
                            full_message.push_str(&format!("\n<git_diff>\n{}\n</git_diff>\n", content));
                        }
                        ContextItem::UserInstructions { text } => {
                            full_message.push_str(&format!("\n<user_instructions>\n{}\n</user_instructions>\n", text));
                        }
                        _ => {}
                    }
                }

                full_message.push_str(&text);
                self.client.send_request(full_message, None).await?;
            }
            UserInput::ToolApproval { request_id, approved, modified_input, .. } => {
                // Send approval via control protocol
                // This would require extending ClaudeSDKClient with approval methods
                // For now, placeholder:
                warn!("Tool approval not yet implemented for Claude Code provider");
            }
            _ => {
                warn!("Unsupported input type for Claude Code provider: {:?}", input);
            }
        }
        Ok(())
    }

    fn receive_events(&mut self) -> Pin<Box<dyn Stream<Item = Result<AgentEvent>> + Send + '_>> {
        let messages = self.client.receive_messages();

        Box::pin(messages.map(|msg_result| {
            msg_result.and_then(|msg| convert_claude_message_to_event(msg))
        }))
    }

    async fn interrupt(&mut self) -> Result<()> {
        self.client.interrupt().await
    }

    async fn get_usage(&self) -> UsageStats {
        let tracker = self.client.get_usage_stats().await;
        UsageStats {
            total_input_tokens: tracker.total_input_tokens,
            total_output_tokens: tracker.total_output_tokens,
            total_cost_usd: tracker.total_cost_usd,
            turns_completed: tracker.session_count,
        }
    }

    fn metadata(&self) -> &SessionMetadata {
        &self.metadata
    }

    async fn close(mut self: Box<Self>) -> Result<()> {
        self.client.disconnect().await
    }
}
```

#### CodexProvider

```rust
pub struct CodexProvider {
    config: codex_core::config::Config,
    auth_manager: Arc<codex_core::AuthManager>,
}

impl CodexProvider {
    pub fn new(
        config: codex_core::config::Config,
        auth_manager: Arc<codex_core::AuthManager>,
    ) -> Self {
        Self { config, auth_manager }
    }
}

#[async_trait]
impl AgentProvider for CodexProvider {
    fn provider_type(&self) -> ProviderType {
        ProviderType::Codex
    }

    async fn create_session(&self, config: SessionConfig) -> Result<Box<dyn AgentSession>> {
        use codex_core::{Codex, protocol::InitialHistory, protocol::SessionSource};

        let initial_history = if let Some(prompt) = config.initial_prompt {
            // Convert prompt to InitialHistory
            InitialHistory::default() // Placeholder
        } else {
            InitialHistory::default()
        };

        let codex_result = Codex::spawn(
            Arc::new(self.config.clone()),
            self.auth_manager.clone(),
            initial_history,
            SessionSource::Interactive,
        ).await.map_err(|e| SdkError::ProviderError {
            provider: "Codex".to_string(),
            message: e.to_string(),
        })?;

        let metadata = SessionMetadata {
            session_id: codex_result.conversation_id.to_string(),
            provider_type: ProviderType::Codex,
            created_at: std::time::SystemTime::now(),
            model: self.config.model_family.to_string(),
            capabilities: self.capabilities(),
        };

        Ok(Box::new(CodexSession {
            codex: codex_result.codex,
            conversation_id: codex_result.conversation_id,
            metadata,
        }))
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            supports_tools: true,
            supports_mcp: true,
            supports_hooks: false, // Codex uses different approval mechanism
            supports_interrupts: true,
            supports_streaming: true,
            supports_file_editing: true,
            supports_shell_exec: true,
            supports_web_search: true,
            supports_code_review: true, // Codex has dedicated review mode
            available_models: vec![
                "claude-3-5-haiku-20241022".to_string(),
                "sonnet".to_string(),
                "opus".to_string(),
            ],
            supports_reasoning: true,
            max_context_tokens: Some(200_000),
            max_output_tokens: Some(8192),
            rate_limit: None,
        }
    }

    async fn estimate_cost(&self, task: &TaskDescriptor) -> Result<CostEstimate> {
        // Similar to ClaudeCodeProvider but may have different cost structure
        // if using Codex-specific billing

        // For now, use same estimation as Claude Code
        ClaudeCodeProvider::new(ClaudeCodeOptions::default())
            .estimate_cost(task)
            .await
    }
}

struct CodexSession {
    codex: codex_core::Codex,
    conversation_id: codex_protocol::ConversationId,
    metadata: SessionMetadata,
}

#[async_trait]
impl AgentSession for CodexSession {
    async fn send_input(&mut self, input: UserInput) -> Result<()> {
        use codex_core::protocol::{Submission, Op, InputItem};

        let submission_id = format!("sub_{}", uuid::Uuid::new_v4());

        let op = match input {
            UserInput::Text(text) => {
                Op::UserInput {
                    items: vec![InputItem::Text { text }],
                }
            }
            UserInput::TextWithContext { text, context } => {
                let mut items = Vec::new();

                // Convert context items to Codex InputItems
                for ctx in context {
                    match ctx {
                        ContextItem::UserInstructions { text } => {
                            items.push(InputItem::UserInstructions { text });
                        }
                        _ => {
                            // Other context types need conversion
                        }
                    }
                }

                items.push(InputItem::Text { text });

                Op::UserInput { items }
            }
            UserInput::ToolApproval { request_id, approved, .. } => {
                Op::ExecApproval {
                    id: request_id,
                    decision: if approved {
                        codex_core::protocol::ReviewDecision::Accept
                    } else {
                        codex_core::protocol::ReviewDecision::Reject
                    },
                }
            }
            UserInput::PatchApproval { request_id, decision } => {
                Op::PatchApproval {
                    id: request_id,
                    decision: match decision {
                        ReviewDecision::Accept => codex_core::protocol::ReviewDecision::Accept,
                        ReviewDecision::Reject => codex_core::protocol::ReviewDecision::Reject,
                        ReviewDecision::Modify { .. } => codex_core::protocol::ReviewDecision::Accept, // Simplified
                    },
                }
            }
            UserInput::GetHistory => Op::GetPath,
            UserInput::ListMcpTools => Op::ListMcpTools,
        };

        let submission = Submission {
            id: submission_id,
            op,
        };

        self.codex.submit(submission).await
            .map_err(|e| SdkError::ProviderError {
                provider: "Codex".to_string(),
                message: e.to_string(),
            })
    }

    fn receive_events(&mut self) -> Pin<Box<dyn Stream<Item = Result<AgentEvent>> + Send + '_>> {
        use codex_core::protocol::Event;

        let events = self.codex.events();

        Box::pin(events.map(|event| {
            convert_codex_event_to_agent_event(event)
        }))
    }

    async fn interrupt(&mut self) -> Result<()> {
        use codex_core::protocol::{Submission, Op};

        let submission = Submission {
            id: format!("interrupt_{}", uuid::Uuid::new_v4()),
            op: Op::Interrupt,
        };

        self.codex.submit(submission).await
            .map_err(|e| SdkError::ProviderError {
                provider: "Codex".to_string(),
                message: e.to_string(),
            })
    }

    async fn get_usage(&self) -> UsageStats {
        // Codex tracks usage differently
        // May need to aggregate from events
        UsageStats::default()
    }

    fn metadata(&self) -> &SessionMetadata {
        &self.metadata
    }

    async fn close(self: Box<Self>) -> Result<()> {
        use codex_core::protocol::{Submission, Op};

        let submission = Submission {
            id: format!("shutdown_{}", uuid::Uuid::new_v4()),
            op: Op::Shutdown,
        };

        self.codex.submit(submission).await
            .map_err(|e| SdkError::ProviderError {
                provider: "Codex".to_string(),
                message: e.to_string(),
            })
    }
}
```

### Message Converters

```rust
/// Convert Claude Code Message → AgentEvent
fn convert_claude_message_to_event(msg: Message) -> Result<AgentEvent> {
    match msg {
        Message::Assistant { message } => {
            for block in message.content {
                match block {
                    ContentBlock::Text(text) => {
                        return Ok(AgentEvent::TextDelta {
                            text: text.text,
                        });
                    }
                    ContentBlock::ToolUse(tool_use) => {
                        return Ok(AgentEvent::ToolUse {
                            tool_name: tool_use.name,
                            tool_id: tool_use.id,
                            input: tool_use.input,
                        });
                    }
                    ContentBlock::Thinking(thinking) => {
                        return Ok(AgentEvent::ThinkingDelta {
                            content: thinking.content,
                        });
                    }
                    _ => {}
                }
            }
            Err(SdkError::ParseError {
                message: "No recognized content block".to_string(),
                raw: format!("{:?}", message),
            })
        }
        Message::Result { usage, total_cost_usd, .. } => {
            if let Some(usage_json) = usage {
                Ok(AgentEvent::UsageUpdate {
                    input_tokens: usage_json["input_tokens"].as_u64().unwrap_or(0),
                    output_tokens: usage_json["output_tokens"].as_u64().unwrap_or(0),
                    cost_usd: total_cost_usd.unwrap_or(0.0),
                })
            } else {
                Ok(AgentEvent::TurnComplete {
                    stop_reason: StopReason::EndTurn,
                })
            }
        }
        _ => Err(SdkError::ParseError {
            message: "Unhandled message type".to_string(),
            raw: format!("{:?}", msg),
        }),
    }
}

/// Convert Codex Event → AgentEvent
fn convert_codex_event_to_agent_event(event: codex_core::protocol::Event) -> Result<AgentEvent> {
    use codex_core::protocol::EventMsg;

    match event.msg {
        EventMsg::AgentMessageDelta { delta } => {
            Ok(AgentEvent::TextDelta { text: delta })
        }
        EventMsg::AgentReasoningDelta { delta } => {
            Ok(AgentEvent::ThinkingDelta { content: delta })
        }
        EventMsg::TokenCount { usage } => {
            Ok(AgentEvent::UsageUpdate {
                input_tokens: usage.input_tokens,
                output_tokens: usage.output_tokens,
                cost_usd: 0.0, // Codex doesn't provide cost directly
            })
        }
        EventMsg::ExecApprovalRequest { id, command, cwd, parsed_command } => {
            Ok(AgentEvent::ExecApprovalRequest {
                request_id: id,
                command,
                cwd,
                parsed_command: parsed_command.map(|p| p.to_string()),
            })
        }
        EventMsg::ApplyPatchApprovalRequest { id, diff, files_changed } => {
            Ok(AgentEvent::PatchApprovalRequest {
                request_id: id,
                patch_content: diff,
                files_affected: files_changed,
            })
        }
        EventMsg::TurnAborted { reason } => {
            Ok(AgentEvent::TurnComplete {
                stop_reason: match reason {
                    codex_core::protocol::TurnAbortReason::UserInterrupt => StopReason::UserInterrupt,
                    _ => StopReason::Error,
                },
            })
        }
        EventMsg::StreamError { message } => {
            Ok(AgentEvent::Error {
                message,
                error_type: ErrorType::InternalError,
                recoverable: false,
            })
        }
        _ => {
            Err(SdkError::ParseError {
                message: "Unhandled Codex event type".to_string(),
                raw: format!("{:?}", event),
            })
        }
    }
}
```

## Usage Examples

### Basic Usage with Single Provider

```rust
use cc_sdk::unified::{ClaudeCodeProvider, UserInput, AgentEvent};
use cc_sdk::ClaudeCodeOptions;

async fn simple_query() -> Result<()> {
    // Create provider
    let provider = ClaudeCodeProvider::new(
        ClaudeCodeOptions::builder()
            .model("claude-3-5-haiku-20241022")
            .max_output_tokens(2000)
            .build()
    );

    // Create session
    let mut session = provider.create_session(SessionConfig {
        initial_prompt: Some("Hello!".to_string()),
        max_turns: Some(5),
    }).await?;

    // Send input
    session.send_input(UserInput::Text("What is 2+2?".to_string())).await?;

    // Receive events
    let mut events = session.receive_events();
    while let Some(event) = events.next().await {
        match event? {
            AgentEvent::TextDelta { text } => print!("{}", text),
            AgentEvent::TurnComplete { .. } => break,
            _ => {}
        }
    }

    // Get usage stats
    let usage = session.get_usage().await;
    println!("\nTokens: {}, Cost: ${:.3}",
        usage.total_input_tokens + usage.total_output_tokens,
        usage.total_cost_usd);

    session.close().await?;
    Ok(())
}
```

### Dynamic Provider Routing

```rust
use cc_sdk::unified::{
    ProviderRouter, TaskDescriptor, TaskType, ModelPreference,
    ClaudeCodeProvider, CodexProvider, RoutingStrategy,
};

async fn dynamic_routing_example() -> Result<()> {
    // Setup providers
    let claude_provider = Box::new(ClaudeCodeProvider::new(
        ClaudeCodeOptions::builder()
            .model("claude-3-5-haiku-20241022")
            .max_output_tokens(2000)
            .build()
    )) as Box<dyn AgentProvider>;

    let codex_provider = Box::new(CodexProvider::new(
        config,
        auth_manager,
    )) as Box<dyn AgentProvider>;

    // Create router
    let router = ProviderRouter::new(vec![claude_provider, codex_provider])
        .with_strategy(RoutingStrategy::CostOptimized);

    // Define tasks with different requirements

    // Task 1: Simple Q&A - will use Claude Code (cheaper, faster)
    let simple_task = TaskDescriptor {
        task_type: TaskType::SimpleQuery,
        required_capabilities: vec![],
        model_preference: ModelPreference::Cost,
        max_cost: Some(0.01),
        max_latency: None,
        priority: Priority::Normal,
    };

    let (mut session, provider_type) = router.create_session_for_task(
        simple_task,
        SessionConfig {
            initial_prompt: Some("What is Rust?".to_string()),
            max_turns: Some(1),
        },
    ).await?;

    println!("Using provider: {:?}", provider_type);

    // ... use session ...

    // Task 2: Code review - will use Codex (supports review mode)
    let review_task = TaskDescriptor {
        task_type: TaskType::CodeReview,
        required_capabilities: vec![Capability::CodeReview],
        model_preference: ModelPreference::Quality,
        max_cost: None,
        max_latency: None,
        priority: Priority::High,
    };

    let (mut session, provider_type) = router.create_session_for_task(
        review_task,
        SessionConfig {
            initial_prompt: Some("Review this code...".to_string()),
            max_turns: Some(5),
        },
    ).await?;

    println!("Using provider: {:?}", provider_type);

    // ... use session ...

    Ok(())
}
```

### Custom Routing Strategy

```rust
use cc_sdk::unified::{ProviderRouter, RoutingStrategy, TaskDescriptor};
use std::sync::Arc;

async fn custom_routing() -> Result<()> {
    let router = ProviderRouter::new(providers)
        .with_strategy(RoutingStrategy::Custom(Arc::new(
            |task: &TaskDescriptor, providers: &[&dyn AgentProvider]| -> Option<usize> {
                // Custom logic: Use Codex for code tasks, Claude Code for everything else

                let is_code_task = matches!(
                    task.task_type,
                    TaskType::CodeGeneration | TaskType::CodeReview | TaskType::FileEditing
                );

                if is_code_task {
                    // Find Codex provider
                    providers.iter().position(|p| {
                        p.provider_type() == ProviderType::Codex
                    })
                } else {
                    // Find Claude Code provider
                    providers.iter().position(|p| {
                        p.provider_type() == ProviderType::ClaudeCode
                    })
                }
            }
        )));

    // Use router...
    Ok(())
}
```

### Task-Specific Routing Configuration

```rust
use cc_sdk::unified::*;

pub struct TaskRouter {
    router: ProviderRouter,
}

impl TaskRouter {
    pub fn new(providers: Vec<Box<dyn AgentProvider>>) -> Self {
        Self {
            router: ProviderRouter::new(providers),
        }
    }

    /// Route simple queries to cheapest provider
    pub async fn simple_query(&self, query: &str) -> Result<Box<dyn AgentSession>> {
        let task = TaskDescriptor {
            task_type: TaskType::SimpleQuery,
            required_capabilities: vec![],
            model_preference: ModelPreference::Cost,
            max_cost: Some(0.01),
            max_latency: Some(Duration::from_secs(5)),
            priority: Priority::Normal,
        };

        let (session, _) = self.router.create_session_for_task(
            task,
            SessionConfig {
                initial_prompt: Some(query.to_string()),
                max_turns: Some(1),
            },
        ).await?;

        Ok(session)
    }

    /// Route code generation to balanced provider
    pub async fn generate_code(&self, prompt: &str) -> Result<Box<dyn AgentSession>> {
        let task = TaskDescriptor {
            task_type: TaskType::CodeGeneration,
            required_capabilities: vec![Capability::Tools],
            model_preference: ModelPreference::Balanced,
            max_cost: Some(0.10),
            max_latency: None,
            priority: Priority::Normal,
        };

        let (session, _) = self.router.create_session_for_task(
            task,
            SessionConfig {
                initial_prompt: Some(prompt.to_string()),
                max_turns: Some(10),
            },
        ).await?;

        Ok(session)
    }

    /// Route code review to quality-focused provider (likely Codex)
    pub async fn review_code(&self, context: Vec<ContextItem>) -> Result<Box<dyn AgentSession>> {
        let task = TaskDescriptor {
            task_type: TaskType::CodeReview,
            required_capabilities: vec![Capability::CodeReview],
            model_preference: ModelPreference::Quality,
            max_cost: None, // No cost limit for reviews
            max_latency: None,
            priority: Priority::High,
        };

        let (session, _) = self.router.create_session_for_task(
            task,
            SessionConfig {
                initial_prompt: None,
                max_turns: Some(5),
            },
        ).await?;

        // Send context immediately
        session.send_input(UserInput::TextWithContext {
            text: "Please review this code.".to_string(),
            context,
        }).await?;

        Ok(session)
    }
}
```

## Implementation Plan

### Phase 0: Research & Design ✅ COMPLETED
- [x] Analyzed Codex Rust codebase (`/Users/zhangalex/Work/Projects/ai/codex/codex-rs`)
- [x] Identified SQ/EQ pattern and 25+ event types
- [x] Analyzed TypeScript SDK wrapper architecture
- [x] Documented key differences between Claude Code SDK and Codex
- [x] Updated design document with research findings

### Phase 1: Core Abstraction Layer (2 weeks)

**Week 1: Foundation**
- [ ] Create `src/unified/` module structure
- [ ] Define core traits in `src/unified/traits.rs`:
  - `AgentProvider` trait with provider_type(), create_session(), capabilities()
  - `AgentSession` trait with send_input(), receive_events(), interrupt()
- [ ] Define unified types in `src/unified/types.rs`:
  - `UserInput` enum (Text, TextWithContext, ToolApproval, etc.)
  - `AgentEvent` enum (TextDelta, ToolUse, UsageUpdate, etc.)
  - `ProviderCapabilities` struct with feature flags
- [ ] Implement `ProviderType` enum (ClaudeCode, Codex, DirectAPI, Custom)
- [ ] Unit tests for type conversions

**Week 2: Message Converters**
- [ ] Implement `src/unified/converters/claude_to_unified.rs`:
  - `convert_claude_message_to_event()` - Message → AgentEvent
  - Handle Assistant, Result, System messages
  - Extract usage stats from Result messages
- [ ] Implement `src/unified/converters/codex_to_unified.rs`:
  - `convert_codex_event_to_agent_event()` - codex::Event → AgentEvent
  - Handle 25+ EventMsg variants
  - Map token counts, approvals, tool calls
- [ ] Implement `src/unified/converters/unified_to_codex.rs`:
  - `convert_user_input_to_op()` - UserInput → codex::Op
  - Handle Op::UserInput, Op::Interrupt, Op::ExecApproval, etc.
- [ ] Integration tests for bidirectional conversion

### Phase 2: Provider Implementations (3 weeks)

**Week 3: ClaudeCodeProvider**
- [ ] Implement `src/unified/providers/claude_code.rs`:
  - `ClaudeCodeProvider` struct wrapping `ClaudeCodeOptions`
  - `ClaudeCodeSession` struct wrapping `ClaudeSDKClient`
  - AgentProvider trait implementation
  - AgentSession trait implementation
- [ ] Handle control protocol nuances (Legacy vs Control format)
- [ ] Map capabilities from ClaudeCodeOptions
- [ ] Tests for ClaudeCodeProvider

**Week 4-5: CodexProvider**
- [ ] Add `codex-core` as optional dependency in Cargo.toml
- [ ] Implement `src/unified/providers/codex.rs`:
  - `CodexProvider` struct wrapping `codex_core::Config`
  - `CodexSession` struct wrapping `codex_core::Codex`
  - Handle SQ/EQ pattern (Submission/Event queues)
  - Implement rollout persistence support
- [ ] Implement session resumption from `~/.codex/sessions`
- [ ] Map Codex capabilities (supports_code_review: true, etc.)
- [ ] Handle ConversationManager integration
- [ ] Tests for CodexProvider

### Phase 3: Dynamic Routing System (2 weeks)

**Week 6: Router Core**
- [ ] Implement `src/unified/routing/router.rs`:
  - `ProviderRouter` struct
  - `select_provider()` method with strategy dispatch
  - `create_session_for_task()` helper
- [ ] Implement `src/unified/routing/strategies.rs`:
  - `FirstAvailable` - use first capable provider
  - `CostOptimized` - select cheapest provider
  - `LatencyOptimized` - select fastest provider
  - `Balanced` - score-based selection
  - `Custom` - user-defined routing function
- [ ] Define `TaskDescriptor` struct with task_type, capabilities, preferences

**Week 7: Cost Estimation & Capabilities**
- [ ] Implement `src/unified/routing/cost_estimation.rs`:
  - Cost models for different task types
  - Model cost multipliers (Haiku: 1x, Sonnet: 5x, Opus: 15x)
  - Token estimation heuristics
- [ ] Implement `src/unified/capabilities.rs`:
  - `Capability` enum (Tools, MCP, Hooks, CodeReview, etc.)
  - `matches_capabilities()` method for filtering
  - Provider capability detection
- [ ] Router integration tests
- [ ] Benchmark routing decisions

### Phase 4: Advanced Features (2 weeks)

**Week 8: Multi-Provider Features**
- [ ] Implement session pooling for concurrent requests
- [ ] Add provider failover/fallback logic
- [ ] Cross-provider token budget management
- [ ] Unified MCP server registry (works with both providers)

**Week 9: Performance & Monitoring**
- [ ] Add performance metrics collection
- [ ] Implement zero-copy optimizations for converters
- [ ] Add routing decision logging/telemetry
- [ ] Benchmark overhead vs direct provider usage (target: <5%)

### Phase 5: Documentation & Examples (1 week)

**Week 10: Polish**
- [ ] Write comprehensive API documentation
- [ ] Create examples:
  - `examples/unified_simple.rs` - Basic single-provider usage
  - `examples/unified_routing.rs` - Dynamic routing demo
  - `examples/unified_claude_vs_codex.rs` - Side-by-side comparison
  - `examples/task_router.rs` - Task-specific routing patterns
- [ ] Write migration guide from direct ClaudeSDKClient usage
- [ ] Performance benchmarking report
- [ ] Integration tests covering all providers and strategies

### Phased Rollout Strategy

**Phase 1-2: Internal (Weeks 1-5)**
- Core abstractions and ClaudeCodeProvider
- Backward compatible (no breaking changes)
- Feature flag: `unified-providers` (opt-in)

**Phase 3: Beta (Weeks 6-7)**
- Add routing system
- CodexProvider behind feature flag: `codex-provider`
- Gather feedback from early adopters

**Phase 4-5: Stable (Weeks 8-10)**
- Advanced features and optimizations
- Full documentation
- Promote to stable in v0.2.0 release

### Success Criteria

1. **API Simplicity**: Create session with < 10 lines of code ✓
2. **Routing Accuracy**: >90% tasks routed to optimal provider
3. **Performance**: <5% overhead vs direct provider usage
4. **Test Coverage**: >80% coverage for unified layer
5. **Adoption**: Positive user feedback, no major migration blockers

## File Structure

```
src/
├── unified/
│   ├── mod.rs                      # Public exports
│   ├── traits.rs                   # AgentProvider, AgentSession
│   ├── types.rs                    # UserInput, AgentEvent, etc.
│   ├── routing/
│   │   ├── mod.rs
│   │   ├── router.rs               # ProviderRouter
│   │   ├── strategies.rs           # Routing strategies
│   │   └── cost_estimation.rs      # Cost models
│   ├── providers/
│   │   ├── mod.rs
│   │   ├── claude_code.rs          # ClaudeCodeProvider
│   │   └── codex.rs                # CodexProvider
│   ├── converters/
│   │   ├── mod.rs
│   │   ├── claude_to_unified.rs
│   │   └── codex_to_unified.rs
│   ├── session.rs                  # SessionConfig, SessionMetadata
│   └── capabilities.rs             # Capability definitions
├── lib.rs                          # Export unified module
└── ... (existing files)

examples/
├── unified_simple.rs               # Basic usage
├── unified_routing.rs              # Dynamic routing
├── unified_claude_vs_codex.rs      # Provider comparison
└── task_router.rs                  # Task-specific routing

tests/
├── unified_tests.rs                # Integration tests
├── provider_tests.rs               # Provider-specific tests
└── routing_tests.rs                # Router tests

docs/
└── UNIFIED_AGENT_ABSTRACTION.md    # This document
```

## Benefits

1. **Flexibility**: Switch between Claude Code and Codex based on task requirements
2. **Cost Optimization**: Route simple queries to cheaper providers
3. **Performance**: Use fastest provider for latency-sensitive tasks
4. **Feature Access**: Leverage unique capabilities of each provider
5. **Future-Proof**: Easy to add Direct API, custom providers, etc.
6. **Type Safety**: Rust's type system ensures correctness
7. **Backward Compatible**: Existing `ClaudeSDKClient` API unchanged

## Risks & Mitigations

### Risk 1: Feature Parity
Different providers have different capabilities.

**Mitigation**:
- Explicit capability declaration
- Capability checking before routing
- Graceful degradation when features unavailable

### Risk 2: Performance Overhead
Message conversion adds latency.

**Mitigation**:
- Zero-copy conversions where possible
- Benchmark and optimize hot paths
- Optional direct provider access for performance-critical code

### Risk 3: Complexity
Abstraction layer increases complexity.

**Mitigation**:
- Keep existing APIs unchanged
- Make unified layer optional
- Clear documentation and examples

### Risk 4: Maintenance Burden
Multiple providers = more code to maintain.

**Mitigation**:
- Comprehensive test coverage
- CI/CD for all providers
- Clear separation of concerns

## Success Metrics

1. **API Simplicity**: Can create session with < 10 lines of code
2. **Routing Accuracy**: >90% of tasks routed to optimal provider
3. **Performance**: < 5% overhead vs direct provider usage
4. **Coverage**: >80% test coverage for unified layer
5. **Adoption**: Positive user feedback on ease of use

## Future Extensions

1. **Direct API Provider**: Native Anthropic API support
2. **Multi-Provider Sessions**: Use multiple providers in single session
3. **Provider Pools**: Load balancing across multiple instances
4. **Smart Caching**: Cache responses across providers
5. **A/B Testing**: Compare provider responses
6. **Cost Tracking**: Detailed cost analytics per provider
7. **Custom Providers**: Plugin system for user-defined providers

## Conclusion

This unified abstraction layer provides a flexible, type-safe way to work with multiple AI agent providers while maintaining the simplicity and power of the existing Claude Code SDK. The dynamic routing system enables intelligent task distribution based on cost, performance, and capability requirements, making it easier to build robust, efficient AI-powered applications.
