//! Lookup command - query APIs for audiobook metadata

use crate::editor::{compute_changes, format_diff, toml_to_metadata};
use crate::lookup::{
    fetch_audible, fetch_openlibrary, merge_results, FieldValue, LookupResult, MergedMetadata,
};
use crate::metadata::{read_metadata, write_metadata, AudiobookMetadata};
use crate::safety::{create_backup, PendingEditsCache};
use anyhow::{bail, Context, Result};
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;

/// Query APIs and merge with existing metadata
pub fn query_and_merge(file: &Path) -> Result<(AudiobookMetadata, MergedMetadata, Vec<String>)> {
    let original_metadata = read_metadata(file)?;
    let results = query_apis_sync(&original_metadata)?;

    if results.is_empty() {
        anyhow::bail!("No results found from any API");
    }

    let sources: Vec<String> = results.iter().map(|r| r.source.clone()).collect();
    let merged = merge_results(&original_metadata, &results);

    Ok((original_metadata, merged, sources))
}

/// Process a single file lookup (shared by lookup and lookup-all)
pub fn process_lookup(
    file: &Path,
    original: &AudiobookMetadata,
    merged: &MergedMetadata,
    no_dry_run: bool,
    yes: bool,
    no_backup: bool,
) -> Result<bool> {
    // Generate TOML
    let toml_content = merged_to_toml(merged);

    // Open in editor
    println!("Opening editor...");
    let edited_toml = open_in_editor(&toml_content)?;

    // Parse edited TOML
    let new_metadata = toml_to_metadata(&edited_toml).context("Failed to parse edited TOML")?;

    // Compute diff
    let changes = compute_changes(original, &new_metadata);

    // Display diff
    let diff_output = format_diff(&file.display().to_string(), &changes);
    println!("{}", diff_output);

    if changes.is_empty() {
        println!("No changes to apply.");
        return Ok(false);
    }

    // Apply changes
    if no_dry_run {
        apply_changes(file, &new_metadata, yes, no_backup)?;
        Ok(true)
    } else {
        let cache = PendingEditsCache::new()?;
        let _cache_path = cache.save(file, &edited_toml)?;
        println!();
        println!("Changes saved to pending cache.");
        println!(
            "To apply: audiobookctl edit \"{}\" --no-dry-run",
            file.display()
        );
        Ok(false)
    }
}

/// Main entry point for the lookup command
pub fn run(file: &Path, no_dry_run: bool, yes: bool, no_backup: bool) -> Result<()> {
    println!("Reading metadata from {}...", file.display());

    let (original, merged, _sources) = query_and_merge(file)?;

    // Check for early exit
    if let Some(sources) = merged.matches_file() {
        println!(
            "{}: metadata matches [{}] - skipping",
            file.display(),
            sources.join(", ")
        );
        return Ok(());
    }

    process_lookup(file, &original, &merged, no_dry_run, yes, no_backup)?;

    Ok(())
}

/// Synchronous wrapper for async API queries using tokio runtime
fn query_apis_sync(metadata: &AudiobookMetadata) -> Result<Vec<LookupResult>> {
    let rt = tokio::runtime::Runtime::new().context("Failed to create tokio runtime")?;
    rt.block_on(query_apis(metadata))
}

