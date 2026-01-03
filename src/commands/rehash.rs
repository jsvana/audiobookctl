//! Rehash command - recalculate hash files for audiobooks

use anyhow::{bail, Context, Result};
use colored::Colorize;
use std::io::{self, Write};
use std::path::Path;
use walkdir::WalkDir;

use crate::hash::{hash_file_path, sha256_file, write_hash_file};

/// Run the rehash command
pub fn run(dir: &Path, force: bool, dry_run: bool) -> Result<()> {
    if !dir.exists() {
        bail!("Directory does not exist: {:?}", dir);
    }
    if !dir.is_dir() {
        bail!("Not a directory: {:?}", dir);
    }

    // Find all m4b files
    print!("Scanning {:?}... ", dir);
    io::stdout().flush().ok();

    let m4b_files: Vec<_> = WalkDir::new(dir)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path().is_file()
                && e.path()
                    .extension()
                    .map(|ext| ext.to_string_lossy().to_lowercase() == "m4b")
                    .unwrap_or(false)
        })
        .map(|e| e.path().to_path_buf())
        .collect();

    println!("found {} .m4b file(s)", m4b_files.len());

    if m4b_files.is_empty() {
        return Ok(());
    }

    // Count how many need rehashing
    let need_hash: Vec<_> = if force {
        m4b_files.clone()
    } else {
        m4b_files
            .iter()
            .filter(|p| !hash_file_path(p).exists())
            .cloned()
            .collect()
    };

    let skip_count = m4b_files.len() - need_hash.len();
    if skip_count > 0 && !force {
        println!(
            "Skipping {} file(s) with existing hash files (use {} to recalculate)",
            skip_count,
            "--force".cyan()
        );
    }

    if need_hash.is_empty() {
        println!("{} All files already have hash files", "✓".green());
        return Ok(());
    }

    if dry_run {
        println!();
        println!("Would hash {} file(s):", need_hash.len());
        for path in &need_hash {
            println!("  {}", path.display());
        }
        println!();
        println!("{}", "Dry run - no hash files written.".yellow());
        println!("Run with {} to write hash files.", "--no-dry-run".cyan());
        return Ok(());
    }

    println!();
    println!("{}", "Computing hashes...".green());

    let total = need_hash.len();
    for (i, path) in need_hash.iter().enumerate() {
        print!(
            "\r\x1b[K({}/{}) {}",
            i + 1,
            total,
            path.file_name().unwrap_or_default().to_string_lossy()
        );
        io::stdout().flush().ok();

        let hash = sha256_file(path).with_context(|| format!("Failed to hash {:?}", path))?;

        write_hash_file(path, &hash)
            .with_context(|| format!("Failed to write hash file for {:?}", path))?;
    }

    // Clear progress line
    print!("\r\x1b[K");
    io::stdout().flush().ok();

    println!(
        "{} {} hash file(s) written",
        "Done!".green().bold(),
        need_hash.len()
    );

    Ok(())
}
