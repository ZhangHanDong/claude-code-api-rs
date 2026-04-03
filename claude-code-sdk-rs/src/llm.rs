//! LLM Proxy — use Claude Code subscription as a direct LLM interface.
//!
//! Strips away the CC agent layer (tools, system prompt, hooks) and provides
//! a simple "send prompt, get text" API powered by your CC subscription.
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use cc_sdk::llm::{self, LlmOptions};
//! use futures::StreamExt;
//!
//! # async fn example() -> cc_sdk::Result<()> {
//! // Simple query — returns full text
//! let response = llm::query("Explain quantum entanglement", None).await?;
//! println!("{}", response.text);
//!
//! // With options
//! let opts = LlmOptions::builder()
//!     .system_prompt("You are a concise translator. Translate to Chinese.")
//!     .model("claude-sonnet-4-20250514")
//!     .build();
//! let response = llm::query("Hello world", Some(opts)).await?;
//!
//! // Streaming — text chunks as they arrive
//! let mut stream = llm::query_stream("Write a haiku", None).await?;
//! while let Some(chunk) = stream.next().await {
//!     print!("{}", chunk?);
//! }
//! # Ok(())
//! # }
//! ```

use crate::errors::Result;
use crate::types::{
    ClaudeCodeOptions, ContentBlock, Effort, Message, PermissionMode, SystemPrompt, ThinkingConfig,
};
use futures::stream::Stream;
use futures::StreamExt;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Minimal options for LLM proxy queries.
///
/// Only exposes fields relevant to direct LLM usage.
/// All CC agent features (tools, hooks, plugins) are automatically disabled.
#[derive(Debug, Clone, Default)]
pub struct LlmOptions {
    /// Custom system prompt. Default: empty string (bypasses CC agent prompt).
    pub system_prompt: Option<String>,
    /// Model to use (e.g., `"claude-sonnet-4-20250514"`). Default: CLI default.
    pub model: Option<String>,
    /// Thinking configuration for extended reasoning.
    pub thinking: Option<ThinkingConfig>,
    /// Maximum conversation turns. Default: 1 (single-turn).
    pub max_turns: Option<i32>,
    /// Maximum output tokens (1–32000).
    pub max_output_tokens: Option<u32>,
    /// Effort level for reasoning depth.
    pub effort: Option<Effort>,
}

/// Builder for [`LlmOptions`].
#[derive(Debug, Default)]
pub struct LlmOptionsBuilder {
    options: LlmOptions,
}

impl LlmOptions {
    /// Create a builder for `LlmOptions`.
    pub fn builder() -> LlmOptionsBuilder {
        LlmOptionsBuilder::default()
    }

    /// Convert to [`ClaudeCodeOptions`] with LLM-proxy defaults.
    ///
    /// This sets:
    /// - Empty system prompt (or user-provided)
    /// - `--tools ""` (disable all tools)
    /// - `--bare` (skip hooks, LSP, plugins)
    /// - `PermissionMode::DontAsk`
    /// - `max_turns: 1` (unless overridden)
    pub(crate) fn to_claude_code_options(&self) -> ClaudeCodeOptions {
        let mut extra_args = HashMap::new();
        extra_args.insert("bare".to_string(), None);
        extra_args.insert("tools".to_string(), Some(String::new()));

        ClaudeCodeOptions {
            system_prompt_v2: Some(SystemPrompt::String(
                self.system_prompt.clone().unwrap_or_default(),
            )),
            permission_mode: PermissionMode::DontAsk,
            max_turns: self.max_turns.or(Some(1)),
            model: self.model.clone(),
            thinking: self.thinking.clone(),
            max_output_tokens: self.max_output_tokens,
            effort: self.effort,
            extra_args,
            ..Default::default()
        }
    }
}

