//! Lookup-all command - batch metadata lookup with queue mode

use crate::commands::backups::current_usage;
use crate::commands::lookup::{merged_to_toml, process_lookup, query_and_merge};
use crate::config::Config;
use crate::editor::{compute_changes, toml_to_metadata};
use crate::lookup::{MergedMetadata, TrustedSource};
use crate::metadata::{write_metadata, AudiobookMetadata};
use crate::organize::scanner::scan_directory;
use crate::safety::backup::{create_backup, format_size};
use anyhow::Result;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

/// File with lookup results ready for processing
struct QueuedFile {
    path: std::path::PathBuf,
    original: AudiobookMetadata,
    merged: MergedMetadata,
    file_size: u64,
}

/// Run batch lookup on a directory
pub fn run(
    dir: &Path,
    auto_accept: bool,
    no_dry_run: bool,
    yes: bool,
    no_backup: bool,
    trust_source: Option<TrustedSource>,
) -> Result<()> {
    let config = Config::load().unwrap_or_default();

    // Step 1: Scan directory
    println!("Scanning {}...", dir.display());
    let files = scan_directory(dir)?;

    if files.is_empty() {
        println!("No .m4b files found.");
        return Ok(());
    }

    println!("Found {} audiobook files.", files.len());
    println!();

    // Step 2: Query APIs for each file
    let mut queued: Vec<QueuedFile> = Vec::new();
    let mut skipped = 0;
    let mut errors = 0;

    for (i, file) in files.iter().enumerate() {
        print!("[{}/{}] Checking {}... ", i + 1, files.len(), file.filename);
        io::stdout().flush()?;

        match query_and_merge(&file.path) {
            Ok((original, merged, sources)) => {
                // Check if trusted source has data
                if let Some(trusted) = trust_source {
                    if !crate::lookup::has_trusted_source_data(&merged, trusted) {
                        println!(
                            "skipped (trusted source '{}' has no data)",
                            trusted.as_str()
                        );
                        skipped += 1;
                        continue;
                    }
                }

                if let Some(matched_sources) = merged.matches_file() {
                    println!("matches [{}] - skipping", matched_sources.join(", "));
                    skipped += 1;
                } else {
                    println!("updates available from [{}]", sources.join(", "));
                    let file_size = fs::metadata(&file.path).map(|m| m.len()).unwrap_or(0);
                    queued.push(QueuedFile {
                        path: file.path.clone(),
                        original,
                        merged,
                        file_size,
                    });
                }
            }
            Err(e) => {
                println!("error: {}", e);
                errors += 1;
            }
        }
    }

    println!();

    if queued.is_empty() {
        println!("All {} files are up to date.", skipped);
        return Ok(());
    }

    // Step 3: Check backup limits
    let queued = check_backup_limits(dir, queued, &config, no_backup)?;

    if queued.is_empty() {
        return Ok(());
    }

    // Step 4: Print summary
    println!(
        "Found {} files with available updates ({} already up to date, {} errors)",
        queued.len(),
        skipped,
        errors
    );
    println!();

    // Step 5: Process queue
    for (i, item) in queued.iter().enumerate() {
        println!(
            "[{}/{}] Processing {}",
            i + 1,
            queued.len(),
            item.path.display()
        );

        if let Some(trusted) = trust_source {
            // Use trusted source mode
            let resolved = crate::lookup::resolve_with_trusted_source(&item.merged, trusted);
            process_trusted_accept(
                &item.path,
                &item.original,
                &resolved,
                no_dry_run,
                no_backup,
                trusted,
            )?;
        } else if auto_accept {
            process_auto_accept(
                &item.path,
                &item.original,
                &item.merged,
                no_dry_run,
                no_backup,
            )?;
        } else {
            process_lookup(
                &item.path,
                &item.original,
                &item.merged,
                no_dry_run,
                yes,
                no_backup,
            )?;
        }

        println!();
    }

    Ok(())
}

