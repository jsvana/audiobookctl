mod cli;
mod commands;
mod config;
mod editor;
mod lookup;
mod metadata;
mod organize;
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
            commit,
            commit_all,
        } => {
            commands::edit::run(
                file.as_deref(),
                no_dry_run,
                yes,
                no_backup,
                commit,
                commit_all,
            )?;
        }
        Commands::Lookup {
            file,
            no_dry_run,
            yes,
            no_backup,
        } => {
            commands::lookup::run(&file, no_dry_run, yes, no_backup)?;
        }
        Commands::LookupAll {
            dir,
            auto_accept,
            no_dry_run,
            yes,
            no_backup,
        } => {
            commands::lookup_all::run(&dir, auto_accept, no_dry_run, yes, no_backup)?;
        }
        Commands::Organize {
            source,
            dest,
            format,
            no_dry_run,
            allow_uncategorized,
            list,
        } => {
            commands::organize::run(
                &source,
                dest.as_ref(),
                format.as_deref(),
                no_dry_run,
                allow_uncategorized,
                list,
            )?;
        }
        Commands::Fix {
            dest,
            no_dry_run,
            show_all,
        } => {
            commands::fix::run(dest.as_ref(), no_dry_run, show_all)?;
        }
        Commands::Fields => {
            commands::fields::run()?;
        }
        Commands::Init { force } => {
            commands::init::run(force)?;
        }
        Commands::Backups { action } => {
            use cli::BackupsAction;
            match action {
                BackupsAction::List { dir } => {
                    commands::backups::list(&dir)?;
                }
                BackupsAction::Clean { dir, all, yes } => {
                    commands::backups::clean(&dir, all, yes)?;
                }
            }
        }
        Commands::Pending { action } => {
            use cli::PendingAction;
            match action {
                PendingAction::List { diff } => {
                    commands::pending::list(diff)?;
                }
                PendingAction::Show { file } => {
                    commands::pending::show(&file)?;
                }
                PendingAction::Apply {
                    file,
                    yes,
                    no_backup,
                } => {
                    commands::pending::apply(file.as_deref(), yes, no_backup)?;
                }
                PendingAction::Clear { file } => {
                    commands::pending::clear(file.as_deref())?;
                }
            }
        }
    }

    Ok(())
}
