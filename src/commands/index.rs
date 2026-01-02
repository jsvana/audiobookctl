//! Index command - build or update library database

use anyhow::{Context, Result};
use colored::Colorize;
use std::path::Path;
use walkdir::WalkDir;

use crate::database::LibraryDb;
use crate::hash::sha256_file;
use crate::metadata::read_metadata;

/// Run the index command
pub fn run(dir: &Path, full: bool, prune: bool) -> Result<()> {
    if !dir.exists() {
        anyhow::bail!("Directory does not exist: {:?}", dir);
    }
    if !dir.is_dir() {
        anyhow::bail!("Not a directory: {:?}", dir);
    }

    println!("Opening database in {:?}...", dir);
    let db = LibraryDb::open(dir)?;

    if prune {
        println!("Pruning missing files...");
        let removed = db.prune()?;
        println!("{} {} record(s) removed", "Done!".green().bold(), removed);
        return Ok(());
    }

    println!("Scanning for .m4b files...");
    let mut indexed = 0;
    let mut skipped = 0;
    let mut errors = 0;

    for entry in WalkDir::new(dir)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();

        if !path.is_file() {
            continue;
        }

        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext.to_lowercase() != "m4b" {
            continue;
        }

        let relative = path.strip_prefix(dir).unwrap_or(path);
        let relative_str = relative.to_string_lossy();

        // Check if already indexed (unless --full)
        if !full {
            if let Ok(Some(existing)) = db.get_by_path(&relative_str) {
                // Quick check: same size?
                if let Ok(meta) = std::fs::metadata(path) {
                    if meta.len() as i64 == existing.file_size {
                        skipped += 1;
                        continue;
                    }
                }
            }
        }

        // Index file
        print!("  Indexing {}... ", relative_str);

        match index_file(&db, dir, path) {
            Ok(()) => {
                println!("{}", "OK".green());
                indexed += 1;
            }
            Err(e) => {
                println!("{}: {}", "ERROR".red(), e);
                errors += 1;
            }
        }
    }

    println!();
    println!(
        "{} {} indexed, {} skipped, {} errors",
        "Done!".green().bold(),
        indexed,
        skipped,
        errors
    );
    println!("Database: {:?}", dir.join(".audiobookctl.db"));

    Ok(())
}

fn index_file(db: &LibraryDb, base: &Path, path: &Path) -> Result<()> {
    let metadata = read_metadata(path).context("Failed to read metadata")?;
    let file_size = std::fs::metadata(path)?.len() as i64;
    let hash = sha256_file(path)?;
    let relative = path.strip_prefix(base).unwrap_or(path);

    db.upsert(&relative.to_string_lossy(), file_size, &hash, &metadata)?;

    Ok(())
}
