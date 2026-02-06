use futures::StreamExt;
use nexus_claude::{Result, query};
use std::process::Command;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Testing process leak fix...\n");

    // Check initial Claude processes
    let initial_count = count_claude_processes();
    println!("Initial Claude processes: {initial_count}");

    // Run multiple queries
    for i in 1..=5 {
        println!("\n--- Query {i} ---");

        // Create a query and consume only the first message
        let mut messages = query(format!("Say 'Test {i}'"), None).await?;

        // Only take the first message then drop the stream
        if let Some(msg) = messages.next().await {
            match msg {
                Ok(m) => println!("Got message: {:?}", std::mem::discriminant(&m)),
                Err(e) => println!("Error: {e}"),
            }
        }

        // Stream is dropped here, should trigger cleanup
        drop(messages);

        // Give some time for cleanup
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        let current_count = count_claude_processes();
        println!("Claude processes after query {i}: {current_count}");

        if current_count > initial_count + 1 {
            println!(
                "⚠️ WARNING: Process leak detected! Expected at most {} processes, found {}",
                initial_count + 1,
                current_count
            );
        }
    }

    // Final check after a delay
    println!("\n--- Final check after 2 seconds ---");
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    let final_count = count_claude_processes();
    println!("Final Claude processes: {final_count}");

    if final_count > initial_count {
        println!(
            "❌ FAILED: Process leak detected! {} zombie processes remain",
            final_count - initial_count
        );
    } else {
        println!("✅ SUCCESS: No process leak detected!");
    }

    Ok(())
}

fn count_claude_processes() -> usize {
    let output = Command::new("sh")
        .arg("-c")
        .arg("ps aux | grep -v grep | grep claude | wc -l")
        .output()
        .expect("Failed to execute ps command");

    let count_str = String::from_utf8_lossy(&output.stdout);
    count_str.trim().parse().unwrap_or(0)
}
