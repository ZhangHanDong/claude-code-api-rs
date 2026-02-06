//! Tool context extraction from Claude Code tool calls.
//!
//! This module provides utilities to extract contextual information
//! (files touched, working directory) from tool call inputs.

use serde_json::Value;
use std::collections::HashSet;
use std::path::Path;

/// Context extracted from a single tool call.
#[derive(Debug, Clone, Default)]
pub struct ToolContext {
    /// Files extracted from this tool call
    pub files: Vec<String>,

    /// Working directory detected from this tool call
    pub cwd: Option<String>,
}

impl ToolContext {
    /// Creates a new empty ToolContext.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a ToolContext with a single file.
    pub fn with_file(file: impl Into<String>) -> Self {
        Self {
            files: vec![file.into()],
            cwd: None,
        }
    }

    /// Creates a ToolContext with a working directory.
    pub fn with_cwd(cwd: impl Into<String>) -> Self {
        Self {
            files: Vec::new(),
            cwd: Some(cwd.into()),
        }
    }

    /// Merges another context into this one.
    pub fn merge(&mut self, other: ToolContext) {
        for file in other.files {
            if !self.files.contains(&file) {
                self.files.push(file);
            }
        }
        if self.cwd.is_none() {
            self.cwd = other.cwd;
        }
    }

    /// Returns true if this context contains any useful information.
    pub fn is_empty(&self) -> bool {
        self.files.is_empty() && self.cwd.is_none()
    }
}

/// Trait for extracting context from tool calls.
pub trait ToolContextExtractor {
    /// Extracts context from a tool call.
    ///
    /// # Arguments
    /// * `tool_name` - The name of the tool (e.g., "Read", "Write", "Bash")
    /// * `input` - The JSON input of the tool call
    ///
    /// # Returns
    /// A `ToolContext` containing extracted files and/or cwd.
    fn extract_context(&self, tool_name: &str, input: &Value) -> ToolContext;
}

/// Default implementation of ToolContextExtractor.
///
/// Supports extraction from:
/// - Read, Write, Edit: `file_path` field
/// - Glob, Grep: `path` field
/// - Bash: detects `cd` commands and absolute paths
#[derive(Debug, Clone, Default)]
pub struct DefaultToolContextExtractor;

impl DefaultToolContextExtractor {
    /// Creates a new DefaultToolContextExtractor.
    pub fn new() -> Self {
        Self
    }

