# Phase 1: Show Command Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement `audiobookctl show` command to display m4b audiobook metadata.

**Architecture:** CLI uses clap with derive macros. Metadata module reads m4b files using mp4ameta crate. Output supports pretty-print (default), JSON, and single-field modes.

**Tech Stack:** Rust, clap (CLI), mp4ameta (m4b reading), serde/serde_json (serialization), colored (terminal output)

---

## Task 1: Add Dependencies

**Files:**
- Modify: `Cargo.toml`

**Step 1: Add required dependencies to Cargo.toml**

```toml
[package]
name = "audiobookctl"
version = "0.1.0"
edition = "2021"
description = "CLI tool for reading, editing, and organizing m4b audiobook metadata"
license = "MIT"

[dependencies]
clap = { version = "4", features = ["derive"] }
mp4ameta = "0.11"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
colored = "2"
thiserror = "2"
anyhow = "1"

[dev-dependencies]
tempfile = "3"
assert_cmd = "2"
predicates = "3"
```

**Step 2: Verify dependencies resolve**

Run: `cargo check`
Expected: Compiles successfully

**Step 3: Commit**

```bash
git add Cargo.toml
git commit -m "feat: add dependencies for show command"
```

---

## Task 2: Define Metadata Struct

**Files:**
- Create: `src/metadata/mod.rs`
- Create: `src/metadata/fields.rs`
- Modify: `src/main.rs` (add module declaration)

**Step 1: Create metadata module structure**

Create `src/metadata/mod.rs`:
```rust
mod fields;

pub use fields::AudiobookMetadata;
```

**Step 2: Define the AudiobookMetadata struct**

Create `src/metadata/fields.rs`:
```rust
use serde::{Deserialize, Serialize};

/// Comprehensive audiobook metadata from m4b files
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AudiobookMetadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub narrator: Option<String>,
    pub series: Option<String>,
    pub series_position: Option<u32>,
    pub year: Option<u32>,
    pub description: Option<String>,
    pub publisher: Option<String>,
    pub genre: Option<String>,
    pub duration_seconds: Option<u64>,
    pub chapter_count: Option<u32>,
    pub isbn: Option<String>,
    pub asin: Option<String>,
    /// Cover art info (not the bytes - just format and dimensions if available)
    pub cover_info: Option<String>,
}
```

**Step 3: Add module to main.rs**

Modify `src/main.rs` to add at the top:
```rust
mod metadata;
```

**Step 4: Verify it compiles**

Run: `cargo check`
Expected: Compiles successfully

**Step 5: Commit**

```bash
git add src/metadata/
git commit -m "feat: define AudiobookMetadata struct"
```

---

## Task 3: Implement Metadata Reader

**Files:**
- Create: `src/metadata/reader.rs`
- Modify: `src/metadata/mod.rs`

**Step 1: Write unit test for reader**

Add to `src/metadata/reader.rs`:
```rust
use crate::metadata::AudiobookMetadata;
use anyhow::{Context, Result};
use std::path::Path;

/// Read metadata from an m4b file
pub fn read_metadata(path: &Path) -> Result<AudiobookMetadata> {
    let tag = mp4ameta::Tag::read_from_path(path)
        .with_context(|| format!("Failed to read m4b file: {}", path.display()))?;

    Ok(AudiobookMetadata {
        title: tag.title().map(String::from),
        author: tag.artist().map(String::from),
        narrator: tag.take_strings_of(&mp4ameta::FreeformIdent::new(
            "com.apple.iTunes",
            "NARRATOR",
        )).next().or_else(|| {
            // Try standard narrator atom
            None // mp4ameta doesn't have direct narrator support
        }),
        series: tag.tv_show_name().map(String::from),
        series_position: tag.tv_episode_number().map(|n| n as u32),
        year: tag.year().and_then(|s| s.parse().ok()),
        description: tag.description().map(String::from),
        publisher: None, // mp4ameta doesn't expose publisher directly
        genre: tag.genre().map(String::from),
        duration_seconds: tag.duration().map(|d| d.as_secs()),
        chapter_count: None, // Would need separate chapter parsing
        isbn: tag.take_strings_of(&mp4ameta::FreeformIdent::new(
            "com.apple.iTunes",
            "ISBN",
        )).next(),
        asin: tag.take_strings_of(&mp4ameta::FreeformIdent::new(
            "com.apple.iTunes",
            "ASIN",
        )).next(),
        cover_info: tag.artwork().map(|art| {
            let fmt = match art.fmt {
                mp4ameta::ImgFmt::Jpeg => "JPEG",
                mp4ameta::ImgFmt::Png => "PNG",
                mp4ameta::ImgFmt::Bmp => "BMP",
            };
            format!("embedded ({} bytes, {})", art.data.len(), fmt)
        }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_read_nonexistent_file_returns_error() {
        let path = PathBuf::from("/nonexistent/file.m4b");
        let result = read_metadata(&path);
        assert!(result.is_err());
    }
}
```

