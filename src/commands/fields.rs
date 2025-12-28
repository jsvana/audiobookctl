use anyhow::Result;

use crate::organize::PLACEHOLDERS;

/// Run the fields command - list available format placeholders
pub fn run() -> Result<()> {
    println!("Available format placeholders:");
    println!();

    for (name, description) in PLACEHOLDERS {
        println!("  {{{}}}  - {}", name, description);
    }

    println!();
    println!("Example format: \"{{author}}/{{series}}/{{title}}/{{filename}}\"");
    println!();
    println!("Padding: Use {{series_position:02}} for zero-padded numbers (e.g., 01, 02)");

    Ok(())
}