/// Query APIs concurrently
async fn query_apis(metadata: &AudiobookMetadata) -> Result<Vec<LookupResult>> {
    let client = reqwest::Client::new();

    // Extract search parameters from existing metadata
    let title = metadata.title.as_deref();
    let author = metadata.author.as_deref();
    let isbn = metadata.isbn.as_deref();

    // Query both APIs concurrently
    print!("Querying Audible... ");
    io::stdout().flush()?;

    let audible_future = fetch_audible(&client, title, author);

    print!("Querying Open Library... ");
    io::stdout().flush()?;

    let openlibrary_future = fetch_openlibrary(&client, title, author, isbn);

    // Run both concurrently
    let (audible_result, openlibrary_result) = tokio::join!(audible_future, openlibrary_future);

    println!(); // Newline after status messages

    let mut results = Vec::new();

    // Handle Audible result
    match audible_result {
        Ok(Some(result)) => {
            println!(
                "  Audible: found \"{}\"",
                result.title.as_deref().unwrap_or("Unknown")
            );
            results.push(result);
        }
        Ok(None) => {
            println!("  Audible: no results");
        }
        Err(e) => {
            eprintln!("  Audible: error - {}", e);
        }
    }

    // Handle Open Library result
    match openlibrary_result {
        Ok(Some(result)) => {
            println!(
                "  Open Library: found \"{}\"",
                result.title.as_deref().unwrap_or("Unknown")
            );
            results.push(result);
        }
        Ok(None) => {
            println!("  Open Library: no results");
        }
        Err(e) => {
            eprintln!("  Open Library: error - {}", e);
        }
    }

    Ok(results)
}

/// Generate TOML from merged metadata with conflict annotations
pub fn merged_to_toml(merged: &MergedMetadata) -> String {
    let mut lines = Vec::new();

    lines.push("# Audiobook Metadata - Lookup Results".to_string());
    lines.push("# Edit values below. For conflicts, uncomment your preferred value.".to_string());
    lines.push(String::new());

    // Helper to add a field based on its FieldValue
    fn add_field(lines: &mut Vec<String>, name: &str, value: &FieldValue) {
        match value {
            FieldValue::Agreed { value: v, sources } => {
                let source_list = sources.join(", ");
                lines.push(format!(
                    "{} = \"{}\"  # [{}]",
                    name,
                    escape_toml_string(v),
                    source_list
                ));
            }
            FieldValue::Conflicting {
                selected,
                alternatives,
            } => {
                lines.push(format!("# {}: Sources disagree - pick one:", name));
                // Find which group contains the selected value
                for (sources, alt_value) in alternatives {
                    let source_list = sources.join(", ");
                    if alt_value == selected {
                        lines.push(format!(
                            "{} = \"{}\"  # [{}]",
                            name,
                            escape_toml_string(alt_value),
                            source_list
                        ));
                    } else {
                        lines.push(format!(
                            "# {} = \"{}\"  # [{}]",
                            name,
                            escape_toml_string(alt_value),
                            source_list
                        ));
                    }
                }
            }
            FieldValue::Empty => {
                lines.push(format!("# {} = \"\"", name));
            }
        }
    }

    // Helper for numeric fields
    fn add_field_numeric(lines: &mut Vec<String>, name: &str, value: &FieldValue) {
        match value {
            FieldValue::Agreed { value: v, sources } => {
                let source_list = sources.join(", ");
                lines.push(format!("{} = {}  # [{}]", name, v, source_list));
            }
            FieldValue::Conflicting {
                selected,
                alternatives,
            } => {
                lines.push(format!("# {}: Sources disagree - pick one:", name));
                for (sources, alt_value) in alternatives {
                    let source_list = sources.join(", ");
                    if alt_value == selected {
                        lines.push(format!("{} = {}  # [{}]", name, alt_value, source_list));
                    } else {
                        lines.push(format!("# {} = {}  # [{}]", name, alt_value, source_list));
                    }
                }
            }
            FieldValue::Empty => {
                lines.push(format!("# {} = 0", name));
            }
        }
    }

    add_field(&mut lines, "title", &merged.title);
    add_field(&mut lines, "author", &merged.author);
    add_field(&mut lines, "narrator", &merged.narrator);
    add_field(&mut lines, "series", &merged.series);
    add_field_numeric(&mut lines, "series_position", &merged.series_position);
    add_field_numeric(&mut lines, "year", &merged.year);
    add_field(&mut lines, "description", &merged.description);
    add_field(&mut lines, "publisher", &merged.publisher);
    add_field(&mut lines, "genre", &merged.genre);
    add_field(&mut lines, "isbn", &merged.isbn);
    add_field(&mut lines, "asin", &merged.asin);

    lines.push(String::new());
    lines.join("\n")
}

