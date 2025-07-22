# Claude Code SDK Examples

This directory contains examples demonstrating various features of the Claude Code SDK for Rust.

## Prerequisites

Before running any examples, ensure you have:

1. **Claude Code CLI installed**:
   ```bash
   npm install -g @anthropic-ai/claude-code
   ```

2. **API credentials configured**:
   ```bash
   export ANTHROPIC_API_KEY="your-api-key"
   ```

## Running Examples

All examples can be run using `cargo run --example`:

```bash
# Navigate to the SDK directory
cd claude-code-sdk-rs

# Run a specific example
cargo run --example simple_query
```

## Available Examples

### 1. Simple Query (`simple_query.rs`)

Basic usage of the `query()` function for one-shot queries.

```bash
cargo run --example simple_query
```

**Features demonstrated:**
- Basic query without options
- Query with custom options (model, system prompt)
- Note: File operations are not supported in query mode

### 2. Interactive Client (`interactive.rs`)

Interactive REPL-style client for conversations with Claude.

```bash
cargo run --example interactive
```

**How to use:**
1. The client will connect and show an initial greeting
2. Type your messages and press Enter to send
3. Claude will respond in real-time
4. Special commands:
   - `quit` - Exit the program
   - `interrupt` - Interrupt the current operation
5. Example conversation:
   ```
   You: What is the capital of France?
   Claude: The capital of France is Paris.
   
   You: Tell me more about it
   Claude: Paris is the capital and largest city of France...
   
   You: quit
   ```

**Features demonstrated:**
- Interactive conversation loop
- Stateful conversation (context is maintained)
- Interrupt handling
- Real-time responses

### 3. Permission Modes (`permission_modes.rs`)

Demonstrates different permission modes and their effects.

```bash
cargo run --example permission_modes
```

**Features demonstrated:**
- `Default` mode - Would prompt for permissions (in interactive mode)
- `AcceptEdits` mode - Auto-accepts edit prompts
- `BypassPermissions` mode - Allows all operations
- Tool restrictions with allowed/disallowed tools

### 4. Query with File Operations (`query_with_file_ops.rs`)

Shows how to use `query()` with `BypassPermissions` for file operations.

```bash
cargo run --example query_with_file_ops
```

**Features demonstrated:**
- Using `BypassPermissions` to allow file operations in query mode
- Creating files with query()
- Important security considerations

### 5. File Operations (`file_operations.rs`)

Demonstrates file operations using the interactive client.

```bash
cargo run --example file_operations
```

**Features demonstrated:**
- Creating files with `ClaudeSDKClient`
- Reading and modifying files
- Using `BypassPermissions` for automatic file operations
- Cost tracking

## Important Notes

### Permission Modes

- **Default**: CLI prompts for dangerous operations (only works in interactive mode)
- **AcceptEdits**: Auto-accepts edit prompts but still checks permissions
- **BypassPermissions**: Bypasses all permission checks - use with caution!

### File Operations

- The `query()` function uses `--print` mode which has limitations
- For file operations in query mode, you must use `BypassPermissions`
- For interactive permission prompts, use `ClaudeSDKClient`

### Security

- Never use `BypassPermissions` in production or untrusted environments
- Always restrict tools using `allowed_tools` when possible
- Be careful with file paths and operations

## Troubleshooting

### "Claude CLI not found"

Ensure Claude CLI is installed and in your PATH:
```bash
which claude
```

### "API key not found"

Set your Anthropic API key:
```bash
export ANTHROPIC_API_KEY="your-key-here"
```

### Permission errors

- In query mode: Use `BypassPermissions`
- In interactive mode: Respond to permission prompts or use appropriate permission mode

### Model errors

Ensure you're using a valid model name:
- `claude-3-5-sonnet-20241022`
- `claude-3-5-haiku-20241022`
- Check the Claude documentation for the latest model names

## New Advanced Examples

### 6. Rust Question Processor (`rust_question_processor.rs`)

A comprehensive example showing how to process Rust programming questions and generate complete solutions.

```bash
cargo run --example rust_question_processor
```

**Features demonstrated:**
- Multi-step processing (create project, verify, document)
- Question set batch processing from files
- Timing and progress tracking
- Error handling with retry logic
- Metadata collection and documentation generation

### 7. Code Generator (`code_generator.rs`)

A focused example on generating Rust code solutions with a clean API.

```bash
cargo run --example code_generator
```

**Features demonstrated:**
- Simple, clean SDK usage
- Step-by-step solution generation
- Progress indicators and emojis
- Multiple example problems
- Concise output formatting

### 8. Batch Processor (`batch_processor.rs`)

Advanced batch processing with sophisticated error handling.

```bash
cargo run --example batch_processor
```

**Features demonstrated:**
- Batch question processing from files
- Rate limit detection and retry logic
- Comprehensive statistics tracking
- Efficient client reuse across questions
- Progress tracking with percentages