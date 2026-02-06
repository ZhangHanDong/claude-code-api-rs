use futures::StreamExt;
use nexus_claude::{Result, query};
use std::process::Command;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Testing concurrent process leak fix...\n");

    // Check initial Claude processes
    let initial_count = count_claude_processes();
    println!("Initial Claude processes: {initial_count}");

    println!("\n--- Running 5 concurrent queries ---");

    // Create 5 concurrent queries
    let mut handles = vec![];
    for i in 1..=5 {
        let handle = tokio::spawn(async move {
            println!("Starting query {i}");

            // Create a query and consume only the first message
            let mut messages = query(format!("Say 'Concurrent test {i}'"), None).await?;

            // Only take the first message then drop the stream
            if let Some(msg) = messages.next().await {
                match msg {
                    Ok(_) => println!("Query {i} got response"),
                    Err(e) => println!("Query {i} error: {e}"),
                }
            }

            // Stream is dropped here, should trigger cleanup
            drop(messages);

            println!("Query {i} completed");
            Ok::<(), nexus_claude::SdkError>(())
        });
        handles.push(handle);
    }

    // Wait for all queries to complete
    for handle in handles {
        let _ = handle.await;
    }

    // Give some time for cleanup
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    let concurrent_count = count_claude_processes();
    println!("\nClaude processes after concurrent queries: {concurrent_count}");

    if concurrent_count > initial_count + 1 {
        println!(
            "⚠️ WARNING: Possible process leak! Expected at most {} processes, found {}",
            initial_count + 1,
            concurrent_count
        );
    }

    // Final check after more delay
    println!("\n--- Final check after 3 more seconds ---");
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    let final_count = count_claude_processes();
    println!("Final Claude processes: {final_count}");

    if final_count > initial_count {
        println!(
            "❌ FAILED: Process leak detected! {} zombie processes remain",
            final_count - initial_count
        );

        // Show which processes are still running
        let output = Command::new("sh")
            .arg("-c")
            .arg("ps aux | grep -v grep | grep claude")
            .output()
            .expect("Failed to execute ps command");
        println!("\nRemaining Claude processes:");
        println!("{}", String::from_utf8_lossy(&output.stdout));
    } else {
        println!("✅ SUCCESS: No process leak detected in concurrent scenario!");
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
