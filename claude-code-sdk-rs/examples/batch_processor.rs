//! Batch Question Processor using Claude Code SDK
//!
//! This example demonstrates batch processing of multiple programming questions
//! with retry logic and progress tracking.

use nexus_claude::{ClaudeCodeOptions, ContentBlock, InteractiveClient, PermissionMode, Result};
use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};
use tokio::time::sleep;

#[derive(Debug)]
struct ProcessingStats {
    total: usize,
    successful: usize,
    failed: usize,
    total_duration: Duration,
}

/// Process a batch of questions from a file
async fn process_question_batch(file_path: &Path) -> Result<ProcessingStats> {
    let content =
        fs::read_to_string(file_path).map_err(|e| nexus_claude::SdkError::InvalidState {
            message: format!("Failed to read file: {e}"),
        })?;

    let questions: Vec<&str> = content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();

    let mut stats = ProcessingStats {
        total: questions.len(),
        successful: 0,
        failed: 0,
        total_duration: Duration::new(0, 0),
    };

    let batch_start = Instant::now();
    println!("📊 Starting batch processing of {} questions", stats.total);
    println!("{}", "=".repeat(60));

    // Create a single client for the entire batch
    let options = create_claude_options();
    let mut client = InteractiveClient::new(options)?;
    client.connect().await?;

    for (idx, question) in questions.iter().enumerate() {
        println!(
            "\n🔹 Processing question {}/{}: {}",
            idx + 1,
            stats.total,
            question
        );

        let question_start = Instant::now();
        let result = process_single_question(&mut client, question, idx + 1).await;
        let question_duration = question_start.elapsed();

        match result {
            Ok(_) => {
                stats.successful += 1;
                println!("✅ Success! (took {:.2}s)", question_duration.as_secs_f64());
            },
            Err(e) => {
                stats.failed += 1;
                println!(
                    "❌ Failed: {:?} (took {:.2}s)",
                    e,
                    question_duration.as_secs_f64()
                );

                // Implement retry logic for rate limits
                if is_rate_limit_error(&e) {
                    println!("⏳ Rate limit detected, waiting 30 seconds...");
                    sleep(Duration::from_secs(30)).await;
                }
            },
        }

        // Progress update
        let progress = ((idx + 1) as f32 / stats.total as f32) * 100.0;
        println!(
            "📈 Progress: {:.1}% ({}/{})",
            progress,
            idx + 1,
            stats.total
        );
    }

    client.disconnect().await?;
    stats.total_duration = batch_start.elapsed();

    Ok(stats)
}

/// Process a single question
async fn process_single_question(
    client: &mut InteractiveClient,
    question: &str,
    index: usize,
) -> Result<()> {
    let project_name = format!("solution_{index:03}");

    // Generate solution
    let prompt = format!(
        "Create a minimal Rust project called '{project_name}' that solves: {question}. \
        Include tests and use best practices."
    );

    let messages = client.send_and_receive(prompt).await?;

    // Check if Claude successfully created the project
    let success = messages.iter().any(|msg| {
        if let nexus_claude::Message::Assistant { message } = msg {
            message.content.iter().any(|content| {
                if let ContentBlock::Text(text) = content {
                    text.text.contains("created") || text.text.contains("successfully")
                } else {
                    false
                }
            })
        } else {
            false
        }
    });

    if !success {
        return Err(nexus_claude::SdkError::InvalidState {
            message: "Failed to create project".to_string(),
        });
    }

    // Quick verification
    let verify = "Run 'cargo check' on the project to ensure it compiles.";
    client.send_and_receive(verify.to_string()).await?;

    Ok(())
}

/// Create Claude options for batch processing
fn create_claude_options() -> ClaudeCodeOptions {
    ClaudeCodeOptions::builder()
        .system_prompt("You are a Rust expert. Create concise, working solutions.")
        .model("sonnet")
        .permission_mode(PermissionMode::AcceptEdits)
        .allowed_tools(vec![
            "bash".to_string(),
            "write_file".to_string(),
            "edit_file".to_string(),
        ])
        .max_turns(10) // Lower for batch processing
        .build()
}

/// Check if error is rate limit related
fn is_rate_limit_error(error: &nexus_claude::SdkError) -> bool {
    let error_str = format!("{error:?}").to_lowercase();
    error_str.contains("rate") || error_str.contains("limit") || error_str.contains("quota")
}

/// Print processing statistics
fn print_stats(stats: &ProcessingStats) {
    println!("\n📊 Batch Processing Summary");
    println!("{}", "=".repeat(60));
    println!("Total questions: {}", stats.total);
    println!(
        "Successful: {} ({}%)",
        stats.successful,
        (stats.successful as f32 / stats.total as f32 * 100.0) as u32
    );
    println!("Failed: {}", stats.failed);
    println!(
        "Total time: {:.2} seconds",
        stats.total_duration.as_secs_f64()
    );
    println!(
        "Average time per question: {:.2} seconds",
        stats.total_duration.as_secs_f64() / stats.total as f64
    );
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("🦀 Claude Code SDK - Batch Question Processor\n");

    // Example 1: Create a sample questions file
    let questions_file = "rust_questions.txt";
    if !Path::new(questions_file).exists() {
        println!("📝 Creating sample questions file...");
        let sample_questions = r#"Implement a stack data structure with push, pop, and peek operations
Create a function to check if a string is a valid palindrome
Write a program to find all prime numbers up to N using the Sieve of Eratosthenes
Implement a simple rate limiter using token bucket algorithm
Create a generic binary tree with in-order traversal"#;

        fs::write(questions_file, sample_questions).expect("Failed to create questions file");
        println!("✅ Created {questions_file}\n");
    }

    // Process the questions
    match process_question_batch(Path::new(questions_file)).await {
        Ok(stats) => {
            print_stats(&stats);
            println!("\n✨ Batch processing completed successfully!");
        },
        Err(e) => {
            eprintln!("\n❌ Batch processing failed: {e:?}");
        },
    }

    Ok(())
}
