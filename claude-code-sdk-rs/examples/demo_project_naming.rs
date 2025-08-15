//! Demo: Project Naming Convention
//!
//! Shows how the project naming works:
//! - Question set: qs00035.txt
//! - Question 43 -> Project: q0003500043
//! - Question 44 -> Project: q0003500044

use regex::Regex;

fn demonstrate_naming() {
    println!("🦀 Rust Question Processor - Project Naming Demo");
    println!("{}", "=".repeat(50));

    // Example question set files
    let question_sets = vec![
        ("qs00001.txt", vec![1, 2, 3, 4, 5]),
        ("qs00002.txt", vec![1, 2, 3, 4, 5]),
        ("qs00035.txt", vec![43, 44, 45, 46, 47]),
    ];

    for (qs_file, questions) in question_sets {
        println!("\nQuestion Set: {}", qs_file);

        // Extract qs number
        let qs_number = qs_file
            .strip_prefix("qs")
            .and_then(|s| s.strip_suffix(".txt"))
            .unwrap_or("00000");

        println!("  QS Number: {}", qs_number);
        println!("  Projects:");

        for &q_num in &questions {
            let project_name = format!("q{}{:05}", qs_number, q_num);
            println!("    Question {} → annotations/{}", q_num, project_name);
        }
    }

    println!("\n📁 Directory Structure:");
    println!("```");
    println!(".");
    println!("├── qs/");
    println!("│   ├── qs00001.txt");
    println!("│   ├── qs00002.txt");
    println!("│   └── qs00035.txt");
    println!("└── annotations/");
    println!("    ├── q0000100001/  # From qs00001.txt, question 1");
    println!("    ├── q0000100002/  # From qs00001.txt, question 2");
    println!("    ├── q0003500043/  # From qs00035.txt, question 43");
    println!("    └── q0003500044/  # From qs00035.txt, question 44");
    println!("```");

    // Show how to parse a question line
    println!("\n📝 Parsing Question Lines:");
    let question_regex = Regex::new(r"^(\d+)\.\s*(.+)$").unwrap();

    let sample_lines = vec![
        "43. Implement a lock-free concurrent queue using atomic operations",
        "44. Create a procedural macro that derives a serialization trait",
    ];

    for line in sample_lines {
        if let Some(captures) = question_regex.captures(line) {
            let q_num: u32 = captures[1].parse().unwrap();
            let q_text = &captures[2];
            println!("\n  Line: {}", line);
            println!("  → Question Number: {}", q_num);
            println!("  → Question Text: {}", q_text);
            println!("  → Project Name (for qs00035): q00035{:05}", q_num);
        }
    }
}

fn main() {
    demonstrate_naming();

    println!("\n✅ This naming convention ensures:");
    println!("  - Unique project names across all question sets");
    println!("  - Easy identification of source (qs file + question number)");
    println!("  - Sorted order in file listings");
    println!("  - Support for up to 99,999 question sets");
    println!("  - Support for up to 99,999 questions per set");
}
