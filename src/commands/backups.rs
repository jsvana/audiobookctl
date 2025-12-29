//! Backups command - manage .bak files

use crate::config::Config;
use crate::safety::backup::{find_all_backups, format_size};
use anyhow::{Context, Result};
use std::fs;
use std::io::{self, Write};
use std::path::Path;

/// List all backup files
pub fn list(dir: &Path) -> Result<()> {
    let backups = find_all_backups(dir)?;

    if backups.is_empty() {
        println!("No backup files found in {}", dir.display());
        return Ok(());
    }

    let total_size: u64 = backups.iter().map(|b| b.size_bytes).sum();

    println!("Backup files in {}:", dir.display());
    println!();

    for backup in &backups {
        println!(
            "  {} ({})",
            backup.backup_path.display(),
            format_size(backup.size_bytes)
        );
    }

    println!();
    println!(
        "Total: {} files, {}",
        backups.len(),
        format_size(total_size)
    );

    // Show limit from config
    if let Ok(config) = Config::load() {
        let limit = config.backups.max_storage_bytes;
        let percent = (total_size as f64 / limit as f64) * 100.0;
        println!(
            "Limit: {} ({:.1}% used)",
            format_size(limit),
            percent
        );
    }

    Ok(())
}

/// Clean backup files interactively or all at once
pub fn clean(dir: &Path, all: bool, yes: bool) -> Result<()> {
    let backups = find_all_backups(dir)?;

    if backups.is_empty() {
        println!("No backup files to clean.");
        return Ok(());
    }

    let total_size: u64 = backups.iter().map(|b| b.size_bytes).sum();

    if all {
        if !yes {
            print!(
                "Delete {} backup files ({})? [y/N] ",
                backups.len(),
                format_size(total_size)
            );
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;

            if !input.trim().eq_ignore_ascii_case("y") {
                println!("Aborted.");
                return Ok(());
            }
        }

        for backup in &backups {
            fs::remove_file(&backup.backup_path)
                .with_context(|| format!("Failed to delete {}", backup.backup_path.display()))?;
            println!("Deleted: {}", backup.backup_path.display());
        }

        println!();
        println!("Cleaned {} files, freed {}", backups.len(), format_size(total_size));
    } else {
        // Interactive mode
        let mut deleted_count = 0;
        let mut deleted_size = 0u64;

        for backup in &backups {
            print!(
                "Delete {} ({})? [y/N/q] ",
                backup.backup_path.display(),
                format_size(backup.size_bytes)
            );
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let input = input.trim().to_lowercase();

            if input == "q" {
                break;
            }

            if input == "y" || input == "yes" {
                fs::remove_file(&backup.backup_path)?;
                deleted_count += 1;
                deleted_size += backup.size_bytes;
                println!("  Deleted.");
            } else {
                println!("  Skipped.");
            }
        }

        if deleted_count > 0 {
            println!();
            println!("Cleaned {} files, freed {}", deleted_count, format_size(deleted_size));
        }
    }

    Ok(())
}

/// Get current backup storage usage
pub fn current_usage(dir: &Path) -> Result<u64> {
    let backups = find_all_backups(dir)?;
    Ok(backups.iter().map(|b| b.size_bytes).sum())
}
