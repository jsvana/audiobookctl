use crate::editor::{compute_changes, format_diff, metadata_to_toml, toml_to_metadata};
use crate::metadata::{read_metadata, write_metadata, AudiobookMetadata};
use crate::safety::{
    backup_path_for, create_backup, delete_backup, find_all_backups, format_size, has_backup,
    PendingEditsCache,
};
use anyhow::{bail, Context, Result};
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;

pub fn run(
    file: Option<&Path>,
    no_dry_run: bool,
    yes: bool,
    no_backup: bool,
    commit: bool,
    commit_all: bool,
) -> Result<()> {
    // Handle --commit-all (no file needed)
    if commit_all {
        return handle_commit_all();
    }

    // All other operations require a file
    let file =
        file.ok_or_else(|| anyhow::anyhow!("No file specified. Use: audiobookctl edit <file>"))?;

    // Handle --commit for specific file
    if commit {
        return handle_commit(file);
    }

    // Main edit flow
    let cache = PendingEditsCache::new()?;

    // Read current metadata
    let original_metadata = read_metadata(file)?;

    // Check for pending edit
    let (edited_toml, from_cache) = if no_dry_run && cache.has_pending(file)? {
        // Load from cache
        let pending = cache.load(file)?.unwrap();
        println!("Loading pending edit from cache...");
        (pending.toml_content, true)
    } else {
        // Open in editor
        let toml = metadata_to_toml(&original_metadata);
        let edited = open_in_editor(&toml)?;
        (edited, false)
    };

    // Parse edited TOML
    let new_metadata = toml_to_metadata(&edited_toml).context("Failed to parse edited TOML")?;

    // Compute and display diff
    let changes = compute_changes(&original_metadata, &new_metadata);
    let diff_output = format_diff(&file.display().to_string(), &changes);
    println!("{}", diff_output);

    if changes.is_empty() {
        if from_cache {
            cache.clear(file)?;
        }
        return Ok(());
    }

    if no_dry_run {
        // Apply changes
        apply_changes(file, &new_metadata, &cache, yes, no_backup)?;
    } else {
        // Save to pending cache
        let _cache_path = cache.save(file, &edited_toml)?;
        println!();
        println!("Changes saved to pending cache.");
        println!(
            "To apply: audiobookctl edit \"{}\" --no-dry-run",
            file.display()
        );
    }

    Ok(())
}

fn open_in_editor(content: &str) -> Result<String> {
    let editor = std::env::var("EDITOR")
        .or_else(|_| std::env::var("VISUAL"))
        .unwrap_or_else(|_| "vi".to_string());

    // Create temp file
    let temp_dir = std::env::temp_dir();
    let temp_path = temp_dir.join("audiobookctl_edit.toml");

    std::fs::write(&temp_path, content).context("Failed to create temp file for editing")?;

    // Open editor
    let status = Command::new(&editor)
        .arg(&temp_path)
        .status()
        .with_context(|| format!("Failed to open editor: {}", editor))?;

    if !status.success() {
        bail!("Editor exited with error");
    }

    // Read back
    let edited = std::fs::read_to_string(&temp_path).context("Failed to read edited file")?;

    // Clean up
    let _ = std::fs::remove_file(&temp_path);

    Ok(edited)
}

fn apply_changes(
    file: &Path,
    new_metadata: &AudiobookMetadata,
    cache: &PendingEditsCache,
    yes: bool,
    no_backup: bool,
) -> Result<()> {
    // Confirm
    if !yes {
        print!("Apply these changes to {}? [y/N] ", file.display());
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        if !input.trim().eq_ignore_ascii_case("y") && !input.trim().eq_ignore_ascii_case("yes") {
            println!("Aborted.");
            return Ok(());
        }
    }

    // Create backup
    if !no_backup {
        let backup_path = create_backup(file)?;
        println!("Created backup: {}", backup_path.display());
    } else {
        println!("Warning: No backup created. Changes cannot be undone.");
    }

    // Write changes
    write_metadata(file, new_metadata)?;
    println!("Changes applied successfully.");

    // Clear pending cache
    cache.clear(file)?;

    Ok(())
}

fn handle_commit(file: &Path) -> Result<()> {
    if !has_backup(file) {
        bail!("No backup found for: {}", file.display());
    }

    let backup = backup_path_for(file);
    let size = std::fs::metadata(&backup).map(|m| m.len()).unwrap_or(0);

    print!(
        "Delete backup {} ({})? [y/N] ",
        backup.display(),
        format_size(size)
    );
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    if input.trim().eq_ignore_ascii_case("y") || input.trim().eq_ignore_ascii_case("yes") {
        delete_backup(file)?;
        println!("Backup deleted. Change committed.");
    } else {
        println!("Aborted.");
    }

    Ok(())
}

fn handle_commit_all() -> Result<()> {
    let current_dir = std::env::current_dir()?;
    let backups = find_all_backups(&current_dir)?;

    if backups.is_empty() {
        println!("No backup files found.");
        return Ok(());
    }

    let total_size: u64 = backups.iter().map(|b| b.size_bytes).sum();

    println!(
        "Found {} backup files ({} total):",
        backups.len(),
        format_size(total_size)
    );
    for backup in &backups {
        println!(
            "  {} ({})",
            backup.backup_path.display(),
            format_size(backup.size_bytes)
        );
    }
    println!();

    print!("Delete all backups? [y/N] ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    if input.trim().eq_ignore_ascii_case("y") || input.trim().eq_ignore_ascii_case("yes") {
        for backup in &backups {
            std::fs::remove_file(&backup.backup_path)?;
        }
        println!("Deleted {} backup(s).", backups.len());
    } else {
        println!("Aborted.");
    }

    Ok(())
}
