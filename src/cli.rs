use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "audiobookctl")]
#[command(about = "CLI tool for reading, editing, and organizing m4b audiobook metadata")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Increase output verbosity
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Suppress non-essential output
    #[arg(short, long, global = true)]
    pub quiet: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Display metadata for an m4b file
    Show {
        /// Path to the m4b file
        file: PathBuf,

        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// Show only a specific field
        #[arg(long)]
        field: Option<String>,
    },
}
