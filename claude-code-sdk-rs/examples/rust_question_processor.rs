//! Rust Question Processor using Claude Code SDK
//!
//! This example demonstrates how to use the Claude Code SDK to process Rust programming
//! questions and generate annotated code solutions with comprehensive unit tests.

use chrono::{DateTime, Utc};
use nexus_claude::{ClaudeCodeOptions, ContentBlock, InteractiveClient, PermissionMode, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tokio::time::sleep;

/// Process a single Rust programming question
async fn process_single_question(question: &str, target_dir: &str) -> Result<()> {
    let script_start = Instant::now();
    let start_time: DateTime<Utc> = Utc::now();

    println!(
        "Starting Rust development procedure at {}",
        start_time.format("%Y-%m-%d %H:%M:%S UTC")
    );
    println!("User question: {question}");
    println!("================================================");

    let current_dir = std::env::current_dir().expect("Failed to get current directory");
    let annotations_dir = current_dir.join("annotations");

    // Ensure annotations directory exists
    fs::create_dir_all(&annotations_dir).expect("Failed to create annotations directory");

    let full_dir = annotations_dir.display();
    let system_prompt = format!(
        "You are a senior Rust developer. The current working directory is {}. \
        Create a minimal Rust project (use cargo) named '{}' in the subdirectory '{}' \
        as the answer to the user's question. Add comprehensive unit tests. \
        DON'T DELETE ANY OTHER DIRECTORIES IN THE DIR './annotations/'!",
        current_dir.display(),
        target_dir,
        full_dir
    );

    // Configure Claude with appropriate permissions for file operations
    let options = ClaudeCodeOptions::builder()
        .system_prompt(&system_prompt)
        .model("sonnet")
        .permission_mode(PermissionMode::AcceptEdits)
        .allowed_tools(vec![
            "bash".to_string(),
            "write_file".to_string(),
            "edit_file".to_string(),
            "read_file".to_string(),
        ])
        .max_turns(100)
        .cwd(current_dir)
        .build();

    let mut client = InteractiveClient::new(options)?;
    client.connect().await?;

    // Step 1: Create minimal Rust project
    println!("Step 1: Creating minimal Rust project...");
    let step1_start = Instant::now();

    let response = client.send_and_receive(question.to_string()).await?;
    print_response(&response);

    let step1_duration = step1_start.elapsed();
    println!("Step 1 completed in {} seconds", step1_duration.as_secs());
    println!("================================================");

    sleep(Duration::from_secs(3)).await;

    // Step 2: Pipeline verification
    println!("Step 2: Verifying project with pipeline checks...");
    let step2_start = Instant::now();

    let verification_prompt = "Please make sure to pass the pipeline of 'cargo check; cargo test; cargo clippy;' \
        to verify the correctness of this minimal rust project. \
        At last tell me how many times you iterated. \
        Don't generate any README file in this step.";

    let response = client
        .send_and_receive(verification_prompt.to_string())
        .await?;
    print_response(&response);

    let step2_duration = step2_start.elapsed();
    println!("Step 2 completed in {} seconds", step2_duration.as_secs());
    println!("================================================");

    sleep(Duration::from_secs(3)).await;

    // Step 3: Generate documentation
    println!("Step 3: Generating documentation...");
    let step3_start = Instant::now();

    let doc_prompt = "In the target directory, please execute `cargo clean` and then create a detailed \
        README.md file to record: the metadata including the llm version, date, os version, \
        rustc version, rust toolchain, rust target, and other essential info; \
        and the log of the entire procedure, step by step. \
        At last tell me how many times you iterated.";

    let response = client.send_and_receive(doc_prompt.to_string()).await?;
    print_response(&response);

    let step3_duration = step3_start.elapsed();
    println!("Step 3 completed in {} seconds", step3_duration.as_secs());
    println!("================================================");

    // Disconnect from Claude
    client.disconnect().await?;

    let total_duration = script_start.elapsed();

    println!("TIMING SUMMARY:");
    println!(
        "Step 1 (Project Creation): {} seconds",
        step1_duration.as_secs()
    );
    println!(
        "Step 2 (Pipeline Verification): {} seconds",
        step2_duration.as_secs()
    );
    println!(
        "Step 3 (Documentation): {} seconds",
        step3_duration.as_secs()
    );
    println!(
        "Total procedure time: {} seconds ({} minutes {} seconds)",
        total_duration.as_secs(),
        total_duration.as_secs() / 60,
        total_duration.as_secs() % 60
    );

    let end_time: DateTime<Utc> = Utc::now();
    println!(
        "Script completed successfully at {}!",
        end_time.format("%Y-%m-%d %H:%M:%S UTC")
    );

    Ok(())
}

/// Process a question set file
async fn process_question_set(question_set_file: &Path, start_from: Option<u32>) -> Result<()> {
    if !question_set_file.exists() {
        return Err(nexus_claude::SdkError::InvalidState {
            message: format!(
                "Question set file '{}' not found!",
                question_set_file.display()
            ),
        });
    }

    let content = fs::read_to_string(question_set_file).map_err(|e| {
        nexus_claude::SdkError::InvalidState {
            message: format!("Failed to read question set file: {e}"),
        }
    })?;

    // Extract question set number from filename
    let basename = question_set_file
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| nexus_claude::SdkError::InvalidState {
            message: "Invalid filename".to_string(),
        })?;

    let qs_number =
        basename
            .strip_prefix("qs")
            .ok_or_else(|| nexus_claude::SdkError::InvalidState {
                message: "Filename should start with 'qs'".to_string(),
            })?;

    println!("Processing question set: {}", question_set_file.display());
    println!("Question set number: {qs_number}");
    if let Some(start) = start_from {
        println!("Starting from question: {start}");
    }
    println!("================================================");

    let question_regex = regex::Regex::new(r"^(\d+)\.\s*(.+)$").expect("Failed to compile regex");

    let mut processed_count = 0;
    let set_start_time = Instant::now();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if let Some(captures) = question_regex.captures(line) {
            let question_num: u32 = captures[1].parse().unwrap_or(0);

            if let Some(start) = start_from
                && question_num < start
            {
                continue;
            }

            let question_text = &captures[2];
            let formatted_question_num = format!("{question_num:05}");
            let target_dir = format!("q{qs_number}{formatted_question_num}");

            println!("Processing question {question_num}: {question_text}");
            println!("Target directory: {target_dir}");
            println!("----------------------------------------");

            let question_start = Instant::now();

            match process_single_question(question_text, &target_dir).await {
                Ok(_) => {
                    let question_duration = question_start.elapsed();
                    processed_count += 1;
                    println!(
                        "Completed question {} in {} seconds",
                        question_num,
                        question_duration.as_secs()
                    );
                },
                Err(e) => {
                    println!("ERROR: Failed to process question {question_num}: {e:?}");
                    // Continue with next question instead of stopping
                },
            }
            println!("----------------------------------------");
        }
    }

    let total_duration = set_start_time.elapsed();
    println!("================================================");
    println!("SUMMARY for {}:", question_set_file.display());
    println!("Successfully processed: {processed_count} questions");
    println!(
        "Total processing time: {} seconds ({} minutes)",
        total_duration.as_secs(),
        total_duration.as_secs() / 60
    );

    Ok(())
}

