use anyhow::{bail, Context, Result};
use colored::Colorize;
use std::path::{Path, PathBuf};

use crate::config::Config;
use crate::organize::{scan_directory, tree, FixPlan, FormatTemplate};

/// Run the fix command - scan organized library and fix non-compliant paths
pub fn run(dest_override: Option<&PathBuf>, no_dry_run: bool, show_all: bool) -> Result<()> {
    // Load config
    let config = Config::load().context("Failed to load config")?;

    // Get format string
    let format_str = config
        .format(None)
        .context("No format specified. Set [organize] format in config")?;

    // Get destination (library to scan)
    let dest = config
        .dest(dest_override)
        .context("No destination specified. Set [organize] dest in config or use --dest")?;

    // Parse format template
    let template = FormatTemplate::parse(&format_str).context("Failed to parse format string")?;

    // Validate destination directory
    if !dest.exists() {
        bail!("Library directory does not exist: {:?}", dest);
    }
    if !dest.is_dir() {
        bail!("Library path is not a directory: {:?}", dest);
    }

    // Scan library
    println!("Scanning {:?}...", dest);
    let files = scan_directory(&dest).context("Failed to scan library")?;

    if files.is_empty() {
        println!("No .m4b files found in {:?}", dest);
        return Ok(());
    }

    println!("Found {} .m4b file(s)", files.len());
    println!();

    // Build fix plan
    let plan = FixPlan::build(&files, &template, &dest);

    // Check for conflicts
    if !plan.conflicts.is_empty() {
        print_conflicts(&plan.conflicts);
        bail!("Cannot proceed with destination conflicts.");
    }

    // Display results
    print_results(&plan, show_all);

    // Handle uncategorized (files with missing metadata)
    if !plan.uncategorized.is_empty() {
        println!();
        println!(
            "{}: {} file(s) have missing metadata and cannot be checked",
            "Warning".yellow().bold(),
            plan.uncategorized.len()
        );
        for file in &plan.uncategorized {
            println!(
                "  {} - missing: {}",
                file.source.display(),
                file.missing_fields.join(", ")
            );
        }
    }

    // Execute if --no-dry-run and there are files to fix
    if !plan.needs_fix.is_empty() {
        if no_dry_run {
            execute_fix(&plan)?;
        } else {
            println!();
            println!("{}", "Dry run - no files moved.".yellow());
            println!("Run with {} to move files.", "--no-dry-run".cyan());
        }
    }

    Ok(())
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

fn print_results(plan: &FixPlan, show_all: bool) {
    let needs_fix_count = plan.needs_fix.len();
    let compliant_count = plan.compliant.len();

    if needs_fix_count == 0 {
        println!(
            "{} All {} file(s) are compliant!",
            "✓".green(),
            compliant_count
        );
        return;
    }

    println!(
        "{} file(s) need fixing, {} file(s) are compliant",
        needs_fix_count.to_string().yellow(),
        compliant_count.to_string().green()
    );
    println!();

    // Print files that need fixing
    println!("Files needing adjustment:");
    print!("{}", tree::render_list(&plan.needs_fix));

    // Print compliant files if --show-all
    if show_all && !plan.compliant.is_empty() {
        println!();
        println!("Compliant files:");
        for path in &plan.compliant {
            println!("  {} {}", "✓".green(), path.display());
        }
    }
}

fn execute_fix(plan: &FixPlan) -> Result<()> {
    println!();
    println!("{}", "Moving files...".green());

    for op in &plan.needs_fix {
        // Create parent directories
        if let Some(parent) = op.dest.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory {:?}", parent))?;
        }

        // Move file (rename)
        std::fs::rename(&op.source, &op.dest)
            .with_context(|| format!("Failed to move {:?} to {:?}", op.source, op.dest))?;

        println!("  {} {}", "✓".green(), op.dest.display());

        // Try to remove empty parent directories
        cleanup_empty_dirs(&op.source);
    }

    println!();
    println!(
        "{} {} file(s) moved.",
        "Done!".green().bold(),
        plan.needs_fix.len()
    );

    Ok(())
}

/// Remove empty parent directories after moving a file
fn cleanup_empty_dirs(file_path: &Path) {
    let mut current = file_path.parent();

    while let Some(dir) = current {
        // Try to remove the directory (will fail if not empty)
        if std::fs::remove_dir(dir).is_err() {
            break;
        }
        current = dir.parent();
    }
}
