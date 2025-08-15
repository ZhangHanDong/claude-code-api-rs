//! Test example to verify paths are correct

use std::env;
use std::path::Path;

fn main() {
    println!("Path verification test\n");
    println!("======================\n");

    // Get current directory
    let current_dir = env::current_dir().expect("Failed to get current directory");
    println!("Current directory: {}", current_dir.display());

    // Check various paths
    let paths_to_check = vec![
        "examples/claude-settings.json",
        "examples/custom-claude-settings.json",
        "claude-code-sdk-rs/examples/claude-settings.json",
        "claude-code-sdk-rs/examples/custom-claude-settings.json",
    ];

    println!("\nChecking relative paths:");
    for path_str in &paths_to_check {
        let path = Path::new(path_str);
        println!("  {} -> exists: {}", path_str, path.exists());
    }

    println!("\nChecking absolute paths:");
    for path_str in &paths_to_check {
        let abs_path = current_dir.join(path_str);
        println!("  {} -> exists: {}", abs_path.display(), abs_path.exists());
    }

    // Show the correct absolute paths
    println!("\nCorrect absolute paths:");
    let settings1 = current_dir.join("examples/claude-settings.json");
    let settings2 = current_dir.join("examples/custom-claude-settings.json");

    if settings1.exists() {
        println!("✓ {}", settings1.display());
    }
    if settings2.exists() {
        println!("✓ {}", settings2.display());
    }
}
