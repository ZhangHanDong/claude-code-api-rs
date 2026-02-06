//! Full-featured Rust Question Processor using Claude Code SDK
//!
//! Matches the original script functionality including:
//! - Question set file naming (qs00001.txt)
//! - Project naming (q0000100001)
//! - Annotations directory structure
//! - Start from specific question number

use chrono::{DateTime, Utc};
use nexus_claude::{ClaudeCodeOptions, ContentBlock, InteractiveClient, PermissionMode, Result};
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tokio::time::sleep;

struct QuestionSetProcessor {
    client: InteractiveClient,
    annotations_dir: PathBuf,
    qs_number: String,
}

impl QuestionSetProcessor {
    async fn new(annotations_dir: PathBuf) -> Result<Self> {
        fs::create_dir_all(&annotations_dir).map_err(|e| nexus_claude::SdkError::InvalidState {
            message: format!("Failed to create annotations directory: {e}"),
        })?;

        let current_dir = std::env::current_dir().expect("Failed to get current directory");

        let system_prompt = format!(
            "You are a senior Rust developer, the current working directory is {}, \
            now you are asked to help user to create a minimal Rust project (use cargo) \
            in the subdirectory `{}` as the answer against the user's question, \
            and please add comprehensive unit tests for it. \
            DON'T DELETE ANY OTHER DIRECTORIES IN THE DIR `./annotations/`!",
            current_dir.display(),
            annotations_dir.display()
        );

        let options = ClaudeCodeOptions::builder()
            .system_prompt(&system_prompt)
            .model("sonnet")
            .permission_mode(PermissionMode::BypassPermissions)
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

        Ok(Self {
            client,
            annotations_dir,
            qs_number: String::new(),
        })
    }

    async fn process_question_set(
        &mut self,
        question_set_file: &Path,
        start_from: Option<u32>,
    ) -> Result<()> {
        // Extract question set number from filename (e.g., qs00035.txt -> 00035)
        let basename = question_set_file
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| nexus_claude::SdkError::InvalidState {
                message: "Invalid filename".to_string(),
            })?;

        self.qs_number = basename
            .strip_prefix("qs")
            .ok_or_else(|| nexus_claude::SdkError::InvalidState {
                message: "Filename should start with 'qs'".to_string(),
            })?
            .to_string();

        println!("Processing question set: {}", question_set_file.display());
        println!("Question set number: {}", self.qs_number);
        if let Some(start) = start_from {
            println!("Starting from question: {start}");
        }
        println!("================================================");

        let content = fs::read_to_string(question_set_file).map_err(|e| {
            nexus_claude::SdkError::InvalidState {
                message: format!("Failed to read question set file: {e}"),
            }
        })?;

        let question_regex = Regex::new(r"^(\d+)\.\s*(.+)$").expect("Failed to compile regex");

