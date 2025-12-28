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

    /// Edit metadata in $EDITOR with diff preview
    Edit {
        /// Path to the m4b file
        file: Option<PathBuf>,

        /// Actually apply changes (default: dry-run)
        #[arg(long)]
        no_dry_run: bool,

        /// Skip confirmation prompt
        #[arg(long)]
        yes: bool,

        /// Skip creating backup file
        #[arg(long = "no-backup-i-void-my-warranty")]
        no_backup: bool,

        /// Clear pending edit(s)
        #[arg(long)]
        clear: bool,

        /// Delete backup after verifying changes
        #[arg(long)]
        commit: bool,

        /// Delete all backup files recursively
        #[arg(long)]
        commit_all: bool,
    },

    /// Look up metadata from online sources (Audnexus, Open Library)
    Lookup {
        /// Path to the m4b file
        file: PathBuf,

        /// Actually apply changes (default: dry-run)
        #[arg(long)]
        no_dry_run: bool,

        /// Skip confirmation prompt
        #[arg(long)]
        yes: bool,

        /// Skip creating backup file
        #[arg(long = "no-backup-i-void-my-warranty")]
        no_backup: bool,
    },

    /// Organize audiobooks into a structured directory format
    Organize {
        /// Source directory containing .m4b files to organize
        #[arg(long)]
        source: PathBuf,

        /// Destination directory (uses config default if not specified)
        #[arg(long)]
        dest: Option<PathBuf>,

        /// Format string for directory structure (uses config default if not specified)
        #[arg(long)]
        format: Option<String>,

        /// Actually copy files (default: dry-run)
        #[arg(long)]
        no_dry_run: bool,

        /// Allow files with missing metadata (placed in __uncategorized__)
        #[arg(long)]
        allow_uncategorized: bool,

        /// Show source→dest list instead of tree view
        #[arg(long)]
        list: bool,
    },

    /// Scan organized library and fix non-compliant paths
    Fix {
        /// Library directory to scan (uses config default if not specified)
        #[arg(long)]
        dest: Option<PathBuf>,

        /// Actually move files (default: dry-run)
        #[arg(long)]
        no_dry_run: bool,

        /// Show all files including compliant ones
        #[arg(long)]
        show_all: bool,
    },

    /// List available format placeholders for organizing
    Fields,

    /// Create a config file interactively
    Init {
        /// Overwrite existing config file
        #[arg(long)]
        force: bool,
    },
}
