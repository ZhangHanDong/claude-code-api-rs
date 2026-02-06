use std::env;
use std::process::Command;

#[cfg(unix)]
use std::os::unix::process::CommandExt;

fn main() {
    // Get the path to the claude-code-api binary
    let exe_path = env::current_exe().expect("Failed to get current executable path");

    let exe_dir = exe_path
        .parent()
        .expect("Failed to get executable directory");

    #[cfg(windows)]
    let claude_code_api_path = exe_dir.join("claude-code-api.exe");
    #[cfg(not(windows))]
    let claude_code_api_path = exe_dir.join("claude-code-api");

    // Forward all arguments to claude-code-api
    let args: Vec<String> = env::args().skip(1).collect();

    // Execute claude-code-api with the same arguments
    #[cfg(unix)]
    {
        // On Unix, use exec() to replace the current process
        let err = Command::new(&claude_code_api_path).args(&args).exec();
        eprintln!("Failed to execute claude-code-api: {}", err);
        std::process::exit(1);
    }

    #[cfg(windows)]
    {
        // On Windows, spawn and wait for the child process
        match Command::new(&claude_code_api_path).args(&args).status() {
            Ok(status) => {
                std::process::exit(status.code().unwrap_or(1));
            },
            Err(e) => {
                eprintln!("Failed to execute claude-code-api: {}", e);
                std::process::exit(1);
            },
        }
    }
}
