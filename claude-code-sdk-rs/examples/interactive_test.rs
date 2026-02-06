//! Interactive test example for various API modes

use std::io::{self, Write};
use tracing::Level;

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    println!("=== Claude Code SDK Interactive Test ===\n");
    println!("This is a mock test that doesn't require Claude CLI.\n");

    loop {
        println!("\nSelect test mode:");
        println!("1. OneShot Mode (single query)");
        println!("2. Batch Mode (multiple queries)");
        println!("3. Interactive Mode (conversation)");
        println!("4. Performance Test");
        println!("5. Exit");
        print!("\nYour choice (1-5): ");
        io::stdout().flush().unwrap();

        let mut choice = String::new();
        io::stdin().read_line(&mut choice).unwrap();

        match choice.trim() {
            "1" => test_oneshot_mode().await,
            "2" => test_batch_mode().await,
            "3" => test_interactive_mode().await,
            "4" => test_performance().await,
            "5" => {
                println!("Goodbye!");
                break;
            },
            _ => println!("Invalid choice. Please try again."),
        }
    }
}

async fn test_oneshot_mode() {
    println!("\n--- OneShot Mode Test ---");
    print!("Enter your question: ");
    io::stdout().flush().unwrap();

    let mut prompt = String::new();
    io::stdin().read_line(&mut prompt).unwrap();

    // Simulate response
    let response = mock_query(prompt.trim()).await;
    println!("\nResponse: {response}");
}

async fn test_batch_mode() {
    println!("\n--- Batch Mode Test ---");
    println!("Enter questions (one per line, empty line to finish):");

    let mut prompts = Vec::new();
    loop {
        let mut prompt = String::new();
        io::stdin().read_line(&mut prompt).unwrap();
        let prompt = prompt.trim();

        if prompt.is_empty() {
            break;
        }
        prompts.push(prompt.to_string());
    }

    if prompts.is_empty() {
        println!("No questions entered.");
        return;
    }

    println!("\nProcessing {} queries...", prompts.len());
    let start = std::time::Instant::now();

    // Simulate batch processing
    for (i, prompt) in prompts.iter().enumerate() {
        let response = mock_query(prompt).await;
        println!("\nQuery {}: {}", i + 1, prompt);
        println!("Response: {response}");
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    println!("\nBatch completed in {:?}", start.elapsed());
}

async fn test_interactive_mode() {
    println!("\n--- Interactive Mode Test ---");
    println!("Starting conversation (type 'exit' to end):");

    let mut context = Vec::new();

    loop {
        print!("\nYou: ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let input = input.trim();

        if input == "exit" {
            println!("Ending conversation.");
            break;
        }

        context.push(format!("User: {input}"));

        // Simulate contextual response
        let response = mock_contextual_response(input, &context).await;
        println!("Assistant: {response}");

        context.push(format!("Assistant: {response}"));
    }
}

async fn test_performance() {
    println!("\n--- Performance Test ---");
    print!("Number of queries to test (1-100): ");
    io::stdout().flush().unwrap();

    let mut count_str = String::new();
    io::stdin().read_line(&mut count_str).unwrap();

    let count: usize = match count_str.trim().parse() {
        Ok(n) if n > 0 && n <= 100 => n,
        _ => {
            println!("Invalid number. Using 10.");
            10
        },
    };

    println!("\nRunning {count} queries...");
    let start = std::time::Instant::now();

    let mut total_time = std::time::Duration::ZERO;
    let mut min_time = std::time::Duration::MAX;
    let mut max_time = std::time::Duration::ZERO;

    for i in 1..=count {
        let query_start = std::time::Instant::now();
        let _ = mock_query(&format!("Query {i}")).await;
        let query_time = query_start.elapsed();

        total_time += query_time;
        min_time = min_time.min(query_time);
        max_time = max_time.max(query_time);

        print!(".");
        io::stdout().flush().unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }

    let total_elapsed = start.elapsed();
    let avg_time = total_time / count as u32;

    println!("\n\nPerformance Results:");
    println!("Total queries: {count}");
    println!("Total time: {total_elapsed:?}");
    println!("Average time per query: {avg_time:?}");
    println!("Min query time: {min_time:?}");
    println!("Max query time: {max_time:?}");
    println!(
        "Queries per second: {:.2}",
        count as f64 / total_elapsed.as_secs_f64()
    );
}

async fn mock_query(prompt: &str) -> String {
    // Simulate processing delay
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Generate mock responses
    match prompt.to_lowercase().as_str() {
        s if s.contains("hello") => "Hello! How can I help you today?".to_string(),
        s if s.contains("weather") => {
            "I'm a mock API and don't have access to real weather data.".to_string()
        },
        s if s.contains("capital") && s.contains("france") => {
            "The capital of France is Paris.".to_string()
        },
        s if s.contains("2 + 2") || s.contains("2+2") => "2 + 2 = 4".to_string(),
        s if s.contains("squared") => {
            if let Some(num) = s.split_whitespace().find_map(|w| w.parse::<i32>().ok()) {
                format!("{} squared is {}", num, num * num)
            } else {
                "Please provide a number to square.".to_string()
            }
        },
        _ => format!("Mock response to: {prompt}"),
    }
}

async fn mock_contextual_response(prompt: &str, context: &[String]) -> String {
    // Simulate contextual understanding
    if context.len() > 2 && prompt.contains("what") && prompt.contains("said") {
        return "Based on our conversation, we discussed various topics.".to_string();
    }

    mock_query(prompt).await
}