/// Escape special characters in TOML strings
fn escape_toml_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

/// Open content in the user's preferred editor
fn open_in_editor(content: &str) -> Result<String> {
    let editor = std::env::var("EDITOR")
        .or_else(|_| std::env::var("VISUAL"))
        .unwrap_or_else(|_| "vi".to_string());

    // Create temp file
    let temp_dir = std::env::temp_dir();
    let temp_path = temp_dir.join("audiobookctl_lookup.toml");

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

/// Apply changes to the file with confirmation and backup
fn apply_changes(
    file: &Path,
    new_metadata: &AudiobookMetadata,
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

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merged_to_toml_agreed_fields() {
        let merged = MergedMetadata {
            title: FieldValue::Agreed {
                value: "The Martian".to_string(),
                sources: vec!["file".to_string(), "audible".to_string()],
            },
            author: FieldValue::Agreed {
                value: "Andy Weir".to_string(),
                sources: vec!["audible".to_string()],
            },
            narrator: FieldValue::Empty,
            series: FieldValue::Empty,
            series_position: FieldValue::Empty,
            year: FieldValue::Agreed {
                value: "2014".to_string(),
                sources: vec!["file".to_string(), "audible".to_string()],
            },
            description: FieldValue::Empty,
            publisher: FieldValue::Empty,
            genre: FieldValue::Empty,
            isbn: FieldValue::Empty,
            asin: FieldValue::Empty,
        };

        let toml = merged_to_toml(&merged);

        assert!(toml.contains("title = \"The Martian\"  # [file, audible]"));
        assert!(toml.contains("author = \"Andy Weir\"  # [audible]"));
        assert!(toml.contains("year = 2014  # [file, audible]"));
    }

    #[test]
    fn test_merged_to_toml_conflicting_fields() {
        let merged = MergedMetadata {
            title: FieldValue::Conflicting {
                selected: "The Martian".to_string(),
                alternatives: vec![
                    (
                        vec!["file".to_string(), "audible".to_string()],
                        "The Martian".to_string(),
                    ),
                    (
                        vec!["openlibrary".to_string()],
                        "The Martian: A Novel".to_string(),
                    ),
                ],
            },
            author: FieldValue::Agreed {
                value: "Andy Weir".to_string(),
                sources: vec!["audible".to_string()],
            },
            narrator: FieldValue::Empty,
            series: FieldValue::Empty,
            series_position: FieldValue::Empty,
            year: FieldValue::Conflicting {
                selected: "2014".to_string(),
                alternatives: vec![
                    (
                        vec!["audible".to_string(), "audnexus".to_string()],
                        "2014".to_string(),
                    ),
                    (vec!["openlibrary".to_string()], "2011".to_string()),
                ],
            },
            description: FieldValue::Empty,
            publisher: FieldValue::Empty,
            genre: FieldValue::Empty,
            isbn: FieldValue::Empty,
            asin: FieldValue::Empty,
        };

        let toml = merged_to_toml(&merged);

        assert!(toml.contains("# title: Sources disagree - pick one:"));
        assert!(toml.contains("title = \"The Martian\"  # [file, audible]"));
        assert!(toml.contains("# title = \"The Martian: A Novel\"  # [openlibrary]"));

        assert!(toml.contains("# year: Sources disagree - pick one:"));
        assert!(toml.contains("year = 2014  # [audible, audnexus]"));
        assert!(toml.contains("# year = 2011  # [openlibrary]"));
    }

    #[test]
    fn test_escape_toml_string() {
        assert_eq!(escape_toml_string("hello"), "hello");
        assert_eq!(escape_toml_string("hello\"world"), "hello\\\"world");
        assert_eq!(escape_toml_string("line1\nline2"), "line1\\nline2");
        assert_eq!(escape_toml_string("path\\to\\file"), "path\\\\to\\\\file");
    }
}