        let mut processed_count = 0;
        let mut failed_questions = Vec::new();
        let set_start_time = Instant::now();

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            if let Some(captures) = question_regex.captures(line) {
                let question_num: u32 = captures[1].parse().unwrap_or(0);

                // Skip questions before start_from if specified
                if let Some(start) = start_from
                    && question_num < start
                {
                    continue;
                }

                let question_text = &captures[2];

                // Generate project name: q{qs_number}{question_num:05}
                let formatted_question_num = format!("{question_num:05}");
                let target_dir = format!("q{}{}", self.qs_number, formatted_question_num);

                println!("\nProcessing question {question_num}: {question_text}");
                println!("Target directory: annotations/{target_dir}");
                println!("----------------------------------------");

                let question_start = Instant::now();

                match self
                    .process_single_question(question_text, &target_dir)
                    .await
                {
                    Ok(_) => {
                        let duration = question_start.elapsed();
                        processed_count += 1;
                        println!(
                            "✓ Completed question {} in {} seconds",
                            question_num,
                            duration.as_secs()
                        );
                    },
                    Err(e) => {
                        let duration = question_start.elapsed();
                        failed_questions.push((question_num, format!("{e:?}")));
                        println!(
                            "✗ Failed question {} after {} seconds: {:?}",
                            question_num,
                            duration.as_secs(),
                            e
                        );
                    },
                }
            }
        }

        let total_duration = set_start_time.elapsed();

        // Print summary
        println!("\n================================================");
        println!("SUMMARY for {}:", question_set_file.display());
        println!("Successfully processed: {processed_count}");
        println!("Failed: {}", failed_questions.len());
        println!(
            "Total time: {} seconds ({} minutes {} seconds)",
            total_duration.as_secs(),
            total_duration.as_secs() / 60,
            total_duration.as_secs() % 60
        );

        if !failed_questions.is_empty() {
            println!("\nFailed questions:");
            for (num, error) in &failed_questions {
                println!("  - Question {num}: {error}");
            }
        }

        Ok(())
    }

    async fn process_single_question(&mut self, question: &str, target_dir: &str) -> Result<()> {
        let full_target_path = self.annotations_dir.join(target_dir);
        let script_start = Instant::now();
        let start_time: DateTime<Utc> = Utc::now();

        println!("Starting at {}", start_time.format("%Y-%m-%d %H:%M:%S UTC"));

        // Step 1: Create minimal Rust project
        println!("\nStep 1: Creating minimal Rust project...");
        let step1_start = Instant::now();

        let create_prompt = format!(
            "Create a minimal Rust project with cargo named '{}' in the directory '{}' \
            that solves this problem: {}. Add comprehensive unit tests.",
            target_dir,
            full_target_path.display(),
            question
        );

        let response = self.client.send_and_receive(create_prompt).await?;
        print_response_summary(&response);

        let step1_duration = step1_start.elapsed();
        println!("Step 1 completed in {} seconds", step1_duration.as_secs());

        sleep(Duration::from_secs(3)).await;

        // Step 2: Pipeline verification
        println!("\nStep 2: Verifying project with pipeline checks...");
        let step2_start = Instant::now();

        let verify_prompt = format!(
            "In the project directory '{}', please make sure to pass the pipeline of \
            'cargo check; cargo test; cargo clippy;' to verify the correctness of this \
            minimal rust project. At last tell me how many times you iterated. \
            Don't generate any README file in this step.",
            full_target_path.display()
        );

        let response = self.client.send_and_receive(verify_prompt).await?;
        print_response_summary(&response);

        let step2_duration = step2_start.elapsed();
        println!("Step 2 completed in {} seconds", step2_duration.as_secs());

        sleep(Duration::from_secs(3)).await;

        // Step 3: Generate documentation
        println!("\nStep 3: Generating documentation...");
        let step3_start = Instant::now();

        let doc_prompt = format!(
            "In the target directory '{}', please execute `cargo clean` and then create \
            a detailed README.md file to record: the metadata including the llm version, \
            date, os version, rustc version, rust toolchain, rust target, and some other \
            essential info; and the log of the entire procedure, step by step. \
            At last tell me how many times you iterated.",
            full_target_path.display()
        );

        let response = self.client.send_and_receive(doc_prompt).await?;
        print_response_summary(&response);

        let step3_duration = step3_start.elapsed();
        println!("Step 3 completed in {} seconds", step3_duration.as_secs());

        let total_duration = script_start.elapsed();

        println!("\nTIMING SUMMARY:");
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
        println!("Total: {} seconds", total_duration.as_secs());

        Ok(())
    }
}

fn print_response_summary(messages: &[nexus_claude::Message]) {
    for msg in messages {
        if let nexus_claude::Message::Assistant { message } = msg {
            for content in &message.content {
                if let ContentBlock::Text(text) = content {
                    // Print first 200 chars or look for completion indicators
                    if text.text.contains("created")
                        || text.text.contains("completed")
                        || text.text.contains("iterated")
                    {
                        println!(
                            "  → {}",
                            text.text.lines().take(3).collect::<Vec<_>>().join("\n    ")
                        );
                    }
                }
            }
        }
    }
}

