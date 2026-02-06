# Nexus - Claude Code SDK for Rust

[![CI](https://github.com/this-rs/nexus/actions/workflows/ci.yml/badge.svg)](https://github.com/this-rs/nexus/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/this-rs/nexus/branch/main/graph/badge.svg)](https://codecov.io/gh/this-rs/nexus)
[![Crates.io](https://img.shields.io/crates/v/nexus-claude.svg)](https://crates.io/crates/nexus-claude)
[![Documentation](https://docs.rs/nexus-claude/badge.svg)](https://docs.rs/nexus-claude)
[![License](https://img.shields.io/crates/l/nexus-claude.svg)](LICENSE)
[![MSRV](https://img.shields.io/badge/MSRV-1.88-orange.svg)](https://www.rust-lang.org)

**Nexus** is a Rust SDK for Claude Code CLI with **persistent memory** and **autonomous context retrieval**. It provides both simple query interfaces and full interactive client capabilities, enhanced with cross-session memory that automatically retrieves relevant context.

> **Fork Notice**: This project is a fork of [ZhangHanDong/claude-code-api-rs](https://github.com/ZhangHanDong/claude-code-api-rs) (`cc-sdk`), extended with persistent memory capabilities and autonomous context retrieval.

## What's New in Nexus

- **Persistent Memory System** - Conversations are stored and indexed for future retrieval
- **Multi-Factor Relevance Scoring** - Context is scored by semantic similarity, working directory, file overlap, and recency
- **Autonomous Context Injection** - Relevant historical context is automatically injected into prompts
- **Tool Context Extraction** - Automatically tracks files touched and working directory changes

## Features

### Core Features (from cc-sdk)
- Simple Query Interface - One-shot queries with the `query()` function
- Interactive Client - Stateful conversations with context retention
- Streaming Support - Real-time message streaming
- Interrupt Capability - Cancel ongoing operations
- Full Configuration - Comprehensive options for Claude Code
- Type Safety - Strongly typed with serde support
- Async/Await - Built on Tokio for async operations
- Control Protocol - Full support for permissions, hooks, and MCP servers
- Token Optimization - Built-in tools to minimize costs and track usage
- Auto CLI Download - Automatically downloads Claude Code CLI if not found
- File Checkpointing - Rewind file changes to any point in conversation
- Structured Output - JSON schema validation for responses

### Nexus Memory Features
- **MessageDocument** - Rich message storage with cwd, files_touched, and summaries
- **ToolContextExtractor** - Extracts context from Read, Write, Edit, Bash, Glob, Grep tool calls
- **RelevanceScorer** - Multi-factor scoring: `relevance = (semantic * 0.4) + (cwd_match * 0.3) + (files_overlap * 0.2) + (recency * 0.1)`
- **ContextInjector** - Retrieves and formats historical context for prompt injection
- **Meilisearch Integration** - Full-text search with semantic retrieval

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
nexus-claude = "0.5.0"
tokio = { version = "1.0", features = ["full"] }
futures = "0.3"
```

### With Persistent Memory

To enable the memory system (requires Meilisearch):

```toml
[dependencies]
nexus-claude = { version = "0.5.0", features = ["memory"] }
```

## Quick Start

### Simple Query

```rust
use nexus_claude::{query, Result};
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<()> {
    let mut messages = query("What is 2 + 2?", None).await?;

    while let Some(msg) = messages.next().await {
        println!("{:?}", msg?);
    }

    Ok(())
}
```

### Interactive Client

```rust
use nexus_claude::{InteractiveClient, ClaudeCodeOptions, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let mut client = InteractiveClient::new(ClaudeCodeOptions::default())?;
    client.connect().await?;

    let messages = client.send_and_receive(
        "Help me write a Python web server".to_string()
    ).await?;

    for msg in &messages {
        if let nexus_claude::Message::Assistant { message } = msg {
            println!("Claude: {:?}", message);
        }
    }

    client.disconnect().await?;
    Ok(())
}
```

## Memory System

The memory system enables persistent context across sessions. Messages are stored with metadata (working directory, files touched) and retrieved using multi-factor relevance scoring.

### Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    Memory System Architecture                    │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────┐    ┌─────────────────┐    ┌───────────────┐   │
│  │ Tool Calls  │───>│ ToolContext     │───>│ MessageDoc    │   │
│  │ (Read,Edit) │    │ Extractor       │    │ (cwd,files)   │   │
│  └─────────────┘    └─────────────────┘    └───────┬───────┘   │
│                                                     │           │
│                                                     v           │
│  ┌─────────────┐    ┌─────────────────┐    ┌───────────────┐   │
│  │ Meilisearch │<───│ Memory Provider │<───│ Store         │   │
│  │ (storage)   │    │                 │    │               │   │
│  └──────┬──────┘    └─────────────────┘    └───────────────┘   │
│         │                                                       │
│         v                                                       │
│  ┌─────────────┐    ┌─────────────────┐    ┌───────────────┐   │
│  │ Semantic    │───>│ Relevance       │───>│ Context       │   │
│  │ Search      │    │ Scorer          │    │ Injector      │   │
│  └─────────────┘    └─────────────────┘    └───────────────┘   │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Multi-Factor Relevance Scoring

Context is scored using multiple factors:

```
relevance = (semantic * 0.4) + (cwd_match * 0.3) + (files_overlap * 0.2) + (recency * 0.1)
```

| Factor | Weight | Description |
|--------|--------|-------------|
| Semantic | 0.4 | Text similarity via Meilisearch |
| CWD Match | 0.3 | 1.0 = exact match, 0.5 = parent/child, 0.0 = no relation |
| Files Overlap | 0.2 | Jaccard index: \|A∩B\| / \|A∪B\| |
| Recency | 0.1 | Exponential decay: e^(-age_hours / 24) |

### Using the Memory System

```rust
use nexus_claude::memory::{
    MemoryIntegrationBuilder, ContextInjector, MemoryConfig,
};

#[tokio::main]
async fn main() -> Result<()> {
    // Create a memory manager for conversation tracking
    let mut manager = MemoryIntegrationBuilder::new()
        .enabled(true)
        .cwd("/projects/my-app")
        .url("http://localhost:7700")  // Meilisearch URL
        .min_relevance_score(0.3)
        .max_context_items(5)
        .build();

    // Record messages during conversation
    manager.record_user_message("How do I implement JWT auth?");

    // Process tool calls to capture context
    manager.process_tool_call("Read", &serde_json::json!({
        "file_path": "/projects/my-app/src/auth.rs"
    }));

    manager.record_assistant_message("I've analyzed your auth module...");

    // Store messages for future retrieval
    let messages = manager.take_pending_messages();

    // Create context injector for retrieval
    let injector = ContextInjector::new(
        MemoryConfig::default()
            .with_url("http://localhost:7700")
            .with_enabled(true)
    ).await?;

    // Retrieve relevant historical context
    let context = injector.get_context_prefix(
        "JWT authentication",
        Some("/projects/my-app"),
        &["/projects/my-app/src/auth.rs".to_string()],
    ).await?;

    if let Some(ctx) = context {
        println!("Injected context:\n{}", ctx);
    }

    Ok(())
}
```

### Memory Chat Example

Try the interactive memory chat to see the system in action:

```bash
# Start Meilisearch
docker run -d -p 7700:7700 getmeili/meilisearch:latest

# Run the memory chat example
cargo run --example memory_chat --features memory

# With verbose mode to see injected context
cargo run --example memory_chat --features memory -- --verbose
```

**Commands:**
- `/help` - Show available commands
- `/context` - Show current context (cwd, files touched)
- `/history` - Show retrieved historical context
- `/stats` - Show memory statistics
- `/clear` - Clear current conversation
- `/quit` - Exit the chat

## Configuration Options

```rust
use nexus_claude::{ClaudeCodeOptions, PermissionMode};

let options = ClaudeCodeOptions::builder()
    .system_prompt("You are a helpful coding assistant")
    .model("sonnet")  // or "opus", "haiku"
    .permission_mode(PermissionMode::AcceptEdits)
    .max_turns(10)
    .max_output_tokens(2000)
    .cwd("/path/to/project")
    .add_dir("/path/to/related/project")
    .auto_download_cli(true)  // Default: auto-download if not found
    .build();
```

## Supported Models

| Model | Alias | Description |
|-------|-------|-------------|
| Claude Opus 4.5 | `"opus"` | Most capable |
| Claude Sonnet 4.5 | `"sonnet"` | Balanced (recommended) |
| Claude Haiku 4.5 | `"haiku"` | Fastest and cheapest |

## Python SDK Parity

This SDK maintains **100% feature parity** with the official Python `claude-agent-sdk`:

| Feature | Status |
|---------|--------|
| Simple query API | Parity |
| Interactive client | Parity |
| Streaming messages | Parity |
| Tools configuration | Parity |
| Permission modes | Parity |
| MCP servers | Parity |
| Hooks and callbacks | Parity |
| Auto CLI download | Parity |
| File checkpointing | Parity |
| Structured output | Parity |

## Prerequisites

### Claude Code CLI

The CLI is **automatically downloaded** if not found on your system.

For manual installation:
```bash
npm install -g @anthropic-ai/claude-code
```

### Meilisearch (for memory feature)

```bash
docker run -d -p 7700:7700 getmeili/meilisearch:latest
```

Or with a master key:
```bash
docker run -d -p 7700:7700 \
  -e MEILI_MASTER_KEY='your-key' \
  getmeili/meilisearch:latest
```

## Environment Variables

```bash
# Required for SDK operation
export ANTHROPIC_USER_EMAIL="your-email@example.com"

# Optional: Model selection
export CLAUDE_MODEL="claude-sonnet-4-5-20250929"

# Optional: Meilisearch configuration (for memory feature)
export MEILISEARCH_URL="http://localhost:7700"
export MEILISEARCH_KEY="your-key"  # If using authentication
```

## Documentation

- [Token Optimization Guide](docs/TOKEN_OPTIMIZATION.md)
- [Environment Variables](docs/ENVIRONMENT_VARIABLES.md)
- [Models Guide](docs/models-guide.md)
- [Hook Event Names](docs/HOOK_EVENT_NAMES.md)
- [FAQ](docs/FAQ.md)

## Examples

Check the `examples/` directory:

- `memory_chat.rs` - Interactive chat with persistent memory
- `interactive_demo.rs` - Interactive conversation demo
- `simple_query.rs` - Simple query example
- `streaming_output.rs` - Streaming message handling
- `permission_modes.rs` - Permission mode examples
- `hooks_typed.rs` - Strongly-typed hook callbacks

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

This project is a fork of [ZhangHanDong/claude-code-api-rs](https://github.com/ZhangHanDong/claude-code-api-rs) by Zhang Handong. The original project provided the foundation for Claude Code CLI integration in Rust.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## Author

- **Théotime Rivière** - Nexus fork maintainer
- **Zhang Handong** - Original cc-sdk author
