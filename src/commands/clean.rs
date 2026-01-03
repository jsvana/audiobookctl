//! Clean command - remove unexpected files from organized library

use anyhow::{bail, Context, Result};
use colored::Colorize;
use std::collections::HashSet;
use std::path::PathBuf;
use walkdir::WalkDir;

use crate::config::Config;
use crate::database::LibraryDb;

/// Extensions recognized as auxiliary files (e.g., book.cue for book.m4b)
const AUXILIARY_EXTENSIONS: &[&str] = &["cue", "pdf", "jpg", "png"];

/// Check if a file is a hash file (book.m4b.sha256) and if its matching m4b exists
fn is_orphan_hash_file(path: &std::path::Path) -> Option<bool> {
    let filename = path.file_name()?.to_str()?;
    if !filename.ends_with(".m4b.sha256") {
        return None; // Not a hash file
    }
    // Remove .sha256 to get the m4b path
    let m4b_filename = &filename[..filename.len() - 7]; // Remove ".sha256"
    let parent = path.parent()?;
    let m4b_path = parent.join(m4b_filename);
    Some(!m4b_path.exists())
}

/// Run the clean command
pub fn run(dest_override: Option<&PathBuf>, dry_run: bool) -> Result<()> {
    // Load config and get directory
    let config = Config::load().context("Failed to load config")?;
    let dir = config
        .dest(dest_override)
        .context("No destination specified. Set [organize] dest in config or use --dest")?;

    if !dir.exists() {
        bail!("Directory does not exist: {:?}", dir);
    }
    if !dir.is_dir() {
        bail!("Not a directory: {:?}", dir);
    }

    println!("Opening database in {:?}...", dir);
    let db = LibraryDb::open(&dir)?;

    // Get all known paths from database
    let known_paths: HashSet<String> = db
        .search_text("", 100000)?
        .into_iter()
        .map(|r| r.file_path)
        .collect();

    println!("Database has {} indexed audiobooks", known_paths.len());
    println!("Scanning for unexpected files...");

    let mut unexpected_m4b: Vec<std::path::PathBuf> = Vec::new();
    let mut orphan_auxiliary: Vec<std::path::PathBuf> = Vec::new();
    let mut empty_dirs: Vec<std::path::PathBuf> = Vec::new();

    // First pass: find unexpected m4b files
    for entry in WalkDir::new(&dir)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();

        if !path.is_file() {
            continue;
        }

        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_default();

        if ext != "m4b" {
            continue;
        }

        let relative = path.strip_prefix(&dir).unwrap_or(path);
        let relative_str = relative.to_string_lossy().to_string();

        if !known_paths.contains(&relative_str) {
            unexpected_m4b.push(path.to_path_buf());
        }
    }

    // Second pass: find orphan auxiliary files and hash files (no matching m4b)
    for entry in WalkDir::new(&dir)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();

        if !path.is_file() {
            continue;
        }

        // Check for orphan hash files (book.m4b.sha256 where book.m4b doesn't exist)
        if let Some(true) = is_orphan_hash_file(path) {
            orphan_auxiliary.push(path.to_path_buf());
            continue;
        }

        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_default();

        if !AUXILIARY_EXTENSIONS.contains(&ext.as_str()) {
            continue;
        }

        // Check if there's a matching m4b file
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        let parent = path.parent().unwrap_or(&dir);
        let m4b_path = parent.join(format!("{}.m4b", stem));

        if !m4b_path.exists() {
            orphan_auxiliary.push(path.to_path_buf());
        }
    }

    // Third pass: find empty directories
    for entry in WalkDir::new(&dir)
        .follow_links(true)
        .contents_first(true) // Process contents before directory
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();

        if path == dir.as_path() {
            continue; // Don't remove the root directory
        }

        if path.is_dir() {
            if let Ok(mut entries) = std::fs::read_dir(path) {
                if entries.next().is_none() {
                    empty_dirs.push(path.to_path_buf());
                }
            }
        }
    }

    // Report findings
    println!();
    if unexpected_m4b.is_empty() && orphan_auxiliary.is_empty() && empty_dirs.is_empty() {
        println!("{} No unexpected files found", "✓".green());
        return Ok(());
    }

    if !unexpected_m4b.is_empty() {
        println!(
            "{} {} unexpected .m4b file(s):",
            "Found".yellow().bold(),
            unexpected_m4b.len()
        );
        for path in &unexpected_m4b {
            println!("  {}", path.strip_prefix(&dir).unwrap_or(path).display());
        }
        println!();
    }

    if !orphan_auxiliary.is_empty() {
        println!(
            "{} {} orphan auxiliary file(s) (no matching .m4b):",
            "Found".yellow().bold(),
            orphan_auxiliary.len()
        );
        for path in &orphan_auxiliary {
            println!("  {}", path.strip_prefix(&dir).unwrap_or(path).display());
        }
        println!();
    }

    if !empty_dirs.is_empty() {
        println!(
            "{} {} empty director(y/ies):",
            "Found".yellow().bold(),
            empty_dirs.len()
        );
        for path in &empty_dirs {
            println!("  {}", path.strip_prefix(&dir).unwrap_or(path).display());
        }
        println!();
    }

    // Remove files if not dry run
    if dry_run {
        println!("{}", "Dry run - no files removed.".yellow());
        println!("Run with {} to remove files.", "--no-dry-run".cyan());
    } else {
        let mut removed = 0;

        // Remove unexpected m4b files
        for path in &unexpected_m4b {
            std::fs::remove_file(path).with_context(|| format!("Failed to remove {:?}", path))?;
            println!("  {} {}", "Removed".red(), path.display());
            removed += 1;
        }

        // Remove orphan auxiliary files
        for path in &orphan_auxiliary {
            std::fs::remove_file(path).with_context(|| format!("Failed to remove {:?}", path))?;
            println!("  {} {}", "Removed".red(), path.display());
            removed += 1;
        }

        // Remove empty directories (in reverse order so children come before parents)
        empty_dirs.sort();
        empty_dirs.reverse();
        for path in &empty_dirs {
            // Re-check if empty (removal of files above may have changed things)
            if let Ok(mut entries) = std::fs::read_dir(path) {
                if entries.next().is_none() {
                    std::fs::remove_dir(path)
                        .with_context(|| format!("Failed to remove directory {:?}", path))?;
                    println!("  {} {}/", "Removed".red(), path.display());
                    removed += 1;
                }
            }
        }

        println!();
        println!("{} {} item(s) removed", "Done!".green().bold(), removed);
    }

    Ok(())
}