/// Check backup storage limits and truncate queue if necessary
fn check_backup_limits(
    dir: &Path,
    mut queued: Vec<QueuedFile>,
    config: &Config,
    no_backup: bool,
) -> Result<Vec<QueuedFile>> {
    if no_backup {
        return Ok(queued);
    }

    let current = current_usage(dir)?;
    let limit = config.backups.max_storage_bytes;
    let queued_size: u64 = queued.iter().map(|f| f.file_size).sum();

    if current + queued_size <= limit {
        return Ok(queued);
    }

    // Need to truncate
    let available = limit.saturating_sub(current);
    let mut allowed_size = 0u64;
    let mut allowed_count = 0;

    for item in &queued {
        if allowed_size + item.file_size <= available {
            allowed_size += item.file_size;
            allowed_count += 1;
        } else {
            break;
        }
    }

    println!(
        "Found {} files with updates, but backup limit ({}) allows only {}.",
        queued.len(),
        format_size(limit),
        allowed_count
    );
    println!("Current backup usage: {}", format_size(current));
    println!("Run `audiobookctl backups clean` or increase limit in config.");

    if allowed_count == 0 {
        println!();
        println!("Cannot process any files - backup limit reached.");
        return Ok(Vec::new());
    }

    println!("Processing first {} files...", allowed_count);
    println!();

    queued.truncate(allowed_count);
    Ok(queued)
}

/// Auto-accept changes when all sources agree
fn process_auto_accept(
    file: &Path,
    original: &AudiobookMetadata,
    merged: &MergedMetadata,
    no_dry_run: bool,
    no_backup: bool,
) -> Result<()> {
    // Check if there are any actual conflicts
    let has_conflicts = has_real_conflicts(merged);

    if has_conflicts {
        // Fall back to interactive mode for this file
        println!("  Has conflicts - opening editor...");
        process_lookup(file, original, merged, no_dry_run, false, no_backup)?;
    } else {
        // Auto-apply all agreed values that differ from file
        let toml = merged_to_toml(merged);
        let new_metadata = toml_to_metadata(&toml)?;
        let changes = compute_changes(original, &new_metadata);

        if changes.is_empty() {
            println!("  No changes to apply.");
            return Ok(());
        }

        // Show what will be auto-applied
        let fields: Vec<&str> = changes.iter().map(|c| c.field.as_str()).collect();
        println!("  Auto-applying: {}", fields.join(", "));

        if no_dry_run {
            if !no_backup {
                create_backup(file)?;
            }
            write_metadata(file, &new_metadata)?;
            println!("  Applied.");
        } else {
            println!("  (dry-run, use --no-dry-run to apply)");
        }
    }

    Ok(())
}

/// Auto-accept using trusted source values
fn process_trusted_accept(
    file: &Path,
    original: &AudiobookMetadata,
    resolved: &MergedMetadata,
    no_dry_run: bool,
    no_backup: bool,
    trusted: TrustedSource,
) -> Result<()> {
    let toml = merged_to_toml(resolved);
    let new_metadata = toml_to_metadata(&toml)?;
    let changes = compute_changes(original, &new_metadata);

    if changes.is_empty() {
        println!("  No changes from '{}'.", trusted.as_str());
        return Ok(());
    }

    let fields: Vec<&str> = changes.iter().map(|c| c.field.as_str()).collect();
    println!("  Trusted '{}': {}", trusted.as_str(), fields.join(", "));

    if no_dry_run {
        if !no_backup {
            create_backup(file)?;
        }
        write_metadata(file, &new_metadata)?;
        println!("  Applied.");
    } else {
        println!("  (dry-run, use --no-dry-run to apply)");
    }

    Ok(())
}

/// Check if merged metadata has any real conflicts (not just empty fields)
fn has_real_conflicts(merged: &MergedMetadata) -> bool {
    use crate::lookup::FieldValue;

    let fields = [
        &merged.title,
        &merged.author,
        &merged.narrator,
        &merged.series,
        &merged.series_position,
        &merged.year,
        &merged.description,
        &merged.publisher,
        &merged.genre,
        &merged.isbn,
        &merged.asin,
    ];

    fields
        .iter()
        .any(|f| matches!(f, FieldValue::Conflicting { .. }))
}