**Step 2: Update metadata/mod.rs to export reader**

```rust
mod fields;
mod reader;

pub use fields::AudiobookMetadata;
pub use reader::read_metadata;
```

**Step 3: Run test to verify error handling works**

Run: `cargo test test_read_nonexistent`
Expected: PASS

**Step 4: Commit**

```bash
git add src/metadata/
git commit -m "feat: implement m4b metadata reader"
```

---

## Task 4: Implement CLI Structure with Clap

**Files:**
- Create: `src/cli.rs`
- Modify: `src/main.rs`

**Step 1: Create CLI module**

Create `src/cli.rs`:
```rust
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
```

**Step 2: Update main.rs to use CLI**

Replace `src/main.rs` content:
```rust
mod cli;
mod metadata;

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

mod commands {
    pub mod show;
}
```

**Step 3: Verify it compiles (will fail - show command not yet implemented)**

Run: `cargo check`
Expected: Error about missing commands::show module

**Step 4: Commit CLI structure**

```bash
git add src/cli.rs
git commit -m "feat: add CLI structure with clap"
```

---

## Task 5: Implement Show Command

**Files:**
- Create: `src/commands/mod.rs`
- Create: `src/commands/show.rs`
- Modify: `src/main.rs`

**Step 1: Create commands module**

Create `src/commands/mod.rs`:
```rust
pub mod show;
```

**Step 2: Implement show command**

Create `src/commands/show.rs`:
```rust
use crate::metadata::{read_metadata, AudiobookMetadata};
use anyhow::{bail, Result};
use colored::Colorize;
use std::path::Path;

pub fn run(path: &Path, json: bool, field: Option<&str>, quiet: bool) -> Result<()> {
    let metadata = read_metadata(path)?;

    if let Some(field_name) = field {
        print_single_field(&metadata, field_name)?;
    } else if json {
        print_json(&metadata)?;
    } else {
        print_pretty(&metadata, path, quiet)?;
    }

    Ok(())
}

fn print_single_field(metadata: &AudiobookMetadata, field: &str) -> Result<()> {
    let value = match field {
        "title" => metadata.title.as_deref(),
        "author" => metadata.author.as_deref(),
        "narrator" => metadata.narrator.as_deref(),
        "series" => metadata.series.as_deref(),
        "description" => metadata.description.as_deref(),
        "publisher" => metadata.publisher.as_deref(),
        "genre" => metadata.genre.as_deref(),
        "isbn" => metadata.isbn.as_deref(),
        "asin" => metadata.asin.as_deref(),
        "cover_info" => metadata.cover_info.as_deref(),
        "year" => {
            if let Some(y) = metadata.year {
                println!("{}", y);
            }
            return Ok(());
        }
        "series_position" => {
            if let Some(p) = metadata.series_position {
                println!("{}", p);
            }
            return Ok(());
        }
        "duration_seconds" => {
            if let Some(d) = metadata.duration_seconds {
                println!("{}", d);
            }
            return Ok(());
        }
        "chapter_count" => {
            if let Some(c) = metadata.chapter_count {
                println!("{}", c);
            }
            return Ok(());
        }
        _ => bail!("Unknown field: {}. Valid fields: title, author, narrator, series, series_position, year, description, publisher, genre, isbn, asin, duration_seconds, chapter_count, cover_info", field),
    };

    if let Some(v) = value {
        println!("{}", v);
    }
    Ok(())
}

fn print_json(metadata: &AudiobookMetadata) -> Result<()> {
    let json = serde_json::to_string_pretty(metadata)?;
    println!("{}", json);
    Ok(())
}

fn print_pretty(metadata: &AudiobookMetadata, path: &Path, quiet: bool) -> Result<()> {
    if !quiet {
        println!("{}", path.display().to_string().bold());
        println!("{}", "─".repeat(40));
    }

    print_field("Title", metadata.title.as_deref());
    print_field("Author", metadata.author.as_deref());
    print_field("Narrator", metadata.narrator.as_deref());

    if metadata.series.is_some() || metadata.series_position.is_some() {
        let series_str = match (&metadata.series, metadata.series_position) {
            (Some(s), Some(p)) => format!("{} #{}", s, p),
            (Some(s), None) => s.clone(),
            (None, Some(p)) => format!("#{}", p),
            (None, None) => unreachable!(),
        };
        print_field("Series", Some(&series_str));
    }

    if let Some(year) = metadata.year {
        print_field("Year", Some(&year.to_string()));
    }

    print_field("Genre", metadata.genre.as_deref());
    print_field("Publisher", metadata.publisher.as_deref());

    if let Some(duration) = metadata.duration_seconds {
        let hours = duration / 3600;
        let minutes = (duration % 3600) / 60;
        let seconds = duration % 60;
        print_field("Duration", Some(&format!("{:02}:{:02}:{:02}", hours, minutes, seconds)));
    }

    if let Some(chapters) = metadata.chapter_count {
        print_field("Chapters", Some(&chapters.to_string()));
    }

    print_field("ISBN", metadata.isbn.as_deref());
    print_field("ASIN", metadata.asin.as_deref());
    print_field("Cover", metadata.cover_info.as_deref());

    if let Some(desc) = &metadata.description {
        println!();
        println!("{}", "Description:".cyan());
        // Wrap description at 80 chars
        for line in textwrap_simple(desc, 80) {
            println!("  {}", line);
        }
    }

    Ok(())
}

fn print_field(label: &str, value: Option<&str>) {
    if let Some(v) = value {
        println!("{:>12}: {}", label.cyan(), v);
    }
}

/// Simple text wrapping without external dependency
fn textwrap_simple(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    for paragraph in text.split('\n') {
        let words: Vec<&str> = paragraph.split_whitespace().collect();
        let mut current_line = String::new();

        for word in words {
            if current_line.is_empty() {
                current_line = word.to_string();
            } else if current_line.len() + 1 + word.len() <= width {
                current_line.push(' ');
                current_line.push_str(word);
            } else {
                lines.push(current_line);
                current_line = word.to_string();
            }
        }

        if !current_line.is_empty() {
            lines.push(current_line);
        }
    }
    lines
}
```

