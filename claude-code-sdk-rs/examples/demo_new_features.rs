//! Demo of new v0.1.6 features without requiring Claude CLI
//!
//! This example demonstrates the new settings and add_dirs features

use cc_sdk::ClaudeCodeOptions;
use std::env;
use std::path::PathBuf;

fn main() {
    println!("Claude Code SDK v0.1.6 - New Features Demo");
    println!("==========================================\n");

    // Demo 1: Settings parameter
    println!("1. Settings Parameter:");
    println!("----------------------");

    let current_dir = env::current_dir().expect("Failed to get current directory");
    let settings_path = current_dir.join("examples/claude-settings.json");

    let options_with_settings = ClaudeCodeOptions::builder()
        .settings(settings_path.to_str().unwrap())
        .model("claude-3-opus-20240229")
        .build();

    println!(
        "   Created options with settings: {:?}",
        options_with_settings.settings
    );
    println!();

    // Demo 2: Add single directory
    println!("2. Add Single Directory:");
    println!("------------------------");

    let options_single_dir = ClaudeCodeOptions::builder()
        .cwd("/main/project")
        .add_dir("/additional/project1")
        .add_dir("/additional/project2")
        .build();

    println!("   Working directory: {:?}", options_single_dir.cwd);
    println!(
        "   Additional directories: {:?}",
        options_single_dir.add_dirs
    );
    println!();

    // Demo 3: Add multiple directories at once
    println!("3. Add Multiple Directories:");
    println!("----------------------------");

    let dirs = vec![
        PathBuf::from("/project/frontend"),
        PathBuf::from("/project/backend"),
        PathBuf::from("/project/shared"),
    ];

    let options_multi_dir = ClaudeCodeOptions::builder().add_dirs(dirs.clone()).build();

    println!(
        "   Added {} directories at once:",
        options_multi_dir.add_dirs.len()
    );
    for dir in &options_multi_dir.add_dirs {
        println!("     - {}", dir.display());
    }
    println!();

    // Demo 4: Complete configuration
    println!("4. Complete Configuration Example:");
    println!("----------------------------------");

    let complete_options = ClaudeCodeOptions::builder()
        // Working directory
        .cwd("/Users/zhangalex/Work/Projects/main")
        // Settings file
        .settings(settings_path.to_str().unwrap())
        // Additional directories
        .add_dir("/Users/zhangalex/Work/Projects/lib1")
        .add_dir("/Users/zhangalex/Work/Projects/lib2")
        // Other options
        .system_prompt("You are an expert developer")
        .model("claude-3-opus-20240229")
        .permission_mode(cc_sdk::PermissionMode::AcceptEdits)
        .max_turns(10)
        .build();

    println!("   Working directory: {:?}", complete_options.cwd);
    println!("   Settings file: {:?}", complete_options.settings);
    println!(
        "   Additional dirs: {} directories",
        complete_options.add_dirs.len()
    );
    println!("   Model: {:?}", complete_options.model);
    println!("   Permission mode: {:?}", complete_options.permission_mode);
    println!("   Max turns: {:?}", complete_options.max_turns);
    println!();

    println!("✅ All new features are working correctly!");
    println!("\nTo use these features with Claude CLI, run:");
    println!("  cargo run --example test_settings");
    println!("  cargo run --example test_add_dirs");
    println!("  cargo run --example test_combined_features");
}
