use crate::models::openai::{FunctionCall, Tool};
use serde_json::Value;
use tracing::info;

/// Detects if Claude's response contains JSON that should be formatted as a tool call
pub fn detect_and_convert_tool_call(
    content: &str,
    requested_tools: &Option<Vec<Tool>>,
) -> Option<FunctionCall> {
    // Only process if tools were requested
    if requested_tools.is_none() {
        return None;
    }

    // Try to extract JSON from the content
    let json_result = extract_json_from_content(content);

    if let Some(json_value) = json_result {
        info!("Detected JSON in Claude's response: {:?}", json_value);

        // Check if this looks like a tool call
        if let Some(tool_name) = detect_tool_name(&json_value, requested_tools) {
            info!("Converting to tool call: {}", tool_name);

            // Convert the JSON to a function call
            return Some(FunctionCall {
                name: tool_name,
                arguments: json_value.to_string(),
            });
        } else if let Some(tools) = requested_tools {
            // If no specific tool match but JSON is valid and tools were requested,
            // use the first tool name
            if !tools.is_empty() && json_value.is_object() {
                info!("Using first tool as default: {}", tools[0].function.name);
                return Some(FunctionCall {
                    name: tools[0].function.name.clone(),
                    arguments: json_value.to_string(),
                });
            }
        }
    }

    None
}

/// Extracts JSON from Claude's response content
fn extract_json_from_content(content: &str) -> Option<Value> {
    // Try to parse the entire content as JSON first
    if let Ok(json) = serde_json::from_str::<Value>(content.trim()) {
        return Some(json);
    }

    // Look for JSON blocks in the content
    // Claude often wraps JSON in markdown code blocks
    if let Some(start) = content.find("```json") {
        let after_marker = &content[start + 7..];
        if let Some(end) = after_marker.find("```") {
            let json_str = after_marker[..end].trim();
            if let Ok(json) = serde_json::from_str::<Value>(json_str) {
                return Some(json);
            }
        }
    }

    // Look for any JSON object in the content
    if let Some(start) = content.find('{')
        && let Some(end) = find_matching_brace(&content[start..])
    {
        let json_str = &content[start..start + end + 1];
        if let Ok(json) = serde_json::from_str::<Value>(json_str) {
            return Some(json);
        }
    }

    None
}

/// Finds the matching closing brace for a JSON object
fn find_matching_brace(s: &str) -> Option<usize> {
    let mut depth = 0;
    let mut in_string = false;
    let mut escape_next = false;

    for (i, ch) in s.chars().enumerate() {
        if escape_next {
            escape_next = false;
            continue;
        }

        match ch {
            '\\' if in_string => escape_next = true,
            '"' => in_string = !in_string,
            '{' if !in_string => depth += 1,
            '}' if !in_string => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            },
            _ => {},
        }
    }

    None
}

/// Detects if the JSON matches any of the requested tools
fn detect_tool_name(json: &Value, requested_tools: &Option<Vec<Tool>>) -> Option<String> {
    // Check if the JSON has a function/action/tool field
    if let Some(tool_name) = json
        .get("function")
        .or_else(|| json.get("action"))
        .or_else(|| json.get("tool"))
        .or_else(|| json.get("name"))
        .and_then(|v| v.as_str())
    {
        return Some(tool_name.to_string());
    }

    // Check if the JSON structure matches any of the requested tools
    if let Some(tools) = requested_tools {
        for tool in tools {
            if tool.tool_type == "function"
                && json_matches_tool_schema(json, &tool.function.parameters)
            {
                info!("JSON matches schema for tool: {}", tool.function.name);
                return Some(tool.function.name.clone());
            }
        }
    }

    None
}

/// Checks if JSON matches a tool's parameter schema
fn json_matches_tool_schema(json: &Value, schema: &Value) -> bool {
    // Check if the JSON object matches the schema structure
    if let (Some(json_obj), Some(schema_obj)) = (json.as_object(), schema.as_object())
        && let Some(properties) = schema_obj.get("properties").and_then(|p| p.as_object())
    {
        // Check if JSON has all required properties
        if let Some(required) = schema_obj.get("required").and_then(|r| r.as_array()) {
            let required_props: Vec<&str> = required.iter().filter_map(|v| v.as_str()).collect();

            // All required properties must be present
            for req_prop in &required_props {
                if !json_obj.contains_key(*req_prop) {
                    return false;
                }
            }

            // If all required properties are present, it's a match
            return true;
        } else {
            // No required properties specified, check if JSON has any of the schema properties
            let mut matches = 0;
            let total_props = properties.len();

            for (key, _) in properties {
                if json_obj.contains_key(key) {
                    matches += 1;
                }
            }

            // Consider it a match if at least 50% of properties match
            return matches > 0 && (matches * 2 >= total_props);
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_extract_json_from_content() {
        // Test direct JSON
        let content = r#"{"url": "https://example.com", "action": "preview"}"#;
        let result = extract_json_from_content(content);
        assert!(result.is_some());

        // Test JSON in markdown code block
        let content = r#"Here's the result:
```json
{
  "url": "https://example.com",
  "action": "preview"
}
```"#;
        let result = extract_json_from_content(content);
        assert!(result.is_some());

        // Test JSON embedded in text
        let content = r#"The function call is {"url": "https://example.com"} for preview"#;
        let result = extract_json_from_content(content);
        assert!(result.is_some());
    }

    #[test]
    fn test_detect_tool_name() {
        // Test 1: JSON with action field
        let json_with_action = json!({
            "url": "https://example.com",
            "action": "preview"
        });

        let result = detect_tool_name(&json_with_action, &None);
        assert_eq!(result, Some("preview".to_string()));

        // Test 2: JSON matching tool schema
        let json_matching_schema = json!({
            "url": "https://example.com"
        });

        let tool = Tool {
            tool_type: "function".to_string(),
            function: crate::models::openai::FunctionDefinition {
                name: "url_preview".to_string(),
                description: Some("Preview a URL".to_string()),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "url": {"type": "string"}
                    },
                    "required": ["url"]
                }),
            },
        };

        let tools = vec![tool];
        let result = detect_tool_name(&json_matching_schema, &Some(tools));
        assert_eq!(result, Some("url_preview".to_string()));
    }
}
