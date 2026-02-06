//! Test automatic CLI download functionality
//!
//! This example demonstrates and tests the auto-download feature.
//!
//! Run with:
//! ```bash
//! cargo run --example test_auto_download
//! ```

use nexus_claude::ClaudeCodeOptions;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("nexus_claude=debug,info")
        .init();

    println!("=== Claude Code CLI Auto-Download Test ===\n");

    // Test 1: Check cache directory location
    println!("1. Cache Directory Information:");
    if let Some(cache_dir) = nexus_claude::cli_download::get_cache_dir() {
        println!("   Cache directory: {}", cache_dir.display());
    } else {
        println!("   ❌ Could not determine cache directory");
    }

    if let Some(cli_path) = nexus_claude::cli_download::get_cached_cli_path() {
        println!("   Expected CLI path: {}", cli_path.display());
        if cli_path.exists() {
            println!("   ✅ CLI is already cached!");
        } else {
            println!("   ℹ️  CLI is not yet cached");
        }
    }
    println!();

    // Test 2: Try to find existing CLI
    println!("2. Searching for Existing CLI:");
    match nexus_claude::transport::subprocess::find_claude_cli() {
        Ok(path) => {
            println!("   ✅ Found CLI at: {}", path.display());
            println!("   No download needed!");
        },
        Err(e) => {
            println!("   ℹ️  CLI not found in standard locations");
            println!("   Details: {}", e);
        },
    }
    println!();

    // Test 3: Test auto-download (optional - uncomment to actually download)
    println!("3. Auto-Download Test:");
    println!("   To test auto-download, uncomment the code below and run again.");
    println!();

    // Uncomment this block to test actual download:
    /*
    println!("   Attempting to download CLI...");
    match nexus_claude::cli_download::download_cli(None, Some(Box::new(|downloaded, total| {
        if let Some(total) = total {
            println!("   Progress: {}/{} bytes", downloaded, total);
        } else {
            println!("   Downloaded: {} bytes", downloaded);
        }
    }))).await {
        Ok(path) => {
            println!("   ✅ CLI downloaded successfully to: {}", path.display());
        }
        Err(e) => {
            println!("   ❌ Download failed: {}", e);
        }
    }
    */
    println!();

    // Test 4: Create options with auto_download_cli
    println!("4. ClaudeCodeOptions with auto_download_cli:");
    let options = ClaudeCodeOptions::builder()
        .auto_download_cli(true)
        .model("sonnet")
        .build();

    println!("   auto_download_cli: {}", options.auto_download_cli);
    println!("   model: {:?}", options.model);
    println!();

    // Test 5: Demonstrate SubprocessTransport::new_async
    println!("5. Testing SubprocessTransport::new_async:");
    println!("   This will attempt to find or download CLI...");

    let options_for_transport = ClaudeCodeOptions::builder().auto_download_cli(true).build();

    match nexus_claude::transport::SubprocessTransport::new_async(options_for_transport).await {
        Ok(transport) => {
            println!("   ✅ Transport created successfully!");
            println!("   CLI is ready to use.");
            drop(transport);
        },
        Err(e) => {
            println!("   ❌ Failed to create transport: {}", e);
            println!();
            println!("   This is expected if:");
            println!("   - npm is not installed");
            println!("   - Network is unavailable");
            println!("   - Official install script is not accessible");
        },
    }
    println!();

    println!("=== Test Complete ===");

    Ok(())
}
