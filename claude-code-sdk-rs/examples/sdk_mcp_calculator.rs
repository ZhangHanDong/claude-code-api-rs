//! SDK MCP Server Example - Calculator
//!
//! This example demonstrates how to create an in-process MCP server with
//! calculator tools using the Claude Code Rust SDK.
//!
//! Unlike external MCP servers that require separate processes, this server
//! runs directly within your Rust application, providing better performance
//! and simpler deployment.

use nexus_claude::{
    ClaudeCodeOptions, InteractiveClient, Message, Result, SdkMcpServerBuilder, ToolInputSchema,
    create_simple_tool,
};
use serde_json::json;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Create calculator tools
    let add_tool = create_simple_tool(
        "add",
        "Add two numbers",
        ToolInputSchema {
            schema_type: "object".to_string(),
            properties: {
                let mut props = HashMap::new();
                props.insert(
                    "a".to_string(),
                    json!({"type": "number", "description": "First number"}),
                );
                props.insert(
                    "b".to_string(),
                    json!({"type": "number", "description": "Second number"}),
                );
                props
            },
            required: Some(vec!["a".to_string(), "b".to_string()]),
        },
        |args| async move {
            let a = args["a"]
                .as_f64()
                .ok_or_else(|| nexus_claude::SdkError::invalid_state("Invalid number"))?;
            let b = args["b"]
                .as_f64()
                .ok_or_else(|| nexus_claude::SdkError::invalid_state("Invalid number"))?;
            let result = a + b;
            Ok(format!("{a} + {b} = {result}"))
        },
    );

    let subtract_tool = create_simple_tool(
        "subtract",
        "Subtract one number from another",
        ToolInputSchema {
            schema_type: "object".to_string(),
            properties: {
                let mut props = HashMap::new();
                props.insert(
                    "a".to_string(),
                    json!({"type": "number", "description": "First number"}),
                );
                props.insert(
                    "b".to_string(),
                    json!({"type": "number", "description": "Second number"}),
                );
                props
            },
            required: Some(vec!["a".to_string(), "b".to_string()]),
        },
        |args| async move {
            let a = args["a"]
                .as_f64()
                .ok_or_else(|| nexus_claude::SdkError::invalid_state("Invalid number"))?;
            let b = args["b"]
                .as_f64()
                .ok_or_else(|| nexus_claude::SdkError::invalid_state("Invalid number"))?;
            let result = a - b;
            Ok(format!("{a} - {b} = {result}"))
        },
    );

    let multiply_tool = create_simple_tool(
        "multiply",
        "Multiply two numbers",
        ToolInputSchema {
            schema_type: "object".to_string(),
            properties: {
                let mut props = HashMap::new();
                props.insert(
                    "a".to_string(),
                    json!({"type": "number", "description": "First number"}),
                );
                props.insert(
                    "b".to_string(),
                    json!({"type": "number", "description": "Second number"}),
                );
                props
            },
            required: Some(vec!["a".to_string(), "b".to_string()]),
        },
        |args| async move {
            let a = args["a"]
                .as_f64()
                .ok_or_else(|| nexus_claude::SdkError::invalid_state("Invalid number"))?;
            let b = args["b"]
                .as_f64()
                .ok_or_else(|| nexus_claude::SdkError::invalid_state("Invalid number"))?;
            let result = a * b;
            Ok(format!("{a} × {b} = {result}"))
        },
    );

    let divide_tool = create_simple_tool(
        "divide",
        "Divide one number by another",
        ToolInputSchema {
            schema_type: "object".to_string(),
            properties: {
                let mut props = HashMap::new();
                props.insert(
                    "a".to_string(),
                    json!({"type": "number", "description": "First number"}),
                );
                props.insert(
                    "b".to_string(),
                    json!({"type": "number", "description": "Second number"}),
                );
                props
            },
            required: Some(vec!["a".to_string(), "b".to_string()]),
        },
        |args| async move {
            let a = args["a"]
                .as_f64()
                .ok_or_else(|| nexus_claude::SdkError::invalid_state("Invalid number"))?;
            let b = args["b"]
                .as_f64()
                .ok_or_else(|| nexus_claude::SdkError::invalid_state("Invalid number"))?;
            if b == 0.0 {
                return Err(nexus_claude::SdkError::invalid_state(
                    "Division by zero is not allowed",
                ));
            }
            let result = a / b;
            Ok(format!("{a} ÷ {b} = {result}"))
        },
    );

    // Create the calculator SDK MCP server
    let calculator = SdkMcpServerBuilder::new("calculator")
        .version("2.0.0")
        .tool(add_tool)
        .tool(subtract_tool)
        .tool(multiply_tool)
        .tool(divide_tool)
        .build();

    // Convert to config
    let calc_config = calculator.to_config();

    // Configure Claude to use the calculator server
    let mut mcp_servers = HashMap::new();
    mcp_servers.insert("calc".to_string(), calc_config);

    let options = ClaudeCodeOptions::builder()
        .mcp_servers(mcp_servers)
        .allowed_tools(vec![
            "mcp__calc__add".to_string(),
            "mcp__calc__subtract".to_string(),
            "mcp__calc__multiply".to_string(),
            "mcp__calc__divide".to_string(),
        ])
        .build();

    // Create interactive client
    let mut client = InteractiveClient::new(options)?;
    client.connect().await?;

    // Example prompts
    let prompts = vec![
        "Calculate 15 + 27",
        "What is 100 divided by 7?",
        "Calculate (12 + 8) * 3 - 10",
    ];

    for prompt in prompts {
        println!("\n{}", "=".repeat(50));
        println!("Prompt: {prompt}");
        println!("{}", "=".repeat(50));

        // Send message and receive response
        let messages = client.send_and_receive(prompt.to_string()).await?;

        for message in messages {
            match message {
                Message::User { .. } => {},
                Message::Assistant { message } => {
                    for content in message.content {
                        match content {
                            nexus_claude::ContentBlock::Text(text) => {
                                println!("Claude: {}", text.text);
                            },
                            nexus_claude::ContentBlock::ToolUse(tool_use) => {
                                println!("Using tool: {}", tool_use.name);
                                println!("  Input: {:?}", tool_use.input);
                            },
                            _ => {},
                        }
                    }
                },
                Message::Result {
                    total_cost_usd: Some(cost),
                    ..
                } => {
                    println!("Cost: ${cost:.6}");
                },
                _ => {},
            }
        }
    }

    client.disconnect().await?;
    println!("\n✅ SDK MCP Calculator demo completed!");

    Ok(())
}
