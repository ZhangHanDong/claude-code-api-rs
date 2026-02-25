//! NDJSON (Newline-Delimited JSON) helpers
//!
//! The Claude Code CLI WebSocket protocol uses NDJSON — each message is
//! a single JSON object terminated by `\n`. Multiple messages may arrive
//! in a single WebSocket frame.

use serde_json::Value;
use tracing::warn;

/// Parse a raw string that may contain one or more NDJSON lines.
///
/// Returns a Vec of successfully parsed JSON values.
/// Lines that fail to parse are logged and skipped.
pub fn parse_ndjson(raw: &str) -> Vec<Value> {
    raw.split('\n')
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| match serde_json::from_str::<Value>(line.trim()) {
            Ok(value) => Some(value),
            Err(e) => {
                warn!("Failed to parse NDJSON line: {e} — line: {line}");
                None
            }
        })
        .collect()
}

/// Serialize a JSON value to an NDJSON line (JSON + newline).
pub fn to_ndjson(value: &Value) -> String {
    let mut s = serde_json::to_string(value).unwrap_or_else(|_| "{}".to_string());
    s.push('\n');
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_single_line() {
        let input = r#"{"type":"user","message":{"role":"user","content":"hello"}}"#;
        let result = parse_ndjson(input);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["type"], "user");
    }

    #[test]
    fn test_parse_multiple_lines() {
        let input = r#"{"type":"system","subtype":"init"}
{"type":"assistant","message":{}}
"#;
        let result = parse_ndjson(input);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0]["type"], "system");
        assert_eq!(result[1]["type"], "assistant");
    }

    #[test]
    fn test_parse_empty_lines_skipped() {
        let input = "\n\n{\"type\":\"keep_alive\"}\n\n";
        let result = parse_ndjson(input);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_parse_invalid_json_skipped() {
        let input = "not json\n{\"type\":\"ok\"}\n";
        let result = parse_ndjson(input);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["type"], "ok");
    }

    #[test]
    fn test_to_ndjson() {
        let value = json!({"type": "keep_alive"});
        let result = to_ndjson(&value);
        assert!(result.ends_with('\n'));
        assert!(result.contains("keep_alive"));
    }
}
