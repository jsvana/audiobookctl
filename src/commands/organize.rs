use anyhow::{bail, Context, Result};
use colored::Colorize;
use std::collections::HashMap;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crate::config::Config;
use crate::database::LibraryDb;
use crate::hash::sha256_file;
use crate::metadata::AudiobookMetadata;
use crate::organize::{
    scan_directory_with_progress, tree, AlreadyPresent, FormatTemplate, OrganizePlan, PlanProgress,
    PlannedOperation, UncategorizedFile,
};

/// Run the organize command
pub fn run(
    source: &Path,
    dest_override: Option<&PathBuf>,
    format_override: Option<&str>,
    no_dry_run: bool,
    allow_uncategorized: bool,
    list_mode: bool,
) -> Result<()> {
    // Load config
    let config = Config::load().context("Failed to load config")?;

    // Get format string
    let format_str = config
        .format(format_override)
        .context("No format specified. Set [organize] format in config or use --format")?;

    // Get destination
    let dest = config
        .dest(dest_override)
        .context("No destination specified. Set [organize] dest in config or use --dest")?;

    // Parse format template
    let template = FormatTemplate::parse(&format_str).context("Failed to parse format string")?;

    // Validate source directory
    if !source.exists() {
        bail!("Source directory does not exist: {:?}", source);
    }
    if !source.is_dir() {
        bail!("Source is not a directory: {:?}", source);
    }

    // Scan source directory with progress output
    print!("Scanning {:?}... ", source);
    io::stdout().flush().ok();
    let mut scan_count = 0;
    let files = scan_directory_with_progress(source, |path| {
        scan_count += 1;
        print!(
            "\r\x1b[KScanning {:?}... {} ({})",
            source,
            scan_count,
            path.file_name().unwrap_or_default().to_string_lossy()
        );
        io::stdout().flush().ok();
    })
    .context("Failed to scan source directory")?;
    // Clear the progress line and show final count
    print!("\r\x1b[K"); // Clear line
    io::stdout().flush().ok();

    if files.is_empty() {
        println!("No .m4b files found in {:?}", source);
        return Ok(());
    }

    println!("Found {} .m4b file(s)", files.len());

    // Build metadata map for database writes
    let file_metadata: HashMap<PathBuf, AudiobookMetadata> = files
        .iter()
        .map(|f| (f.path.clone(), f.metadata.clone()))
        .collect();

    // Build plan with progress output for hash comparisons
    print!("Planning...");
    io::stdout().flush().ok();
    let plan = OrganizePlan::build_with_progress(&files, &template, &dest, |progress| {
        match progress {
            PlanProgress::HashingSource(path) => {
                print!(
                    "\r\x1b[KComparing: {} (source)",
                    path.file_name().unwrap_or_default().to_string_lossy()
                );
            }
            PlanProgress::HashingDest(path) => {
                print!(
                    "\r\x1b[KComparing: {} (dest)",
                    path.file_name().unwrap_or_default().to_string_lossy()
                );
            }
        }
        io::stdout().flush().ok();
    });
    // Clear the progress line
    print!("\r\x1b[K");
    io::stdout().flush().ok();

    // Check for missing metadata (without --allow-uncategorized)
    if !plan.uncategorized.is_empty() && !allow_uncategorized {
        print_missing_metadata_error(&plan.uncategorized);
        bail!("Cannot proceed with missing metadata. Use --allow-uncategorized to continue.");
    }

    // Check for conflicts
    if !plan.conflicts.is_empty() {
        print_conflicts(&plan.conflicts);
        bail!("Cannot proceed with destination conflicts.");
    }

    // Display plan
    if list_mode {
        print_list_view(&plan.operations, &plan.uncategorized, allow_uncategorized);
    } else {
        print_tree_view(
            &plan.operations,
            &plan.uncategorized,
            &dest,
            allow_uncategorized,
        );
    }

    // Show already-present files
    print_already_present(&plan.already_present);

    // Execute if --no-dry-run
    if no_dry_run {
        execute_plan(
            &plan.operations,
            &plan.already_present,
            &plan.uncategorized,
            &dest,
            allow_uncategorized,
            &file_metadata,
        )?;
    } else {
        println!();
        println!("{}", "Dry run - no files copied.".yellow());
        println!("Run with {} to copy files.", "--no-dry-run".cyan());
    }

    Ok(())
}

