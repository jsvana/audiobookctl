mod cli;
mod commands;
mod editor;
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
        Commands::Edit {
            file,
            no_dry_run,
            yes,
            no_backup,
            clear,
            commit,
            commit_all,
        } => {
            commands::edit::run(
                file.as_deref(),
                no_dry_run,
                yes,
                no_backup,
                clear,
                commit,
                commit_all,
            )?;
        }
    }

    Ok(())
}
