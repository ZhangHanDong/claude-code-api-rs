//! Code Generator Example using Claude Code SDK
//!
//! This example shows how to use the SDK to generate Rust code solutions
//! with tests and documentation.

use cc_sdk::{ClaudeCodeOptions, InteractiveClient, PermissionMode, Result};
use std::time::Instant;

async fn generate_rust_solution(question: &str, project_name: &str) -> Result<()> {
    println!("🚀 Generating Rust solution for: {}", question);
    println!("📁 Project name: {}", project_name);
    println!("=" .repeat(60));

    // Configure Claude for code generation
    let options = ClaudeCodeOptions::builder()
        .system_prompt(
            "You are an expert Rust developer. Create clean, idiomatic Rust code \
            with comprehensive tests and documentation."
        )
        .model("claude-3-5-sonnet-20241022")
        .permission_mode(PermissionMode::AcceptEdits)
        .allowed_tools(vec![
            "bash".to_string(),
            "write_file".to_string(),
            "edit_file".to_string(),
            "read_file".to_string(),
        ])
        .max_turns(20)
        .build();

    let mut client = InteractiveClient::new(options)?;
    let start_time = Instant::now();

    // Connect to Claude
    client.connect().await?;
    println!("✅ Connected to Claude\n");

    // Step 1: Generate the solution
    println!("📝 Step 1: Generating Rust code...");
    let prompt = format!(
        "Create a new Rust project called '{}' that solves this problem: {}. \
        Include comprehensive unit tests and proper error handling.",
        project_name, question
    );

    let messages = client.send_and_receive(prompt).await?;
    print_claude_response(&messages);

    // Step 2: Verify the solution
    println!("\n🔍 Step 2: Verifying the solution...");
    let verify_prompt = format!(
        "Please run 'cargo check', 'cargo test', and 'cargo clippy' on the {} project \
        to ensure everything is correct. Fix any issues found.",
        project_name
    );

    let messages = client.send_and_receive(verify_prompt).await?;
    print_claude_response(&messages);

    // Step 3: Add documentation
    println!("\n📚 Step 3: Adding documentation...");
    let doc_prompt = format!(
        "Add a comprehensive README.md to the {} project explaining the solution, \
        how to use it, and include examples.",
        project_name
    );

    let messages = client.send_and_receive(doc_prompt).await?;
    print_claude_response(&messages);

    // Disconnect
    client.disconnect().await?;

    let duration = start_time.elapsed();
    println!("\n✨ Solution generated successfully!");
    println!("⏱️  Total time: {:.2} seconds", duration.as_secs_f64());
    println!("=" .repeat(60));

    Ok(())
}

fn print_claude_response(messages: &[cc_sdk::Message]) {
    for msg in messages {
        if let cc_sdk::Message::Assistant { message } = msg {
            for content in &message.content {
                if let cc_sdk::ContentBlock::Text(text) = content {
                    // Only print first 500 chars to keep output readable
                    let preview = if text.text.len() > 500 {
                        format!("{}...", &text.text[..500])
                    } else {
                        text.text.clone()
                    };
                    println!("{}", preview);
                }
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("🦀 Claude Code SDK - Rust Code Generator Example\n");

    // Example problems to solve
    let examples = vec![
        ("Binary Search Implementation", "binary_search"),
        ("LRU Cache with Generics", "lru_cache"),
        ("Thread-Safe Counter", "safe_counter"),
    ];

    // Process each example
    for (i, (question, project_name)) in examples.iter().enumerate() {
        println!("\n📌 Example {}: {}\n", i + 1, question);

        match generate_rust_solution(question, project_name).await {
            Ok(_) => println!("✅ Successfully generated: {}", project_name),
            Err(e) => eprintln!("❌ Failed to generate {}: {:?}", project_name, e),
        }

        // Add delay between examples to avoid rate limits
        if i < examples.len() - 1 {
            println!("\n⏳ Waiting 5 seconds before next example...");
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        }
    }

    println!("\n🎉 All examples completed!");
    Ok(())
}