**Step 3: Update main.rs**

Replace `src/main.rs`:
```rust
mod cli;
mod commands;
mod metadata;

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
```

**Step 4: Verify it compiles**

Run: `cargo check`
Expected: Compiles successfully

**Step 5: Commit**

```bash
git add src/
git commit -m "feat: implement show command with pretty, json, and field output"
```

---

## Task 6: Add Integration Tests

**Files:**
- Create: `tests/integration/mod.rs`
- Create: `tests/integration/show_test.rs`
- Create: `tests/cli_tests.rs`

**Step 1: Create test structure**

Create `tests/cli_tests.rs`:
```rust
use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_show_missing_file_returns_error() {
    let mut cmd = Command::cargo_bin("audiobookctl").unwrap();
    cmd.args(["show", "/nonexistent/file.m4b"]);
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Failed to read m4b file"));
}

#[test]
fn test_show_help() {
    let mut cmd = Command::cargo_bin("audiobookctl").unwrap();
    cmd.args(["show", "--help"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Display metadata"));
}

#[test]
fn test_unknown_field_returns_error() {
    let mut cmd = Command::cargo_bin("audiobookctl").unwrap();
    cmd.args(["show", "--field", "invalid_field", "/nonexistent/file.m4b"]);
    // File error comes first, but if we had a file, field error would show
    cmd.assert().failure();
}

#[test]
fn test_version() {
    let mut cmd = Command::cargo_bin("audiobookctl").unwrap();
    cmd.arg("--version");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("audiobookctl"));
}
```

**Step 2: Run tests**

Run: `cargo test`
Expected: All tests pass

**Step 3: Commit**

```bash
git add tests/
git commit -m "test: add CLI integration tests"
```

---

## Task 7: Run Full CI Checks

**Files:** None (verification only)

**Step 1: Run format check**

Run: `cargo fmt --check`
Expected: No formatting issues (or run `cargo fmt` to fix)

**Step 2: Run clippy**

Run: `cargo clippy -- -D warnings`
Expected: No warnings

**Step 3: Run all tests**

Run: `cargo test`
Expected: All tests pass

**Step 4: Fix any issues found**

If clippy or fmt report issues, fix them and commit:
```bash
cargo fmt
git add -u
git commit -m "style: apply rustfmt and clippy fixes"
```

---

## Task 8: Update Beads Issue

**Step 1: Close the Phase 1 issue**

Run: `bd close audiobookctl-cmk`

**Step 2: Commit beads update**

```bash
git add .beads/
git commit -m "chore: close Phase 1 issue"
```

---

## Task 9: Merge to Main

**Step 1: Ensure all changes are committed**

Run: `git status`
Expected: Clean working tree

**Step 2: Switch to main and merge**

```bash
cd /home/jsvana/projects/audiobookctl
git merge feature/phase1-show
```

**Step 3: Clean up worktree**

```bash
git worktree remove .worktrees/phase1-show
git branch -d feature/phase1-show
```

---

## Summary

After completing all tasks, you will have:
- `audiobookctl show file.m4b` - pretty-printed metadata
- `audiobookctl show file.m4b --json` - JSON output
- `audiobookctl show file.m4b --field author` - single field output
- Full test coverage
- Passing CI (fmt, clippy, test)
