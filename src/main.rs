mod cli;
mod commands;
mod metadata;
mod safety;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Show { file, json, field } => {
            commands::show::run(&file, json, field.as_deref(), cli.quiet)?;
        }
    }

    Ok(())
}
