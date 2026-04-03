use cc_sdk::llm;

#[tokio::main]
async fn main() {
    // Enable debug logging for cc_sdk
    unsafe { std::env::set_var("RUST_LOG", "cc_sdk=debug"); }
    tracing_subscriber::fmt::init();

    eprintln!("[test] starting llm::query...");
    match llm::query("What is 2 + 2? Reply with just the number.", None).await {
        Ok(resp) => {
            println!("Text: [{}]", resp.text);
            println!("Model: {:?}", resp.model);
            if resp.text.contains("4") {
                println!("PASS");
            } else {
                println!("FAIL: expected '4'");
            }
        }
        Err(e) => {
            eprintln!("FAIL: {e}");
        }
    }
}
