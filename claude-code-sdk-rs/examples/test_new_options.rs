//! Test example for new ClaudeCodeOptions fields

use cc_sdk::{ClaudeCodeOptions, Result};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<()> {
    // Test building options with new fields
    let options = ClaudeCodeOptions::builder()
        .system_prompt("You are a helpful assistant")
        .model("claude-3-opus-20240229")
        .settings("/path/to/settings.json")
        .add_dir("/path/to/project1")
        .add_dir("/path/to/project2")
        .build();

    println!("Options created successfully:");
    println!("  Settings: {:?}", options.settings);
    println!("  Add dirs: {:?}", options.add_dirs);

    // Test with add_dirs method
    let dirs = vec![
        PathBuf::from("/path/to/dir1"),
        PathBuf::from("/path/to/dir2"),
        PathBuf::from("/path/to/dir3"),
    ];

    let options2 = ClaudeCodeOptions::builder()
        .add_dirs(dirs)
        .settings("global-settings.json")
        .build();

    println!("\nOptions2 created successfully:");
    println!("  Settings: {:?}", options2.settings);
    println!("  Add dirs: {:?}", options2.add_dirs);

    Ok(())
}