impl LlmOptionsBuilder {
    /// Set a custom system prompt. Pass `""` for no system prompt.
    pub fn system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.options.system_prompt = Some(prompt.into());
        self
    }

    /// Set the model (e.g., `"claude-sonnet-4-20250514"`).
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.options.model = Some(model.into());
        self
    }

    /// Enable extended thinking.
    pub fn thinking(mut self, config: ThinkingConfig) -> Self {
        self.options.thinking = Some(config);
        self
    }

    /// Set maximum turns (default: 1 for single-turn).
    pub fn max_turns(mut self, turns: i32) -> Self {
        self.options.max_turns = Some(turns);
        self
    }

    /// Set maximum output tokens (1–32000).
    pub fn max_output_tokens(mut self, tokens: u32) -> Self {
        self.options.max_output_tokens = Some(tokens);
        self
    }

    /// Set reasoning effort level.
    pub fn effort(mut self, effort: Effort) -> Self {
        self.options.effort = Some(effort);
        self
    }

    /// Build the options.
    pub fn build(self) -> LlmOptions {
        self.options
    }
}

/// Response from an LLM proxy query.
#[derive(Debug, Clone)]
pub struct LlmResponse {
    /// The full text response.
    pub text: String,
    /// Model that generated the response.
    pub model: Option<String>,
    /// CLI session ID.
    pub session_id: Option<String>,
    /// Stop reason (e.g., `"end_turn"`, `"max_tokens"`).
    pub stop_reason: Option<String>,
    /// Raw usage/cost data.
    pub usage: Option<serde_json::Value>,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Send a prompt and get the full text response.
///
/// This is the simplest way to use Claude through your CC subscription.
/// All agent features (tools, system prompt, hooks) are disabled by default.
///
/// # Example
///
/// ```rust,no_run
/// # async fn example() -> cc_sdk::Result<()> {
/// let response = cc_sdk::llm::query("What is 2 + 2?", None).await?;
/// assert!(response.text.contains("4"));
/// # Ok(())
/// # }
/// ```
pub async fn query(prompt: &str, options: Option<LlmOptions>) -> Result<LlmResponse> {
    let opts = options.unwrap_or_default();
    let cc_options = opts.to_claude_code_options();

    let stream = crate::query::query(prompt, Some(cc_options)).await?;
    futures::pin_mut!(stream);

    let mut text_parts: Vec<String> = Vec::new();
    let mut model: Option<String> = None;
    let mut session_id: Option<String> = None;
    let mut stop_reason: Option<String> = None;
    let mut usage: Option<serde_json::Value> = None;

    while let Some(msg_result) = stream.next().await {
        match msg_result? {
            Message::Assistant { message } => {
                if model.is_none() {
                    model.clone_from(&message.model);
                }
                for block in &message.content {
                    if let ContentBlock::Text(text_content) = block {
                        text_parts.push(text_content.text.clone());
                    }
                }
            }
            Message::Result {
                result,
                session_id: sid,
                stop_reason: sr,
                usage: u,
                ..
            } => {
                session_id = Some(sid);
                stop_reason = sr;
                usage = u;
                if text_parts.is_empty() {
                    if let Some(r) = result {
                        text_parts.push(r);
                    }
                }
            }
            _ => {}
        }
    }

    Ok(LlmResponse {
        text: text_parts.join(""),
        model,
        session_id,
        stop_reason,
        usage,
    })
}

/// Send a prompt and get a stream of text chunks.
///
/// Each item is an incremental text delta (not cumulative).
/// Useful for real-time display of streaming responses.
///
/// # Example
///
/// ```rust,no_run
/// use futures::StreamExt;
///
/// # async fn example() -> cc_sdk::Result<()> {
/// let mut stream = cc_sdk::llm::query_stream("Write a poem", None).await?;
/// while let Some(chunk) = stream.next().await {
///     print!("{}", chunk?);
/// }
/// # Ok(())
/// # }
/// ```
pub async fn query_stream(
    prompt: &str,
    options: Option<LlmOptions>,
) -> Result<impl Stream<Item = Result<String>>> {
    let opts = options.unwrap_or_default();
    let cc_options = opts.to_claude_code_options();

    let stream = crate::query::query(prompt, Some(cc_options)).await?;

    // Use scan to track cumulative text length and emit only new deltas.
    // CC's stream-json mode may emit cumulative assistant messages.
    Ok(stream
        .scan(0usize, |seen_len, msg_result| {
            let result = match msg_result {
                Ok(Message::Assistant { message }) => {
                    let full_text: String = message
                        .content
                        .iter()
                        .filter_map(|block| match block {
                            ContentBlock::Text(t) => Some(t.text.as_str()),
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                        .join("");

                    if full_text.len() > *seen_len {
                        let delta = full_text[*seen_len..].to_string();
                        *seen_len = full_text.len();
                        Some(Ok(delta))
                    } else {
                        None
                    }
                }
                Err(e) => Some(Err(e)),
                _ => None,
            };
            futures::future::ready(Some(result))
        })
        .filter_map(|x| futures::future::ready(x)))
}

// ---------------------------------------------------------------------------
// Helpers (internal)
// ---------------------------------------------------------------------------

/// Extract text from a Message, used for testing.
#[cfg(test)]
fn extract_text_from_messages(messages: &[Message]) -> String {
    let mut parts = Vec::new();
    for msg in messages {
        if let Message::Assistant { message } = msg {
            for block in &message.content {
                if let ContentBlock::Text(t) = block {
                    parts.push(t.text.clone());
                }
            }
        }
    }
    parts.join("")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AssistantMessage, TextContent};

    #[test]
    fn test_default_llm_options() {
        let opts = LlmOptions::default();
        assert!(opts.system_prompt.is_none());
        assert!(opts.model.is_none());
        assert!(opts.thinking.is_none());
        assert!(opts.max_turns.is_none()); // None in struct, but to_cc_options defaults to 1
        assert!(opts.max_output_tokens.is_none());
        assert!(opts.effort.is_none());
    }

    #[test]
    fn test_llm_options_builder() {
        let opts = LlmOptions::builder()
            .system_prompt("You are helpful")
            .model("claude-sonnet-4-20250514")
            .max_turns(3)
            .max_output_tokens(4096)
            .effort(Effort::High)
            .build();

        assert_eq!(opts.system_prompt, Some("You are helpful".to_string()));
        assert_eq!(opts.model, Some("claude-sonnet-4-20250514".to_string()));
        assert_eq!(opts.max_turns, Some(3));
        assert_eq!(opts.max_output_tokens, Some(4096));
        assert_eq!(opts.effort, Some(Effort::High));
    }

    #[test]
    fn test_llm_options_to_cc_options_defaults() {
        let opts = LlmOptions::default();
        let cc = opts.to_claude_code_options();

        // DontAsk permission mode
        assert_eq!(cc.permission_mode, PermissionMode::DontAsk);

        // Max turns defaults to 1
        assert_eq!(cc.max_turns, Some(1));

        // System prompt is empty string (strips CC agent prompt)
        match cc.system_prompt_v2 {
            Some(SystemPrompt::String(s)) => assert_eq!(s, ""),
            _ => panic!("Expected empty string system prompt"),
        }

        // --bare and --tools "" in extra_args
        assert_eq!(cc.extra_args.get("bare"), Some(&None));
        assert_eq!(
            cc.extra_args.get("tools"),
            Some(&Some(String::new()))
        );

        // Model not set
        assert!(cc.model.is_none());
    }

    #[test]
    fn test_llm_options_to_cc_options_custom() {
        let opts = LlmOptions::builder()
            .system_prompt("Custom prompt")
            .model("claude-opus-4-20250514")
            .max_turns(5)
            .build();
        let cc = opts.to_claude_code_options();

        match cc.system_prompt_v2 {
            Some(SystemPrompt::String(s)) => assert_eq!(s, "Custom prompt"),
            _ => panic!("Expected custom system prompt"),
        }
        assert_eq!(cc.model, Some("claude-opus-4-20250514".to_string()));
        assert_eq!(cc.max_turns, Some(5));
    }

    #[test]
    fn test_text_extraction() {
        let messages = vec![
            Message::Assistant {
                message: AssistantMessage {
                    content: vec![
                        ContentBlock::Text(TextContent {
                            text: "Hello ".to_string(),
                        }),
                        ContentBlock::Text(TextContent {
                            text: "world!".to_string(),
                        }),
                    ],
                    model: Some("claude-sonnet".to_string()),
                    usage: None,
                    error: None,
                    parent_tool_use_id: None,
                },
            },
            Message::System {
                subtype: "status".to_string(),
                data: serde_json::json!({}),
            },
        ];

        let text = extract_text_from_messages(&messages);
        assert_eq!(text, "Hello world!");
    }

    #[test]
    fn test_text_extraction_ignores_non_text_blocks() {
        let messages = vec![Message::Assistant {
            message: AssistantMessage {
                content: vec![
                    ContentBlock::Thinking(crate::types::ThinkingContent {
                        thinking: "internal reasoning".to_string(),
                        signature: String::new(),
                    }),
                    ContentBlock::Text(TextContent {
                        text: "visible answer".to_string(),
                    }),
                ],
                model: None,
                usage: None,
                error: None,
                parent_tool_use_id: None,
            },
        }];

        let text = extract_text_from_messages(&messages);
        assert_eq!(text, "visible answer");
    }

    #[test]
    fn test_stream_delta_dedup() {
        // Simulate cumulative assistant messages (each contains full text so far)
        let messages: Vec<crate::errors::Result<Message>> = vec![
            Ok(Message::Assistant {
                message: AssistantMessage {
                    content: vec![ContentBlock::Text(TextContent {
                        text: "Hel".to_string(),
                    })],
                    model: None,
                    usage: None,
                    error: None,
                    parent_tool_use_id: None,
                },
            }),
            Ok(Message::Assistant {
                message: AssistantMessage {
                    content: vec![ContentBlock::Text(TextContent {
                        text: "Hello wo".to_string(),
                    })],
                    model: None,
                    usage: None,
                    error: None,
                    parent_tool_use_id: None,
                },
            }),
            Ok(Message::Assistant {
                message: AssistantMessage {
                    content: vec![ContentBlock::Text(TextContent {
                        text: "Hello world!".to_string(),
                    })],
                    model: None,
                    usage: None,
                    error: None,
                    parent_tool_use_id: None,
                },
            }),
        ];

        // Simulate the scan logic
        let mut seen_len = 0usize;
        let mut deltas = Vec::new();

        for msg_result in messages {
            if let Ok(Message::Assistant { message }) = msg_result {
                let full_text: String = message
                    .content
                    .iter()
                    .filter_map(|block| match block {
                        ContentBlock::Text(t) => Some(t.text.as_str()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("");

                if full_text.len() > seen_len {
                    deltas.push(full_text[seen_len..].to_string());
                    seen_len = full_text.len();
                }
            }
        }

        assert_eq!(deltas, vec!["Hel", "lo wo", "rld!"]);
        assert_eq!(deltas.join(""), "Hello world!");
    }

    #[test]
    fn test_llm_response_fields() {
        let resp = LlmResponse {
            text: "test".to_string(),
            model: Some("claude-sonnet".to_string()),
            session_id: Some("sess-123".to_string()),
            stop_reason: Some("end_turn".to_string()),
            usage: Some(serde_json::json!({"input_tokens": 10, "output_tokens": 20})),
        };
        assert_eq!(resp.text, "test");
        assert_eq!(resp.model.as_deref(), Some("claude-sonnet"));
        assert_eq!(resp.stop_reason.as_deref(), Some("end_turn"));
    }
}