    /// Extracts file_path from Read/Write/Edit tool inputs.
    fn extract_file_path(&self, input: &Value) -> Option<String> {
        input
            .get("file_path")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    /// Extracts path from Glob/Grep tool inputs.
    fn extract_path(&self, input: &Value) -> Option<String> {
        input
            .get("path")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    /// Extracts cwd and files from Bash commands.
    fn extract_from_bash(&self, input: &Value) -> ToolContext {
        let command = match input.get("command").and_then(|v| v.as_str()) {
            Some(cmd) => cmd,
            None => return ToolContext::new(),
        };

        let mut context = ToolContext::new();

        // Detect cd commands
        let cwd_path = self.extract_cd_path(command);
        if let Some(ref cwd) = cwd_path {
            context.cwd = Some(cwd.clone());
        }

        // Extract absolute file paths from the command
        // Exclude the cd target path from files
        for path in self.extract_absolute_paths(command) {
            // Skip if this is the cd target
            if let Some(ref cwd) = cwd_path
                && &path == cwd
            {
                continue;
            }
            if !context.files.contains(&path) {
                context.files.push(path);
            }
        }

        context
    }

    /// Extracts the path from a `cd` command.
    fn extract_cd_path(&self, command: &str) -> Option<String> {
        // Match patterns like: cd /path, cd "/path", cd '/path'
        // Also handle: cd /path && other commands
        let command = command.trim();

        // Check if command starts with cd
        if !command.starts_with("cd ") {
            // Check for cd in a chain: something && cd /path
            if let Some(idx) = command.find("&& cd ") {
                let rest = &command[idx + 6..];
                return self.parse_cd_argument(rest);
            }
            return None;
        }

        let rest = &command[3..]; // Skip "cd "
        self.parse_cd_argument(rest)
    }

    /// Parses the argument to a cd command.
    fn parse_cd_argument(&self, arg: &str) -> Option<String> {
        let arg = arg.trim();

        if arg.is_empty() {
            return None;
        }

        // Handle quoted paths
        let path = if arg.starts_with('"') {
            arg.trim_start_matches('"').split('"').next().unwrap_or("")
        } else if arg.starts_with('\'') {
            arg.trim_start_matches('\'')
                .split('\'')
                .next()
                .unwrap_or("")
        } else {
            // Unquoted: take until space or special chars
            arg.split(|c: char| c.is_whitespace() || c == '&' || c == '|' || c == ';')
                .next()
                .unwrap_or("")
        };

        // Only return absolute paths
        if path.starts_with('/') {
            Some(path.to_string())
        } else {
            None
        }
    }

    /// Extracts absolute file paths from a command string.
    fn extract_absolute_paths(&self, command: &str) -> Vec<String> {
        let mut paths = Vec::new();
        let mut current_path = String::new();
        let mut in_path = false;

        for c in command.chars() {
            if c == '/' && !in_path {
                in_path = true;
                current_path.push(c);
            } else if in_path {
                if c.is_whitespace()
                    || c == '"'
                    || c == '\''
                    || c == '&'
                    || c == '|'
                    || c == ';'
                    || c == ')'
                    || c == '('
                {
                    // End of path
                    if current_path.len() > 1 && self.looks_like_file_path(&current_path) {
                        paths.push(current_path.clone());
                    }
                    current_path.clear();
                    in_path = false;
                } else {
                    current_path.push(c);
                }
            }
        }

        // Don't forget the last path if command ends with it
        if in_path && current_path.len() > 1 && self.looks_like_file_path(&current_path) {
            paths.push(current_path);
        }

        paths
    }

    /// Heuristic to determine if a string looks like a file path.
    fn looks_like_file_path(&self, s: &str) -> bool {
        // Must start with /
        if !s.starts_with('/') {
            return false;
        }

        // Should not be just common directories
        let common_dirs = ["/", "/tmp", "/dev", "/proc", "/sys", "/usr", "/bin", "/etc"];
        if common_dirs.contains(&s) {
            return false;
        }

        // Should contain at least one path separator after the root
        let has_depth = s.chars().skip(1).any(|c| c == '/');

        // Should not end with common command flags
        let looks_like_flag = s.ends_with("--") || s.contains("=-");

        has_depth && !looks_like_flag
    }
}

impl ToolContextExtractor for DefaultToolContextExtractor {
    fn extract_context(&self, tool_name: &str, input: &Value) -> ToolContext {
        match tool_name {
            // File operations
            "Read" | "Write" | "Edit" => {
                if let Some(file) = self.extract_file_path(input) {
                    ToolContext::with_file(file)
                } else {
                    ToolContext::new()
                }
            },

            // Search operations
            "Glob" | "Grep" => {
                if let Some(path) = self.extract_path(input) {
                    // For directories, we just note the cwd context
                    if Path::new(&path).is_dir() || !path.contains('.') {
                        ToolContext::with_cwd(path)
                    } else {
                        ToolContext::with_file(path)
                    }
                } else {
                    ToolContext::new()
                }
            },

            // Shell commands
            "Bash" => self.extract_from_bash(input),

            // Unknown tools
            _ => ToolContext::new(),
        }
    }
}

/// Aggregates context from multiple tool calls within a conversation turn.
#[derive(Debug, Clone, Default)]
pub struct MessageContextAggregator {
    /// All files touched in this turn
    files: HashSet<String>,

    /// Current working directory
    cwd: Option<String>,

    /// The extractor to use
    extractor: DefaultToolContextExtractor,
}

impl MessageContextAggregator {
    /// Creates a new MessageContextAggregator.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new aggregator with an initial cwd.
    pub fn with_initial_cwd(cwd: impl Into<String>) -> Self {
        Self {
            cwd: Some(cwd.into()),
            ..Default::default()
        }
    }

    /// Processes a tool call and accumulates context.
    pub fn process_tool_call(&mut self, tool_name: &str, input: &Value) {
        let context = self.extractor.extract_context(tool_name, input);

        // Update cwd if detected
        if let Some(new_cwd) = context.cwd {
            self.cwd = Some(new_cwd);
        }

        // Accumulate files
        for file in context.files {
            self.files.insert(file);
        }
    }

    /// Returns the aggregated files as a sorted vector.
    pub fn files(&self) -> Vec<String> {
        let mut files: Vec<_> = self.files.iter().cloned().collect();
        files.sort();
        files
    }

    /// Returns the current working directory.
    pub fn cwd(&self) -> Option<&str> {
        self.cwd.as_deref()
    }

    /// Resets the aggregator for a new turn.
    pub fn reset(&mut self) {
        self.files.clear();
        // Keep cwd as it persists across turns
    }

    /// Finalizes and returns a ToolContext with all accumulated data.
    pub fn finalize(&self) -> ToolContext {
        ToolContext {
            files: self.files(),
            cwd: self.cwd.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_extract_read_file_path() {
        let extractor = DefaultToolContextExtractor::new();
        let input = json!({
            "file_path": "/Users/dev/project/src/main.rs"
        });

        let context = extractor.extract_context("Read", &input);

        assert_eq!(context.files, vec!["/Users/dev/project/src/main.rs"]);
        assert!(context.cwd.is_none());
    }

    #[test]
    fn test_extract_write_file_path() {
        let extractor = DefaultToolContextExtractor::new();
        let input = json!({
            "file_path": "/tmp/test.txt",
            "content": "Hello"
        });

        let context = extractor.extract_context("Write", &input);

        assert_eq!(context.files, vec!["/tmp/test.txt"]);
    }

    #[test]
    fn test_extract_edit_file_path() {
        let extractor = DefaultToolContextExtractor::new();
        let input = json!({
            "file_path": "/projects/api/src/auth.rs",
            "old_string": "fn old()",
            "new_string": "fn new()"
        });

        let context = extractor.extract_context("Edit", &input);

        assert_eq!(context.files, vec!["/projects/api/src/auth.rs"]);
    }

    #[test]
    fn test_extract_glob_path() {
        let extractor = DefaultToolContextExtractor::new();
        let input = json!({
            "pattern": "**/*.rs",
            "path": "/projects/my-app/src"
        });

        let context = extractor.extract_context("Glob", &input);

        // Directory path becomes cwd context
        assert_eq!(context.cwd, Some("/projects/my-app/src".to_string()));
    }

    #[test]
    fn test_extract_grep_path() {
        let extractor = DefaultToolContextExtractor::new();
        let input = json!({
            "pattern": "TODO",
            "path": "/projects/backend"
        });

        let context = extractor.extract_context("Grep", &input);

        assert_eq!(context.cwd, Some("/projects/backend".to_string()));
    }

    #[test]
    fn test_extract_bash_cd_simple() {
        let extractor = DefaultToolContextExtractor::new();
        let input = json!({
            "command": "cd /projects/my-app"
        });

        let context = extractor.extract_context("Bash", &input);

        assert_eq!(context.cwd, Some("/projects/my-app".to_string()));
    }

    #[test]
    fn test_extract_bash_cd_quoted() {
        let extractor = DefaultToolContextExtractor::new();
        let input = json!({
            "command": "cd \"/projects/my app with spaces\""
        });

        let context = extractor.extract_context("Bash", &input);

        assert_eq!(
            context.cwd,
            Some("/projects/my app with spaces".to_string())
        );
    }

    #[test]
    fn test_extract_bash_cd_chained() {
        let extractor = DefaultToolContextExtractor::new();
        let input = json!({
            "command": "git status && cd /new/directory && ls"
        });

        let context = extractor.extract_context("Bash", &input);

        assert_eq!(context.cwd, Some("/new/directory".to_string()));
    }

    #[test]
    fn test_extract_bash_absolute_paths() {
        let extractor = DefaultToolContextExtractor::new();
        let input = json!({
            "command": "cat /etc/config/app.conf && cp /src/main.rs /dst/main.rs"
        });

        let context = extractor.extract_context("Bash", &input);

        assert!(context.files.contains(&"/etc/config/app.conf".to_string()));
        assert!(context.files.contains(&"/src/main.rs".to_string()));
        assert!(context.files.contains(&"/dst/main.rs".to_string()));
    }

    #[test]
    fn test_extract_bash_ignores_common_dirs() {
        let extractor = DefaultToolContextExtractor::new();
        let input = json!({
            "command": "ls /tmp"
        });

        let context = extractor.extract_context("Bash", &input);

        // /tmp alone should not be extracted as a file
        assert!(context.files.is_empty());
    }

    #[test]
    fn test_unknown_tool() {
        let extractor = DefaultToolContextExtractor::new();
        let input = json!({
            "whatever": "value"
        });

        let context = extractor.extract_context("UnknownTool", &input);

        assert!(context.is_empty());
    }

    #[test]
    fn test_aggregator_multiple_tools() {
        let mut aggregator = MessageContextAggregator::new();

        // Simulate a turn with multiple tool calls
        aggregator.process_tool_call(
            "Read",
            &json!({
                "file_path": "/src/main.rs"
            }),
        );
        aggregator.process_tool_call(
            "Edit",
            &json!({
                "file_path": "/src/lib.rs",
                "old_string": "a",
                "new_string": "b"
            }),
        );
        aggregator.process_tool_call(
            "Bash",
            &json!({
                "command": "cd /projects/app && cargo build"
            }),
        );
        aggregator.process_tool_call(
            "Read",
            &json!({
                "file_path": "/src/main.rs"  // Duplicate
            }),
        );

        let files = aggregator.files();
        assert_eq!(files.len(), 2); // Deduplicated
        assert!(files.contains(&"/src/lib.rs".to_string()));
        assert!(files.contains(&"/src/main.rs".to_string()));
        assert_eq!(aggregator.cwd(), Some("/projects/app"));
    }

    #[test]
    fn test_aggregator_with_initial_cwd() {
        let aggregator = MessageContextAggregator::with_initial_cwd("/initial/path");

        assert_eq!(aggregator.cwd(), Some("/initial/path"));
    }

    #[test]
    fn test_aggregator_cwd_update() {
        let mut aggregator = MessageContextAggregator::with_initial_cwd("/old/path");

        aggregator.process_tool_call(
            "Bash",
            &json!({
                "command": "cd /new/path"
            }),
        );

        assert_eq!(aggregator.cwd(), Some("/new/path"));
    }

    #[test]
    fn test_aggregator_reset() {
        let mut aggregator = MessageContextAggregator::with_initial_cwd("/projects");
        aggregator.process_tool_call(
            "Read",
            &json!({
                "file_path": "/src/main.rs"
            }),
        );

        aggregator.reset();

        assert!(aggregator.files().is_empty());
        assert_eq!(aggregator.cwd(), Some("/projects")); // cwd persists
    }

    #[test]
    fn test_aggregator_finalize() {
        let mut aggregator = MessageContextAggregator::with_initial_cwd("/projects");
        aggregator.process_tool_call(
            "Read",
            &json!({
                "file_path": "/src/main.rs"
            }),
        );

        let context = aggregator.finalize();

        assert_eq!(context.files, vec!["/src/main.rs"]);
        assert_eq!(context.cwd, Some("/projects".to_string()));
    }

    #[test]
    fn test_tool_context_merge() {
        let mut ctx1 = ToolContext::with_file("/file1.rs");
        let ctx2 = ToolContext {
            files: vec!["/file2.rs".to_string(), "/file1.rs".to_string()],
            cwd: Some("/projects".to_string()),
        };

        ctx1.merge(ctx2);

        assert_eq!(ctx1.files.len(), 2); // Deduplicated
        assert_eq!(ctx1.cwd, Some("/projects".to_string()));
    }
}