/// Print response from Claude
fn print_response(messages: &[nexus_claude::Message]) {
    for msg in messages {
        match msg {
            nexus_claude::Message::Assistant { message } => {
                for content in &message.content {
                    if let ContentBlock::Text(text) = content {
                        println!("{}", text.text);
                    }
                }
            },
            nexus_claude::Message::System { subtype, .. } => {
                if subtype != "thinking" {
                    println!("[System: {subtype}]");
                }
            },
            _ => {},
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Example 1: Process a single question
    println!("Example 1: Processing a single Rust question\n");

    let question = "Create a function that calculates the nth Fibonacci number using memoization";
    let target_dir = "fibonacci_memo";

    if let Err(e) = process_single_question(question, target_dir).await {
        eprintln!("Failed to process question: {e:?}");
    }

    println!("\n\n");

    // Example 2: Process a question set (if file exists)
    println!("Example 2: Processing a question set\n");

    let question_set_path = PathBuf::from("qs/qs00001.txt");
    if question_set_path.exists() {
        if let Err(e) = process_question_set(&question_set_path, None).await {
            eprintln!("Failed to process question set: {e:?}");
        }
    } else {
        println!("Question set file not found. Create qs/qs00001.txt with questions like:");
        println!("1. Create a binary search tree implementation");
        println!("2. Implement a thread-safe counter using Arc and Mutex");
        println!("3. Write a parser for simple arithmetic expressions");
    }

    Ok(())
}
