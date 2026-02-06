//! Strongly-typed hooks example (v0.3.0)
//!
//! This example demonstrates how to use the new strongly-typed hook system
//! introduced in v0.3.0. Hooks now use `HookInput` and `HookJSONOutput`
//! for better type safety and IDE support.

use async_trait::async_trait;
use futures::StreamExt;
use nexus_claude::{
    ClaudeCodeOptions, HookCallback, HookContext, HookInput, HookJSONOutput, HookMatcher,
    PermissionMode, SdkError, SyncHookJSONOutput, query,
};
use std::collections::HashMap;
use std::sync::Arc;

/// Example hook that logs all tool uses
struct ToolUseLogger;

#[async_trait]
impl HookCallback for ToolUseLogger {
    async fn execute(
        &self,
        input: &HookInput,
        _tool_use_id: Option<&str>,
        _context: &HookContext,
    ) -> Result<HookJSONOutput, SdkError> {
        match input {
            HookInput::PreToolUse(pre_tool_use) => {
                // Strongly-typed access to hook input
                println!("🔧 About to use tool: {}", pre_tool_use.tool_name);
                println!(
                    "   Input: {}",
                    serde_json::to_string_pretty(&pre_tool_use.tool_input).unwrap_or_default()
                );
                println!("   CWD: {}", pre_tool_use.cwd);
                println!("   Session ID: {}", pre_tool_use.session_id);

                // Return sync hook output allowing the tool use
                Ok(HookJSONOutput::Sync(SyncHookJSONOutput {
                    continue_: Some(true),
                    reason: Some("Tool use logged".to_string()),
                    ..Default::default()
                }))
            },
            HookInput::PostToolUse(post_tool_use) => {
                println!("✅ Tool completed: {}", post_tool_use.tool_name);
                println!(
                    "   Response: {}",
                    serde_json::to_string_pretty(&post_tool_use.tool_response)
                        .unwrap_or_default()
                        .chars()
                        .take(200)
                        .collect::<String>()
                );

                // Return success with no modifications
                Ok(HookJSONOutput::Sync(SyncHookJSONOutput::default()))
            },
            _ => {
                // For other hook types, just continue
                Ok(HookJSONOutput::Sync(SyncHookJSONOutput::default()))
            },
        }
    }
}

/// Example hook that blocks certain tools
struct ToolBlocker {
    blocked_tools: Vec<String>,
}

#[async_trait]
impl HookCallback for ToolBlocker {
    async fn execute(
        &self,
        input: &HookInput,
        _tool_use_id: Option<&str>,
        _context: &HookContext,
    ) -> Result<HookJSONOutput, SdkError> {
        if let HookInput::PreToolUse(pre_tool_use) = input {
            // Check if tool is blocked
            if self.blocked_tools.contains(&pre_tool_use.tool_name) {
                println!("🚫 Blocked tool: {}", pre_tool_use.tool_name);

                // Return hook output that blocks the tool
                return Ok(HookJSONOutput::Sync(SyncHookJSONOutput {
                    continue_: Some(false),
                    decision: Some("block".to_string()),
                    system_message: Some(format!(
                        "Tool '{}' is blocked by policy",
                        pre_tool_use.tool_name
                    )),
                    reason: Some("This tool is not allowed in this session".to_string()),
                    stop_reason: Some("Tool blocked by security policy".to_string()),
                    ..Default::default()
                }));
            }
        }

        // Allow other tools
        Ok(HookJSONOutput::Sync(SyncHookJSONOutput {
            continue_: Some(true),
            ..Default::default()
        }))
    }
}

/// Example hook that adds context to user prompts
struct PromptEnhancer;

#[async_trait]
impl HookCallback for PromptEnhancer {
    async fn execute(
        &self,
        input: &HookInput,
        _tool_use_id: Option<&str>,
        _context: &HookContext,
    ) -> Result<HookJSONOutput, SdkError> {
        if let HookInput::UserPromptSubmit(prompt_submit) = input {
            println!("📝 User prompt: {}", prompt_submit.prompt);
            println!("   Adding helpful context...");

            // Return hook output with additional context
            use nexus_claude::HookSpecificOutput;
            use nexus_claude::UserPromptSubmitHookSpecificOutput;

            return Ok(HookJSONOutput::Sync(SyncHookJSONOutput {
                hook_specific_output: Some(HookSpecificOutput::UserPromptSubmit(
                    UserPromptSubmitHookSpecificOutput {
                        additional_context: Some(
                            "Remember to be concise and clear in your responses.".to_string(),
                        ),
                    },
                )),
                ..Default::default()
            }));
        }

        Ok(HookJSONOutput::Sync(SyncHookJSONOutput::default()))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Strongly-typed Hooks Example (v0.3.0) ===\n");

    // Create hook instances
    let logger = Arc::new(ToolUseLogger);
    let blocker = Arc::new(ToolBlocker {
        blocked_tools: vec!["Bash".to_string()], // Block Bash tool for demo
    });
    let enhancer = Arc::new(PromptEnhancer);

    // Configure hooks
    let mut hooks = HashMap::new();

    // PreToolUse hooks: log and optionally block tools
    hooks.insert(
        "PreToolUse".to_string(),
        vec![HookMatcher {
            matcher: Some(serde_json::json!("*")), // Match all tools
            hooks: vec![logger.clone(), blocker],
        }],
    );

    // PostToolUse hooks: log tool completion
    hooks.insert(
        "PostToolUse".to_string(),
        vec![HookMatcher {
            matcher: Some(serde_json::json!("*")),
            hooks: vec![logger.clone()],
        }],
    );

    // UserPromptSubmit hooks: enhance prompts
    hooks.insert(
        "UserPromptSubmit".to_string(),
        vec![HookMatcher {
            matcher: None,
            hooks: vec![enhancer],
        }],
    );

    // Create options with hooks
    let options = ClaudeCodeOptions {
        hooks: Some(hooks),
        permission_mode: PermissionMode::BypassPermissions,
        ..Default::default()
    };

    println!("Sending query with hooks enabled...\n");

    // Send a query that will trigger hooks
    let mut stream = query(
        "What is 2 + 2? Please calculate this using a tool.",
        Some(options),
    )
    .await?;

    // Process responses
    while let Some(msg) = stream.next().await {
        match msg {
            Ok(message) => {
                println!("📨 Message: {message:?}");
            },
            Err(e) => {
                eprintln!("❌ Error: {e}");
            },
        }
    }

    println!("\n=== Example complete ===");

    Ok(())
}