fn print_missing_metadata_error(uncategorized: &[UncategorizedFile]) {
    eprintln!(
        "{}: {} file(s) are missing required metadata",
        "Error".red().bold(),
        uncategorized.len()
    );
    eprintln!();

    for file in uncategorized {
        eprintln!("  {}", file.source.display());
        eprintln!(
            "    {}: {}",
            "missing".red(),
            file.missing_fields.join(", ")
        );
        eprintln!();
    }

    eprintln!(
        "Use '{}' or '{}' to add metadata.",
        "audiobookctl edit <file>".cyan(),
        "audiobookctl lookup <file>".cyan()
    );
    eprintln!(
        "Or run with {} to place these in __uncategorized__/",
        "--allow-uncategorized".cyan()
    );
}

fn print_conflicts(conflicts: &[crate::organize::Conflict]) {
    eprintln!(
        "{}: {} destination conflict(s) detected",
        "Error".red().bold(),
        conflicts.len()
    );
    eprintln!();

    for conflict in conflicts {
        eprintln!("  {} → {}", "Conflict".red(), conflict.dest.display());

        for source in &conflict.sources {
            eprintln!("    from: {}", source.display());
        }

        if conflict.exists_on_disk {
            eprintln!("    {} (file already exists)", "warning".yellow());
        }

        eprintln!();
    }

    eprintln!("Resolve by renaming files or adjusting metadata.");
}

fn print_already_present(already_present: &[AlreadyPresent]) {
    if already_present.is_empty() {
        return;
    }

    println!();
    println!(
        "{}: {} file(s) already present at destination (hash match)",
        "Info".cyan().bold(),
        already_present.len()
    );

    for file in already_present {
        println!(
            "  {} {} → {}",
            "≡".cyan(),
            file.source
                .file_name()
                .unwrap_or_default()
                .to_string_lossy(),
            file.dest.display()
        );
    }
}

fn print_tree_view(
    operations: &[PlannedOperation],
    uncategorized: &[UncategorizedFile],
    dest: &Path,
    allow_uncategorized: bool,
) {
    println!(
        "Organizing {} file(s) to {:?}",
        operations.len()
            + if allow_uncategorized {
                uncategorized.len()
            } else {
                0
            },
        dest
    );
    println!();

    if !operations.is_empty() {
        print!("{}", tree::render_tree(operations, dest));
    }

    if allow_uncategorized && !uncategorized.is_empty() {
        println!();
        let uncategorized_with_reasons: Vec<_> = uncategorized
            .iter()
            .map(|u| (u.source.clone(), u.missing_fields.clone()))
            .collect();
        print!(
            "{}",
            tree::render_uncategorized(&uncategorized_with_reasons)
        );

        // Print reasons
        println!();
        println!("Uncategorized file reasons:");
        for file in uncategorized {
            println!(
                "  {} - missing: {}",
                file.source
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default(),
                file.missing_fields.join(", ")
            );
        }
    }
}

fn print_list_view(
    operations: &[PlannedOperation],
    uncategorized: &[UncategorizedFile],
    allow_uncategorized: bool,
) {
    println!(
        "Organizing {} file(s)",
        operations.len()
            + if allow_uncategorized {
                uncategorized.len()
            } else {
                0
            }
    );
    println!();

    print!("{}", tree::render_list(operations));

    if allow_uncategorized && !uncategorized.is_empty() {
        println!();
        for file in uncategorized {
            println!(
                "{} → __uncategorized__/{} (missing: {})",
                file.source.display(),
                file.source
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default(),
                file.missing_fields.join(", ")
            );
        }
    }
}

