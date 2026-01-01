use clap::{Parser, Subcommand};
use std::path::PathBuf;

use crate::lookup::TrustedSource;

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

        /// Delete backup after verifying changes
        #[arg(long)]
        commit: bool,

        /// Delete all backup files recursively
        #[arg(long)]
        commit_all: bool,
    },

    /// Search for audiobooks by author, title, or ASIN (no file required)
    Search {
        /// Search by title
        #[arg(long)]
        title: Option<String>,

        /// Search by author
        #[arg(long)]
        author: Option<String>,

        /// Search by ASIN (direct lookup)
        #[arg(long)]
        asin: Option<String>,

        /// Output as JSON
        #[arg(long)]
        json: bool,
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

        /// Trust this source and auto-accept its values (skip editor for conflicts)
        #[arg(long, value_enum)]
        trust_source: Option<TrustedSource>,
    },

    /// Look up metadata for all audiobooks in a directory
    LookupAll {
        /// Directory to scan
        dir: std::path::PathBuf,

        /// Auto-apply when all sources agree (skip editor)
        #[arg(long)]
        auto_accept: bool,

        /// Actually apply changes (default: dry-run)
        #[arg(long)]
        no_dry_run: bool,

        /// Skip confirmation prompts
        #[arg(long)]
        yes: bool,

        /// Skip creating backup files
        #[arg(long = "no-backup-i-void-my-warranty")]
        no_backup: bool,

        /// Trust this source and auto-accept its values (skip editor for conflicts)
        #[arg(long, value_enum)]
        trust_source: Option<TrustedSource>,
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

    /// Manage backup files
    Backups {
        #[command(subcommand)]
        action: BackupsAction,
    },

    /// Manage pending edits
    Pending {
        #[command(subcommand)]
        action: PendingAction,
    },
}

#[derive(Subcommand)]
pub enum BackupsAction {
    /// List all backup files and total size
    List {
        /// Directory to scan (current directory if not specified)
        #[arg(default_value = ".")]
        dir: std::path::PathBuf,
    },
    /// Clean backup files
    Clean {
        /// Directory to scan (current directory if not specified)
        #[arg(default_value = ".")]
        dir: std::path::PathBuf,

        /// Delete all backups without prompting for each
        #[arg(long)]
        all: bool,

        /// Skip confirmation prompt (with --all)
        #[arg(long)]
        yes: bool,
    },
}

#[derive(Subcommand)]
pub enum PendingAction {
    /// List all pending edits
    List {
        /// Show diff preview for each pending edit
        #[arg(long)]
        diff: bool,
    },
    /// Show diff for a specific pending edit
    Show {
        /// Path to the m4b file
        file: PathBuf,
    },
    /// Apply pending edits
    Apply {
        /// Path to specific m4b file (applies all if not specified)
        file: Option<PathBuf>,

        /// Skip confirmation prompt
        #[arg(long)]
        yes: bool,

        /// Skip creating backup files
        #[arg(long = "no-backup-i-void-my-warranty")]
        no_backup: bool,
    },
    /// Clear pending edits
    Clear {
        /// Path to specific m4b file (clears all if not specified)
        file: Option<PathBuf>,
    },
}
