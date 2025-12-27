mod cli;
mod metadata;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Show { file, json, field } => {
            // Placeholder - will be implemented in Task 5
            println!(
                "Show command for: {:?}, json={}, field={:?}",
                file, json, field
            );
        }
    }

    Ok(())
}