fn execute_plan(
    operations: &[PlannedOperation],
    already_present: &[AlreadyPresent],
    uncategorized: &[UncategorizedFile],
    dest: &Path,
    allow_uncategorized: bool,
    file_metadata: &HashMap<PathBuf, AudiobookMetadata>,
) -> Result<()> {
    println!();
    println!("{}", "Copying files...".green());

    let mut aux_count = 0;

    // Copy organized files
    for op in operations {
        // Create parent directories
        if let Some(parent) = op.dest.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory {:?}", parent))?;
        }

        // Compute source hash before copy
        let source_hash = sha256_file(&op.source)
            .with_context(|| format!("Failed to hash source {:?}", op.source))?;

        // Copy m4b file
        std::fs::copy(&op.source, &op.dest)
            .with_context(|| format!("Failed to copy {:?} to {:?}", op.source, op.dest))?;

        // Verify destination hash matches source
        let dest_hash = sha256_file(&op.dest)
            .with_context(|| format!("Failed to hash destination {:?}", op.dest))?;

        if source_hash != dest_hash {
            bail!(
                "Copy verification failed: {:?} -> {:?}\n  Source hash: {}\n  Dest hash:   {}",
                op.source,
                op.dest,
                source_hash,
                dest_hash
            );
        }

        println!("  {} {}", "✓".green(), op.dest.display());

        // Copy auxiliary files
        for aux in &op.auxiliary {
            // Create parent directories for auxiliary file
            if let Some(parent) = aux.dest.parent() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("Failed to create directory {:?}", parent))?;
            }

            // Skip if destination already exists
            if aux.dest.exists() {
                println!(
                    "    {} {} (skipped, exists)",
                    "○".yellow(),
                    aux.dest.file_name().unwrap_or_default().to_string_lossy()
                );
                continue;
            }

            std::fs::copy(&aux.source, &aux.dest)
                .with_context(|| format!("Failed to copy {:?} to {:?}", aux.source, aux.dest))?;

            println!(
                "    {} {}",
                "+".cyan(),
                aux.dest.file_name().unwrap_or_default().to_string_lossy()
            );
            aux_count += 1;
        }
    }

    // Copy uncategorized files
    if allow_uncategorized && !uncategorized.is_empty() {
        let uncategorized_dir = dest.join("__uncategorized__");
        std::fs::create_dir_all(&uncategorized_dir)
            .with_context(|| format!("Failed to create {:?}", uncategorized_dir))?;

        for file in uncategorized {
            let filename = file.source.file_name().context("File has no filename")?;
            let dest_path = uncategorized_dir.join(filename);

            std::fs::copy(&file.source, &dest_path)
                .with_context(|| format!("Failed to copy {:?} to {:?}", file.source, dest_path))?;

            println!("  {} {} (uncategorized)", "✓".yellow(), dest_path.display());
        }
    }

    // Post-copy verification: check each destination directory has only the expected m4b
    println!();
    println!("{}", "Verifying copies...".cyan());
    for op in operations {
        if let Some(parent) = op.dest.parent() {
            let expected_filename = op
                .dest
                .file_name()
                .map(|f| f.to_string_lossy().to_string())
                .unwrap_or_default();

            // Count m4b files in the destination directory
            let m4b_files: Vec<_> = std::fs::read_dir(parent)
                .with_context(|| format!("Failed to read directory {:?}", parent))?
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.path()
                        .extension()
                        .map(|ext| ext.to_string_lossy().to_lowercase() == "m4b")
                        .unwrap_or(false)
                })
                .collect();

            if m4b_files.len() > 1 {
                let filenames: Vec<_> = m4b_files
                    .iter()
                    .map(|e| e.file_name().to_string_lossy().to_string())
                    .collect();
                eprintln!(
                    "  {} Directory {:?} has {} m4b files (expected 1): {:?}",
                    "⚠".yellow(),
                    parent,
                    m4b_files.len(),
                    filenames
                );
            } else if m4b_files.len() == 1 {
                let actual_filename = m4b_files[0].file_name().to_string_lossy().to_string();
                if actual_filename != expected_filename {
                    bail!(
                        "Verification failed: expected {:?} in {:?}, found {:?}",
                        expected_filename,
                        parent,
                        actual_filename
                    );
                }
            }
        }
    }
    println!("  Verification complete");

    println!();
    let total_m4b = operations.len()
        + if allow_uncategorized {
            uncategorized.len()
        } else {
            0
        };
    if aux_count > 0 {
        println!(
            "{} {} audiobook(s) + {} auxiliary file(s) copied.",
            "Done!".green().bold(),
            total_m4b,
            aux_count
        );
    } else {
        println!("{} {} file(s) copied.", "Done!".green().bold(), total_m4b);
    }

    // Update database
    println!();
    println!("{}", "Updating database...".cyan());

    let mut db = LibraryDb::open(dest)?;
    let mut db_count = 0;

    // Use transaction for batch updates
    db.begin_transaction()?;

    for op in operations {
        let metadata = file_metadata
            .get(&op.source)
            .with_context(|| format!("Missing metadata for {:?}", op.source))?;
        let relative = op.dest.strip_prefix(dest).unwrap_or(&op.dest);
        let file_size = std::fs::metadata(&op.dest)?.len() as i64;
        let hash = sha256_file(&op.dest)?;
        db.upsert(&relative.to_string_lossy(), file_size, &hash, metadata)?;
        db_count += 1;
    }

    // Touch already-present files to update their indexed_at timestamp
    for ap in already_present {
        let relative = ap.dest.strip_prefix(dest).unwrap_or(&ap.dest);
        db.touch(&relative.to_string_lossy())?;
        db_count += 1;
    }

    db.commit()?;
    println!("  {} record(s) updated in database", db_count);

    Ok(())
}
