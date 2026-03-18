//! Session history API
//!
//! Provides functions to list, query, and manage Claude Code conversation sessions.

use crate::errors::{Result, SdkError};
use serde::{Deserialize, Serialize};

/// Information about a stored session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    /// Session ID
    pub session_id: String,
    /// Session summary text
    #[serde(default)]
    pub summary: String,
    /// Last modified timestamp (ms since epoch)
    #[serde(default)]
    pub last_modified: i64,
    /// File size in bytes
    #[serde(default)]
    pub file_size: u64,
    /// Custom title set by user
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_title: Option<String>,
    /// First prompt in the session
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_prompt: Option<String>,
    /// Git branch when session was created
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git_branch: Option<String>,
    /// Working directory
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
}

/// A single message from a session's history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMessage {
    /// Message type (user, assistant, system, result)
    #[serde(rename = "type")]
    pub msg_type: String,
    /// Unique message ID
    #[serde(default)]
    pub uuid: String,
    /// Session ID
    #[serde(default)]
    pub session_id: String,
    /// Full message data
    pub message: serde_json::Value,
}

/// Sanitize Unicode by removing zero-width and invisible characters
fn sanitize_unicode(input: &str) -> String {
    input
        .chars()
        .filter(|c| {
            !matches!(
                c,
                '\u{200B}' | '\u{200C}' | '\u{200D}' | '\u{FEFF}' | '\u{00AD}'
            )
        })
        .collect()
}

/// List available sessions
///
/// # Arguments
/// * `directory` - Optional working directory filter
/// * `limit` - Maximum number of sessions to return
/// * `include_worktrees` - Whether to include worktree sessions (default: true)
pub async fn list_sessions(
    directory: Option<&str>,
    limit: Option<usize>,
    include_worktrees: bool,
) -> Result<Vec<SessionInfo>> {
    let cli_path = crate::transport::subprocess::find_claude_cli()?;
    let mut cmd = tokio::process::Command::new(&cli_path);

    cmd.arg("sessions").arg("list").arg("--json");

    if let Some(dir) = directory {
        cmd.arg("--directory").arg(dir);
    }

    if let Some(limit) = limit {
        cmd.arg("--limit").arg(limit.to_string());
    }

    if !include_worktrees {
        cmd.arg("--no-worktrees");
    }

    let output = cmd.output().await.map_err(SdkError::ProcessError)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(SdkError::ConnectionError(format!(
            "Failed to list sessions: {stderr}"
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let sanitized = sanitize_unicode(&stdout);

    serde_json::from_str(&sanitized).map_err(|e| {
        SdkError::parse_error(
            format!("Failed to parse session list: {e}"),
            sanitized,
        )
    })
}

/// Get messages from a specific session
///
/// # Arguments
/// * `session_id` - The session ID to query
/// * `directory` - Optional working directory context
/// * `limit` - Maximum number of messages to return
/// * `offset` - Number of messages to skip from the beginning
pub async fn get_session_messages(
    session_id: &str,
    directory: Option<&str>,
    limit: Option<usize>,
    offset: usize,
) -> Result<Vec<SessionMessage>> {
    let cli_path = crate::transport::subprocess::find_claude_cli()?;
    let mut cmd = tokio::process::Command::new(&cli_path);

    cmd.arg("sessions")
        .arg("messages")
        .arg("--session-id")
        .arg(session_id)
        .arg("--json");

    if let Some(dir) = directory {
        cmd.arg("--directory").arg(dir);
    }

    if let Some(limit) = limit {
        cmd.arg("--limit").arg(limit.to_string());
    }

    if offset > 0 {
        cmd.arg("--offset").arg(offset.to_string());
    }

    let output = cmd.output().await.map_err(SdkError::ProcessError)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(SdkError::ConnectionError(format!(
            "Failed to get session messages: {stderr}"
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let sanitized = sanitize_unicode(&stdout);

    serde_json::from_str(&sanitized).map_err(|e| {
        SdkError::parse_error(
            format!("Failed to parse session messages: {e}"),
            sanitized,
        )
    })
}

/// Rename a session with a custom title
///
/// # Arguments
/// * `session_id` - The session ID to rename
/// * `title` - The new title for the session
pub async fn rename_session(session_id: &str, title: &str) -> Result<()> {
    let cli_path = crate::transport::subprocess::find_claude_cli()?;
    let mut cmd = tokio::process::Command::new(&cli_path);

    cmd.arg("sessions")
        .arg("rename")
        .arg("--session-id")
        .arg(session_id)
        .arg("--title")
        .arg(title);

    let output = cmd.output().await.map_err(SdkError::ProcessError)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(SdkError::ConnectionError(format!(
            "Failed to rename session: {stderr}"
        )));
    }

    Ok(())
}

/// Tag a session with a label
///
/// # Arguments
/// * `session_id` - The session ID to tag
/// * `tag` - The tag to apply, or None to clear the tag
pub async fn tag_session(session_id: &str, tag: Option<&str>) -> Result<()> {
    let cli_path = crate::transport::subprocess::find_claude_cli()?;
    let mut cmd = tokio::process::Command::new(&cli_path);

    cmd.arg("sessions")
        .arg("tag")
        .arg("--session-id")
        .arg(session_id);

    if let Some(tag) = tag {
        cmd.arg("--tag").arg(tag);
    } else {
        cmd.arg("--clear");
    }

    let output = cmd.output().await.map_err(SdkError::ProcessError)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(SdkError::ConnectionError(format!(
            "Failed to tag session: {stderr}"
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_unicode() {
        assert_eq!(sanitize_unicode("hello\u{200B}world"), "helloworld");
        assert_eq!(sanitize_unicode("no\u{FEFF}bom"), "nobom");
        assert_eq!(sanitize_unicode("clean text"), "clean text");
        assert_eq!(sanitize_unicode("\u{200C}\u{200D}"), "");
        assert_eq!(sanitize_unicode("soft\u{00AD}hyphen"), "softhyphen");
    }

    #[test]
    fn test_session_info_deserialize() {
        let json = serde_json::json!({
            "session_id": "sess-123",
            "summary": "Test session",
            "last_modified": 1710000000000_i64,
            "file_size": 4096,
            "custom_title": "My Session",
            "first_prompt": "Hello",
            "git_branch": "main",
            "cwd": "/tmp"
        });

        let info: SessionInfo = serde_json::from_value(json).unwrap();
        assert_eq!(info.session_id, "sess-123");
        assert_eq!(info.custom_title.as_deref(), Some("My Session"));
        assert_eq!(info.git_branch.as_deref(), Some("main"));
    }

    #[test]
    fn test_session_info_minimal() {
        let json = serde_json::json!({
            "session_id": "sess-min"
        });

        let info: SessionInfo = serde_json::from_value(json).unwrap();
        assert_eq!(info.session_id, "sess-min");
        assert_eq!(info.summary, "");
        assert_eq!(info.last_modified, 0);
        assert!(info.custom_title.is_none());
    }

    #[test]
    fn test_session_message_deserialize() {
        let json = serde_json::json!({
            "type": "user",
            "uuid": "uuid-1",
            "session_id": "sess-1",
            "message": {"content": "hello"}
        });

        let msg: SessionMessage = serde_json::from_value(json).unwrap();
        assert_eq!(msg.msg_type, "user");
        assert_eq!(msg.uuid, "uuid-1");
        assert_eq!(msg.message["content"], "hello");
    }
}
