use anyhow::{bail, Context, Result};
use colored::Colorize;
use std::io::{self, Write};
use std::path::PathBuf;

use crate::config::Config;
use crate::organize::PLACEHOLDERS;

/// Run the init command - interactively create a config file
pub fn run(force: bool) -> Result<()> {
    let config_path = Config::config_path()?;

    // Check if config already exists
    if config_path.exists() && !force {
        eprintln!(
            "{}: Config already exists at {}",
            "Error".red().bold(),
            config_path.display()
        );
        eprintln!();
        eprintln!("Use {} to overwrite.", "--force".cyan());
        bail!("Config file already exists");
    }

    println!("{}", "audiobookctl configuration".bold());
    println!();
    println!(
        "This will create a config file at: {}",
        config_path.display().to_string().cyan()
    );
    println!();

    // Get format string
    let format = prompt_format()?;

    // Get destination directory
    let dest = prompt_destination()?;

    // Create config directory
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory {:?}", parent))?;
    }

    // Write config file
    let config_content = format!(
        r#"# audiobookctl configuration
# See 'audiobookctl fields' for available placeholders

[organize]
format = "{}"
dest = "{}"
"#,
        format,
        dest.display()
    );

    std::fs::write(&config_path, &config_content)
        .with_context(|| format!("Failed to write {:?}", config_path))?;

    println!();
    println!(
        "{} Config written to {}",
        "✓".green(),
        config_path.display()
    );
    println!();
    println!("You can now use:");
    println!(
        "  {} - organize files from a source directory",
        "audiobookctl organize --source <dir>".cyan()
    );
    println!(
        "  {} - check and fix existing library structure",
        "audiobookctl fix".cyan()
    );

    Ok(())
}

fn prompt_format() -> Result<String> {
    println!("{}", "Step 1: Choose a format string".bold());
    println!();
    println!("The format string defines how your audiobooks will be organized.");
    println!("Available placeholders:");
    println!();

    for (name, description) in PLACEHOLDERS {
        println!("  {{{}}} - {}", name.cyan(), description);
    }

    println!();
    println!("{}", "Examples:".bold());
    println!();
    println!("  {}", "{author}/{title}/{filename}".green());
    println!("    → Andy Weir/Project Hail Mary/book.m4b");
    println!();
    println!(
        "  {}",
        "{author}/{series}/{series_position:02} - {title}/{filename}".green()
    );
    println!("    → Brandon Sanderson/Mistborn/01 - The Final Empire/book.m4b");
    println!();
    println!("  {}", "{author}/{year} - {title}/{filename}".green());
    println!("    → Andy Weir/2021 - Project Hail Mary/book.m4b");
    println!();

    let default = "{author}/{series}/{title}/{filename}";
    print!("Enter format string [{}]: ", default.green());
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim();

    let format = if input.is_empty() {
        default.to_string()
    } else {
        input.to_string()
    };

    // Validate the format string
    crate::organize::FormatTemplate::parse(&format).context("Invalid format string")?;

    println!();
    Ok(format)
}

fn prompt_destination() -> Result<PathBuf> {
    println!("{}", "Step 2: Choose a destination directory".bold());
    println!();
    println!("This is where your organized audiobooks will be stored.");
    println!();

    // Try to suggest a reasonable default
    let default = dirs::audio_dir()
        .or_else(dirs::home_dir)
        .map(|p| p.join("Audiobooks"))
        .unwrap_or_else(|| PathBuf::from("/audiobooks"));

    print!(
        "Enter destination directory [{}]: ",
        default.display().to_string().green()
    );
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim();

    let dest = if input.is_empty() {
        default
    } else if let Some(stripped) = input.strip_prefix("~/") {
        // Expand ~ to home directory
        if let Some(home) = dirs::home_dir() {
            home.join(stripped)
        } else {
            PathBuf::from(input)
        }
    } else {
        PathBuf::from(input)
    };

    // Warn if directory doesn't exist (but don't fail)
    if !dest.exists() {
        println!();
        println!(
            "{}: Directory {} does not exist yet.",
            "Note".yellow(),
            dest.display()
        );
        println!("It will be created when you run 'organize --no-dry-run'.");
    }

    println!();
    Ok(dest)
}
