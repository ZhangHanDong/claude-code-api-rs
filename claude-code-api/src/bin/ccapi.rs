use std::process::Command;
use std::env;
use std::os::unix::process::CommandExt;

fn main() {
    // Get the path to the claude-code-api binary
    let exe_path = env::current_exe()
        .expect("Failed to get current executable path");
    
    let exe_dir = exe_path.parent()
        .expect("Failed to get executable directory");
    
    let claude_code_api_path = exe_dir.join("claude-code-api");
    
    // Forward all arguments to claude-code-api
    let args: Vec<String> = env::args().skip(1).collect();
    
    // Execute claude-code-api with the same arguments
    let _ = Command::new(claude_code_api_path)
        .args(args)
        .exec();
    
    // If exec fails, print error
    eprintln!("Failed to execute claude-code-api");
    std::process::exit(1);
}