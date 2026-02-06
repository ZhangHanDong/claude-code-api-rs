//! Error types for the Claude Code SDK
//!
//! This module defines all error types that can occur when using the SDK.
//! The errors are designed to be informative and actionable, helping users
//! understand what went wrong and how to fix it.

use thiserror::Error;

/// Main error type for the Claude Code SDK
#[derive(Error, Debug)]
pub enum SdkError {
    /// Claude CLI executable was not found
    #[error(
        "Claude CLI not found. Install with: npm install -g @anthropic-ai/claude-code\n\nSearched in:\n{searched_paths}"
    )]
    CliNotFound {
        /// Paths that were searched for the CLI
        searched_paths: String,
    },

    /// Failed to connect to Claude CLI
    #[error("Failed to connect to Claude CLI: {0}")]
    ConnectionError(String),

    /// Process-related errors
    #[error("Process error: {0}")]
    ProcessError(#[from] std::io::Error),

    /// Failed to parse a message
    #[error("Failed to parse message: {error}\nRaw message: {raw}")]
    MessageParseError {
        /// Parse error description
        error: String,
        /// Raw message that failed to parse
        raw: String,
    },

    /// JSON serialization/deserialization errors
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    /// CLI JSON decode error
    #[error("Failed to decode JSON from CLI output: {line}")]
    CliJsonDecodeError {
        /// Line that failed to decode
        line: String,
        /// Original error
        #[source]
        original_error: serde_json::Error,
    },

    /// Transport layer errors
    #[error("Transport error: {0}")]
    TransportError(String),

    /// Timeout waiting for response
    #[error("Timeout waiting for response after {seconds} seconds")]
    Timeout {
        /// Number of seconds waited before timeout
        seconds: u64,
    },

    /// Session not found
    #[error("Session not found: {0}")]
    SessionNotFound(String),

    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    ConfigError(String),

    /// Control request failed
    #[error("Control request failed: {0}")]
    ControlRequestError(String),

    /// Unexpected response type
    #[error("Unexpected response type: expected {expected}, got {actual}")]
    UnexpectedResponse {
        /// Expected response type
        expected: String,
        /// Actual response type received
        actual: String,
    },

    /// CLI returned an error
    #[error("Claude CLI error: {message}")]
    CliError {
        /// Error message from CLI
        message: String,
        /// Error code if available
        code: Option<String>,
    },

    /// Channel send error
    #[error("Failed to send message through channel")]
    ChannelSendError,

    /// Channel receive error
    #[error("Channel closed unexpectedly")]
    ChannelClosed,

    /// Invalid state transition
    #[error("Invalid state: {message}")]
    InvalidState {
        /// Description of the invalid state
        message: String,
    },

    /// Process exited unexpectedly
    #[error("Claude process exited unexpectedly with code {code:?}")]
    ProcessExited {
        /// Exit code if available
        code: Option<i32>,
    },

    /// Stream ended unexpectedly
    #[error("Stream ended unexpectedly")]
    UnexpectedStreamEnd,

    /// Feature not supported
    #[error("Feature not supported: {feature}")]
    NotSupported {
        /// Description of unsupported feature
        feature: String,
    },
}

/// Result type alias for SDK operations
pub type Result<T> = std::result::Result<T, SdkError>;

impl SdkError {
    /// Create a new MessageParseError
    pub fn parse_error(error: impl Into<String>, raw: impl Into<String>) -> Self {
        Self::MessageParseError {
            error: error.into(),
            raw: raw.into(),
        }
    }

    /// Create a new Timeout error
    pub fn timeout(seconds: u64) -> Self {
        Self::Timeout { seconds }
    }

    /// Create a new UnexpectedResponse error
    pub fn unexpected_response(expected: impl Into<String>, actual: impl Into<String>) -> Self {
        Self::UnexpectedResponse {
            expected: expected.into(),
            actual: actual.into(),
        }
    }

    /// Create a new CliError
    pub fn cli_error(message: impl Into<String>, code: Option<String>) -> Self {
        Self::CliError {
            message: message.into(),
            code,
        }
    }

    /// Create a new InvalidState error
    pub fn invalid_state(message: impl Into<String>) -> Self {
        Self::InvalidState {
            message: message.into(),
        }
    }

    /// Check if the error is recoverable
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            Self::Timeout { .. }
                | Self::ChannelClosed
                | Self::UnexpectedStreamEnd
                | Self::ProcessExited { .. }
        )
    }

    /// Check if the error is a configuration issue
    pub fn is_config_error(&self) -> bool {
        matches!(
            self,
            Self::CliNotFound { .. } | Self::ConfigError(_) | Self::NotSupported { .. }
        )
    }
}

// Implement From for common channel errors
impl<T> From<tokio::sync::mpsc::error::SendError<T>> for SdkError {
    fn from(_: tokio::sync::mpsc::error::SendError<T>) -> Self {
        Self::ChannelSendError
    }
}

impl From<tokio::sync::broadcast::error::RecvError> for SdkError {
    fn from(_: tokio::sync::broadcast::error::RecvError) -> Self {
        Self::ChannelClosed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = SdkError::CliNotFound {
            searched_paths: "/usr/local/bin\n/usr/bin".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("npm install -g @anthropic-ai/claude-code"));
        assert!(msg.contains("/usr/local/bin"));
    }

    #[test]
    fn test_is_recoverable() {
        assert!(SdkError::timeout(30).is_recoverable());
        assert!(SdkError::ChannelClosed.is_recoverable());
        assert!(!SdkError::ConfigError("test".into()).is_recoverable());
    }

    #[test]
    fn test_is_config_error() {
        assert!(SdkError::ConfigError("test".into()).is_config_error());
        assert!(
            SdkError::CliNotFound {
                searched_paths: "test".into()
            }
            .is_config_error()
        );
        assert!(!SdkError::timeout(30).is_config_error());
    }

    #[test]
    fn test_cli_json_decode_error() {
        let line = r#"{"invalid": json"#.to_string();
        let original_err = serde_json::from_str::<serde_json::Value>(&line).unwrap_err();

        let error = SdkError::CliJsonDecodeError {
            line: line.clone(),
            original_error: original_err,
        };

        let error_str = error.to_string();
        assert!(error_str.contains("Failed to decode JSON from CLI output"));
        assert!(error_str.contains(&line));
    }
}
