//! Pending command - manage pending edits

use crate::editor::{compute_changes, format_diff, toml_to_metadata};
use crate::metadata::read_metadata;
use crate::safety::{create_backup, PendingEditsCache};
use crate::metadata::write_metadata;
use anyhow::{bail, Context, Result};
use std::io::{self, Write};
use std::path::Path;

/// List all pending edits
pub fn list(show_diff: bool) -> Result<()> {
    let cache = PendingEditsCache::new()?;
    let edits = cache.list_all()?;

    if edits.is_empty() {
        println!("No pending edits.");
        return Ok(());
    }

    println!("Pending edits:");

    for edit in &edits {
        let timestamp = edit.created_at.format("%Y-%m-%d %H:%M");
        println!(
            "  {} (edit saved {})",
            edit.original_path.display(),
            timestamp
        );

        if show_diff {
            // Show diff if file still exists
            if edit.original_path.exists() {
                match show_diff_for_edit(&edit.original_path, &edit.toml_content) {
                    Ok(diff) => {
                        for line in diff.lines() {
                            println!("    {}", line);
                        }
                    }
                    Err(e) => {
                        println!("    (error reading file: {})", e);
                    }
                }
            } else {
                println!("    (file no longer exists)");
            }
            println!();
        }
    }

    println!();
    println!("{} pending edit(s)", edits.len());

    Ok(())
}

/// Show diff for a specific pending edit
pub fn show(file: &Path) -> Result<()> {
    let cache = PendingEditsCache::new()?;

    let pending = cache.load(file)?;
    match pending {
        Some(edit) => {
            let diff = show_diff_for_edit(&edit.original_path, &edit.toml_content)?;
            println!("{}", diff);
        }
        None => {
            bail!("No pending edit found for: {}", file.display());
        }
    }

    Ok(())
}

/// Apply pending edits
pub fn apply(file: Option<&Path>, yes: bool, no_backup: bool) -> Result<()> {
    let cache = PendingEditsCache::new()?;

    if let Some(file) = file {
        // Apply single file
        apply_single(&cache, file, yes, no_backup)?;
    } else {
        // Apply all
        apply_all(&cache, yes, no_backup)?;
    }

    Ok(())
}

/// Clear pending edits
pub fn clear(file: Option<&Path>) -> Result<()> {
    let cache = PendingEditsCache::new()?;

    if let Some(file) = file {
        if cache.clear(file)? {
            println!("Cleared pending edit for: {}", file.display());
        } else {
            println!("No pending edit found for: {}", file.display());
        }
    } else {
        let count = cache.clear_all()?;
        println!("Cleared {} pending edit(s).", count);
    }

    Ok(())
}

fn show_diff_for_edit(file: &Path, toml_content: &str) -> Result<String> {
    let original_metadata = read_metadata(file)?;
    let new_metadata = toml_to_metadata(toml_content)?;
    let changes = compute_changes(&original_metadata, &new_metadata);
    Ok(format_diff(&file.display().to_string(), &changes))
}

fn apply_single(cache: &PendingEditsCache, file: &Path, yes: bool, no_backup: bool) -> Result<()> {
    let pending = cache.load(file)?;
    let edit = match pending {
        Some(e) => e,
        None => bail!("No pending edit found for: {}", file.display()),
    };

    // Show diff
    let diff = show_diff_for_edit(file, &edit.toml_content)?;
    println!("{}", diff);

    // Confirm
    if !yes {
        print!("Apply changes to {}? [y/N] ", file.display());
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        if !input.trim().eq_ignore_ascii_case("y") && !input.trim().eq_ignore_ascii_case("yes") {
            println!("Aborted.");
            return Ok(());
        }
    }

    // Apply
    let new_metadata = toml_to_metadata(&edit.toml_content)?;

    if !no_backup {
        let backup_path = create_backup(file)?;
        println!("Created backup: {}", backup_path.display());
    }

    write_metadata(file, &new_metadata)?;
    cache.clear(file)?;
    println!("Applied changes to: {}", file.display());

    Ok(())
}

fn apply_all(cache: &PendingEditsCache, yes: bool, no_backup: bool) -> Result<()> {
    let edits = cache.list_all()?;

    if edits.is_empty() {
        println!("No pending edits to apply.");
        return Ok(());
    }

    // Show summary
    println!("Pending edits to apply:");
    for edit in &edits {
        let status = if edit.original_path.exists() {
            ""
        } else {
            " (file missing)"
        };
        println!("  {}{}", edit.original_path.display(), status);
    }
    println!();

    // Confirm
    if !yes {
        print!("Apply {} pending edit(s)? [y/N] ", edits.len());
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        if !input.trim().eq_ignore_ascii_case("y") && !input.trim().eq_ignore_ascii_case("yes") {
            println!("Aborted.");
            return Ok(());
        }
    }

    println!();
    println!("Applying {} pending edit(s)...", edits.len());

    let mut applied = 0;
    let mut failed = 0;

    for edit in &edits {
        let result = apply_edit(cache, &edit.original_path, &edit.toml_content, no_backup);
        match result {
            Ok(()) => {
                println!("  \u{2713} {}", edit.original_path.display());
                applied += 1;
            }
            Err(e) => {
                println!("  \u{2717} {} ({})", edit.original_path.display(), e);
                failed += 1;
            }
        }
    }

    println!();
    println!("Applied: {}, Failed: {}", applied, failed);

    Ok(())
}

fn apply_edit(
    cache: &PendingEditsCache,
    file: &Path,
    toml_content: &str,
    no_backup: bool,
) -> Result<()> {
    if !file.exists() {
        bail!("file not found");
    }

    let new_metadata = toml_to_metadata(toml_content).context("invalid TOML")?;

    if !no_backup {
        create_backup(file)?;
    }

    write_metadata(file, &new_metadata)?;
    cache.clear(file)?;

    Ok(())
}