async fn process_all_question_sets(annotations_dir: PathBuf) -> Result<()> {
    let qs_dir = Path::new("qs");

    if !qs_dir.exists() {
        return Err(nexus_claude::SdkError::InvalidState {
            message: "Question sets directory 'qs/' not found!".to_string(),
        });
    }

    println!("Starting batch processing of all question sets...");
    println!("================================================");

    let mut processor = QuestionSetProcessor::new(annotations_dir).await?;
    let batch_start = Instant::now();

    let mut entries: Vec<_> = fs::read_dir(qs_dir)
        .map_err(|e| nexus_claude::SdkError::InvalidState {
            message: format!("Failed to read qs directory: {e}"),
        })?
        .filter_map(|e| e.ok())
        .collect();

    entries.sort_by_key(|entry| entry.file_name());

    let mut stats = (0, 0, 0); // (total, processed, failed)

    for entry in entries {
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) != Some("txt") {
            continue;
        }

        stats.0 += 1;

        println!(
            "\nProcessing file: {}",
            path.file_name().unwrap().to_string_lossy()
        );

        match processor.process_question_set(&path, None).await {
            Ok(_) => {
                stats.1 += 1;
                println!("✓ Successfully processed: {}", path.display());
            },
            Err(e) => {
                stats.2 += 1;
                println!("✗ Failed to process {}: {:?}", path.display(), e);
            },
        }
    }

    let total_duration = batch_start.elapsed();

    println!("\n================================================");
    println!("BATCH PROCESSING SUMMARY:");
    println!("Total files: {}", stats.0);
    println!("Successfully processed: {}", stats.1);
    println!("Failed: {}", stats.2);
    println!("Total time: {} minutes", total_duration.as_secs() / 60);

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();
    let batch_mode = args.contains(&"--batch".to_string());

    // Set up annotations directory
    let annotations_dir = PathBuf::from("annotations");

    println!("Claude Code SDK - Rust Question Processor\n");

    if batch_mode {
        // Batch mode: Process all question sets
        println!("🚀 Running in BATCH mode");
        process_all_question_sets(annotations_dir.clone()).await?;
    } else {
        // Single mode: Process one question set as demo
        println!("📝 Running in SINGLE mode (use --batch for all question sets)");

        // Create a sample question set file if it doesn't exist
        let sample_qs = PathBuf::from("qs/qs00001.txt");
        if !sample_qs.exists() {
            fs::create_dir_all("qs").unwrap();
            fs::write(
                &sample_qs,
                "1. Create a binary search tree implementation\n\
                 2. Implement a thread-safe counter using Arc and Mutex\n\
                 3. Write a parser for simple arithmetic expressions\n",
            )
            .unwrap();
            println!("Created sample question set: {}", sample_qs.display());
        }

        // Process the question set
        let mut processor = QuestionSetProcessor::new(annotations_dir.clone()).await?;

        // Example: Process specific question set, optionally starting from a specific question
        // processor.process_question_set(&PathBuf::from("qs/qs00035.txt"), Some(44)).await?;

        // For demo: just process the sample
        processor.process_question_set(&sample_qs, None).await?;

        // Disconnect
        drop(processor);
    }

    println!("\n✅ Processing complete!");
    println!("Check the annotations/ directory for generated projects");

    // Show example project names
    if annotations_dir.exists() {
        println!("\nGenerated projects:");
        let mut entries: Vec<_> = fs::read_dir(&annotations_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .collect();
        entries.sort_by_key(|e| e.file_name());

        for entry in entries.iter().take(5) {
            println!("  - {}", entry.file_name().to_string_lossy());
        }
        if entries.len() > 5 {
            println!("  ... and {} more", entries.len() - 5);
        }
    }

    Ok(())
}
