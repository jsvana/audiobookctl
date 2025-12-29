# Batch Lookup Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add batch lookup command with backup limits, plus deduplicated source display and early-exit when no changes.

**Architecture:** Refactor merge logic to group sources by value, extract shared lookup functionality, add new `lookup-all` and `backups` commands.

**Tech Stack:** Rust, clap, tokio, serde/toml

---

## Task 1: Update FieldValue to Store Grouped Sources

**Files:**
- Modify: `src/lookup/merge.rs:7-18`
- Test: `src/lookup/merge.rs` (inline tests)

**Step 1: Write failing test for source grouping**

Add this test to `src/lookup/merge.rs` in the tests module:

```rust
#[test]
fn test_merge_field_groups_agreeing_sources() {
    // When multiple sources have the same value, they should be grouped together
    let existing = None;
    let results = vec![
        ("audible".to_string(), Some("The Martian".to_string())),
        ("openlibrary".to_string(), Some("The Martian".to_string())),
        ("audnexus".to_string(), Some("The Martian".to_string())),
    ];

    let result = merge_field(&existing, &results);
    match result {
        FieldValue::Agreed { value, sources } => {
            assert_eq!(value, "The Martian");
            assert_eq!(sources, vec!["audible", "openlibrary", "audnexus"]);
        }
        _ => panic!("Expected Agreed with sources, got {:?}", result),
    }
}

#[test]
fn test_merge_field_groups_conflicting_by_value() {
    // When sources disagree, group by value
    let existing = None;
    let results = vec![
        ("audible".to_string(), Some("The Martian".to_string())),
        ("audnexus".to_string(), Some("The Martian".to_string())),
        ("openlibrary".to_string(), Some("The Martian: A Novel".to_string())),
    ];

    let result = merge_field(&existing, &results);
    match result {
        FieldValue::Conflicting { selected, alternatives } => {
            assert_eq!(selected, "The Martian");
            // Alternatives should be grouped: (sources, value)
            assert_eq!(alternatives.len(), 2);
            assert_eq!(alternatives[0], (vec!["audible".to_string(), "audnexus".to_string()], "The Martian".to_string()));
            assert_eq!(alternatives[1], (vec!["openlibrary".to_string()], "The Martian: A Novel".to_string()));
        }
        _ => panic!("Expected Conflicting, got {:?}", result),
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --lib merge_field_groups -- --nocapture`
Expected: Compilation error - FieldValue::Agreed doesn't have sources field

**Step 3: Update FieldValue enum**

Replace the FieldValue enum in `src/lookup/merge.rs:7-18`:

```rust
/// Represents a field's merged state
#[derive(Debug, Clone, PartialEq)]
pub enum FieldValue {
    /// All sources agree on this value
    Agreed {
        value: String,
        sources: Vec<String>,
    },
    /// Sources disagree - alternatives grouped by value
    Conflicting {
        selected: String,
        alternatives: Vec<(Vec<String>, String)>, // (source_names, value)
    },
    /// No source has this field
    Empty,
}
```

**Step 4: Update merge_field function**

Replace `merge_field` function in `src/lookup/merge.rs:45-94`:

```rust
fn merge_field(
    existing: &Option<String>,
    results: &[(String, Option<String>)],
) -> FieldValue {
    use std::collections::HashMap;

    // Build list of all sources including existing file metadata
    let mut all_sources: Vec<(String, Option<String>)> = Vec::new();

    if existing.is_some() {
        all_sources.push(("file".to_string(), existing.clone()));
    }
    all_sources.extend(results.iter().cloned());

    // Group sources by value
    let mut value_to_sources: HashMap<String, Vec<String>> = HashMap::new();
    for (source, value) in &all_sources {
        if let Some(v) = value {
            value_to_sources
                .entry(v.clone())
                .or_default()
                .push(source.clone());
        }
    }

    if value_to_sources.is_empty() {
        return FieldValue::Empty;
    }

    // Convert to ordered list (preserve insertion order via all_sources)
    let mut seen_values: Vec<String> = Vec::new();
    for (_, value) in &all_sources {
        if let Some(v) = value {
            if !seen_values.contains(v) {
                seen_values.push(v.clone());
            }
        }
    }

    let grouped: Vec<(Vec<String>, String)> = seen_values
        .iter()
        .map(|v| (value_to_sources.get(v).unwrap().clone(), v.clone()))
        .collect();

    if grouped.len() == 1 {
        let (sources, value) = grouped.into_iter().next().unwrap();
        FieldValue::Agreed { value, sources }
    } else {
        // Select existing value if present, otherwise first value
        let selected = if let Some(existing_val) = existing {
            existing_val.clone()
        } else {
            grouped[0].1.clone()
        };

        FieldValue::Conflicting {
            selected,
            alternatives: grouped,
        }
    }
}
```

**Step 5: Run tests to verify they pass**

Run: `cargo test --lib merge`
Expected: New tests pass, but existing tests fail due to changed FieldValue structure

**Step 6: Update existing tests for new FieldValue structure**

Update all tests in `src/lookup/merge.rs` that pattern match on FieldValue. Change:
- `FieldValue::Agreed(v)` → `FieldValue::Agreed { value: v, sources: _ }`
- `FieldValue::Conflicting { selected, alternatives }` where alternatives is `Vec<(String, String)>` → `Vec<(Vec<String>, String)>`

**Step 7: Run all merge tests**

Run: `cargo test --lib merge`
Expected: PASS

**Step 8: Commit**

```bash
git add src/lookup/merge.rs
git commit -m "feat(lookup): group sources by value in merge results

Update FieldValue enum to track which sources agree on each value.
- Agreed variant now includes list of agreeing sources
- Conflicting alternatives grouped by value with source lists
- Enables deduplicated display in TOML output

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Task 2: Update TOML Display for Grouped Sources

**Files:**
- Modify: `src/commands/lookup.rs:142-212`
- Test: `src/commands/lookup.rs` (inline tests)

**Step 1: Update merged_to_toml helper functions**

Replace the `add_field` and `add_field_numeric` helper functions inside `merged_to_toml` in `src/commands/lookup.rs`:

```rust
fn add_field(lines: &mut Vec<String>, name: &str, value: &FieldValue) {
    match value {
        FieldValue::Agreed { value: v, sources } => {
            let source_list = sources.join(", ");
            lines.push(format!("{} = \"{}\"  # [{}]", name, escape_toml_string(v), source_list));
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
                    lines.push(format!("{} = \"{}\"  # [{}]", name, escape_toml_string(alt_value), source_list));
                } else {
                    lines.push(format!("# {} = \"{}\"  # [{}]", name, escape_toml_string(alt_value), source_list));
                }
            }
        }
        FieldValue::Empty => {
            lines.push(format!("# {} = \"\"", name));
        }
    }
}

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
```

**Step 2: Update tests for new TOML format**

Update the tests in `src/commands/lookup.rs`:

```rust
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
                (vec!["file".to_string(), "audible".to_string()], "The Martian".to_string()),
                (vec!["openlibrary".to_string()], "The Martian: A Novel".to_string()),
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
                (vec!["audible".to_string(), "audnexus".to_string()], "2014".to_string()),
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
```

**Step 3: Run tests**

Run: `cargo test --lib lookup`
Expected: PASS

**Step 4: Commit**

```bash
git add src/commands/lookup.rs
git commit -m "feat(lookup): show grouped sources in TOML display

Display format now shows which sources agree:
- Agreed: title = \"Book\"  # [file, audible, openlibrary]
- Conflicting: groups sources by value

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Task 3: Add Early-Exit When No Changes

**Files:**
- Modify: `src/commands/lookup.rs`
- Modify: `src/lookup/merge.rs`

**Step 1: Add helper to check if all fields match file**

Add to `src/lookup/merge.rs` after the `MergedMetadata` struct:

```rust
impl MergedMetadata {
    /// Check if all fields either match the file or are empty
    /// Returns the sources that were checked if no changes needed
    pub fn matches_file(&self) -> Option<Vec<String>> {
        let fields = [
            &self.title, &self.author, &self.narrator, &self.series,
            &self.series_position, &self.year, &self.description,
            &self.publisher, &self.genre, &self.isbn, &self.asin,
        ];

        let mut all_sources: Vec<String> = Vec::new();

        for field in fields {
            match field {
                FieldValue::Agreed { sources, .. } => {
                    // If file is in sources and all agree, that's fine
                    for s in sources {
                        if s != "file" && !all_sources.contains(s) {
                            all_sources.push(s.clone());
                        }
                    }
                }
                FieldValue::Conflicting { .. } => {
                    // Any conflict means changes available
                    return None;
                }
                FieldValue::Empty => {
                    // Empty is fine
                }
            }
        }

        if all_sources.is_empty() {
            None // No sources checked
        } else {
            Some(all_sources)
        }
    }
}
```

**Step 2: Add test for matches_file**

Add to tests in `src/lookup/merge.rs`:

```rust
#[test]
fn test_matches_file_all_agree() {
    let merged = MergedMetadata {
        title: FieldValue::Agreed {
            value: "Book".to_string(),
            sources: vec!["file".to_string(), "audible".to_string()],
        },
        author: FieldValue::Empty,
        narrator: FieldValue::Empty,
        series: FieldValue::Empty,
        series_position: FieldValue::Empty,
        year: FieldValue::Empty,
        description: FieldValue::Empty,
        publisher: FieldValue::Empty,
        genre: FieldValue::Empty,
        isbn: FieldValue::Empty,
        asin: FieldValue::Empty,
    };

    let result = merged.matches_file();
    assert_eq!(result, Some(vec!["audible".to_string()]));
}

#[test]
fn test_matches_file_has_conflicts() {
    let merged = MergedMetadata {
        title: FieldValue::Conflicting {
            selected: "Book".to_string(),
            alternatives: vec![
                (vec!["file".to_string()], "Book".to_string()),
                (vec!["audible".to_string()], "Other".to_string()),
            ],
        },
        author: FieldValue::Empty,
        narrator: FieldValue::Empty,
        series: FieldValue::Empty,
        series_position: FieldValue::Empty,
        year: FieldValue::Empty,
        description: FieldValue::Empty,
        publisher: FieldValue::Empty,
        genre: FieldValue::Empty,
        isbn: FieldValue::Empty,
        asin: FieldValue::Empty,
    };

    assert_eq!(merged.matches_file(), None);
}
```

**Step 3: Run test**

Run: `cargo test --lib matches_file`
Expected: PASS

**Step 4: Add early exit to lookup command**

In `src/commands/lookup.rs`, after line 29 (after `let merged = merge_results(...)`), add:

```rust
// Check for early exit if no changes
if let Some(sources) = merged.matches_file() {
    println!(
        "{}: metadata matches [{}] - skipping",
        file.display(),
        sources.join(", ")
    );
    return Ok(());
}
```

**Step 5: Run full test suite**

Run: `cargo test`
Expected: PASS

**Step 6: Commit**

```bash
git add src/lookup/merge.rs src/commands/lookup.rs
git commit -m "feat(lookup): early exit when metadata already matches

Skip editor and show message when file metadata matches all lookup
sources. Displays which sources were checked.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Task 4: Add Backups Config

**Files:**
- Modify: `src/config.rs`

**Step 1: Add BackupsConfig struct**

Add after `OrganizeConfig` in `src/config.rs`:

```rust
/// Configuration for backup management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupsConfig {
    /// Maximum storage allowed for backups in bytes (default: 2GB)
    #[serde(default = "default_max_storage")]
    pub max_storage_bytes: u64,
}

fn default_max_storage() -> u64 {
    2 * 1024 * 1024 * 1024 // 2GB
}

impl Default for BackupsConfig {
    fn default() -> Self {
        Self {
            max_storage_bytes: default_max_storage(),
        }
    }
}
```

**Step 2: Add to Config struct**

Update `Config` struct in `src/config.rs`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub organize: OrganizeConfig,
    #[serde(default)]
    pub backups: BackupsConfig,
}
```

**Step 3: Add test for backups config**

Add test in `src/config.rs`:

```rust
#[test]
fn test_backups_config_defaults() {
    let config = Config::default();
    assert_eq!(config.backups.max_storage_bytes, 2 * 1024 * 1024 * 1024);
}

#[test]
fn test_load_with_backups_config() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("config.toml");
    std::fs::write(
        &path,
        r#"
[backups]
max_storage_bytes = 1073741824
"#,
    )
    .unwrap();

    let config = Config::load_from(&path).unwrap();
    assert_eq!(config.backups.max_storage_bytes, 1024 * 1024 * 1024); // 1GB
}
```

**Step 4: Run tests**

Run: `cargo test --lib config`
Expected: PASS

**Step 5: Commit**

```bash
git add src/config.rs
git commit -m "feat(config): add backups configuration

Add max_storage_bytes setting for backup limits (default: 2GB).

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Task 5: Create Backups Command

**Files:**
- Create: `src/commands/backups.rs`
- Modify: `src/commands/mod.rs`
- Modify: `src/cli.rs`
- Modify: `src/main.rs`

**Step 1: Create backups.rs**

Create `src/commands/backups.rs`:

```rust
//! Backups command - manage .bak files

use crate::config::Config;
use crate::safety::backup::{find_all_backups, format_size, BackupInfo};
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
```

**Step 2: Update mod.rs**

Add to `src/commands/mod.rs`:

```rust
pub mod backups;
```

**Step 3: Add CLI command**

Add to `Commands` enum in `src/cli.rs`:

```rust
/// Manage backup files
Backups {
    #[command(subcommand)]
    action: BackupsAction,
},
```

And add the subcommand enum:

```rust
#[derive(Subcommand)]
pub enum BackupsAction {
    /// List all backup files and total size
    List {
        /// Directory to scan (current directory if not specified)
        #[arg(default_value = ".")]
        dir: std::path::PathBuf,
    },
    /// Clean backup files
    Clean {
        /// Directory to scan (current directory if not specified)
        #[arg(default_value = ".")]
        dir: std::path::PathBuf,

        /// Delete all backups without prompting for each
        #[arg(long)]
        all: bool,

        /// Skip confirmation prompt (with --all)
        #[arg(long)]
        yes: bool,
    },
}
```

**Step 4: Add to main.rs**

Add match arm in `src/main.rs`:

```rust
Commands::Backups { action } => {
    use cli::BackupsAction;
    match action {
        BackupsAction::List { dir } => {
            commands::backups::list(&dir)?;
        }
        BackupsAction::Clean { dir, all, yes } => {
            commands::backups::clean(&dir, all, yes)?;
        }
    }
}
```

**Step 5: Run build and tests**

Run: `cargo build && cargo test`
Expected: PASS

**Step 6: Commit**

```bash
git add src/commands/backups.rs src/commands/mod.rs src/cli.rs src/main.rs
git commit -m "feat: add backups command for managing .bak files

New commands:
- audiobookctl backups list <dir>
- audiobookctl backups clean <dir> [--all] [--yes]

Shows storage usage against configured limit.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Task 6: Extract Shared Lookup Functions

**Files:**
- Modify: `src/commands/lookup.rs`

**Step 1: Make query_apis_sync public and extract core logic**

Refactor `src/commands/lookup.rs` to expose reusable functions. Add `pub` to these functions and extract a new `lookup_single` function:

```rust
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
```

**Step 2: Update run() to use extracted functions**

Update the `run` function in `src/commands/lookup.rs`:

```rust
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
```

**Step 3: Run tests**

Run: `cargo test`
Expected: PASS

**Step 4: Commit**

```bash
git add src/commands/lookup.rs
git commit -m "refactor(lookup): extract shared functions for batch lookup

Make query_and_merge and process_lookup public for reuse by lookup-all.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Task 7: Create Lookup-All Command

**Files:**
- Create: `src/commands/lookup_all.rs`
- Modify: `src/commands/mod.rs`
- Modify: `src/cli.rs`
- Modify: `src/main.rs`

**Step 1: Create lookup_all.rs**

Create `src/commands/lookup_all.rs`:

```rust
//! Lookup-all command - batch metadata lookup with queue mode

use crate::commands::backups::current_usage;
use crate::commands::lookup::{merged_to_toml, process_lookup, query_and_merge};
use crate::config::Config;
use crate::editor::{compute_changes, toml_to_metadata};
use crate::lookup::MergedMetadata;
use crate::metadata::{write_metadata, AudiobookMetadata};
use crate::organize::scanner::scan_directory;
use crate::safety::backup::{create_backup, format_size};
use anyhow::{Context, Result};
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
        print!(
            "[{}/{}] Checking {}... ",
            i + 1,
            files.len(),
            file.filename
        );
        io::stdout().flush()?;

        match query_and_merge(&file.path) {
            Ok((original, merged, sources)) => {
                if let Some(matched_sources) = merged.matches_file() {
                    println!("matches [{}] - skipping", matched_sources.join(", "));
                    skipped += 1;
                } else {
                    println!("updates available from [{}]", sources.join(", "));
                    let file_size = fs::metadata(&file.path)
                        .map(|m| m.len())
                        .unwrap_or(0);
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

        if auto_accept {
            process_auto_accept(&item.path, &item.original, &item.merged, no_dry_run, no_backup)?;
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

    fields.iter().any(|f| matches!(f, FieldValue::Conflicting { .. }))
}
```

**Step 2: Update mod.rs**

Add to `src/commands/mod.rs`:

```rust
pub mod lookup_all;
```

**Step 3: Add CLI command**

Add to `Commands` enum in `src/cli.rs`:

```rust
/// Look up metadata for all audiobooks in a directory
LookupAll {
    /// Directory to scan
    dir: std::path::PathBuf,

    /// Auto-apply when all sources agree (skip editor)
    #[arg(long)]
    auto_accept: bool,

    /// Actually apply changes (default: dry-run)
    #[arg(long)]
    no_dry_run: bool,

    /// Skip confirmation prompts
    #[arg(long)]
    yes: bool,

    /// Skip creating backup files
    #[arg(long = "no-backup-i-void-my-warranty")]
    no_backup: bool,
},
```

**Step 4: Add to main.rs**

Add match arm in `src/main.rs`:

```rust
Commands::LookupAll {
    dir,
    auto_accept,
    no_dry_run,
    yes,
    no_backup,
} => {
    commands::lookup_all::run(&dir, auto_accept, no_dry_run, yes, no_backup)?;
}
```

**Step 5: Build and test**

Run: `cargo build && cargo test`
Expected: PASS

**Step 6: Commit**

```bash
git add src/commands/lookup_all.rs src/commands/mod.rs src/cli.rs src/main.rs
git commit -m "feat: add lookup-all command for batch metadata lookup

New command: audiobookctl lookup-all <dir>

Features:
- Queue mode: scans all files, shows summary, processes interactively
- --auto-accept: auto-apply when all sources agree
- Backup storage limits enforced from config
- Skips files where metadata already matches

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Task 8: Add Integration Tests

**Files:**
- Create: `tests/lookup_all_tests.rs`

**Step 1: Create CLI tests**

Create `tests/lookup_all_tests.rs`:

```rust
use assert_cmd::Command;

#[test]
fn test_lookup_all_help() {
    let mut cmd = Command::cargo_bin("audiobookctl").unwrap();
    cmd.args(["lookup-all", "--help"])
        .assert()
        .success()
        .stdout(predicates::str::contains("Look up metadata for all audiobooks"));
}

#[test]
fn test_lookup_all_empty_directory() {
    let temp = tempfile::tempdir().unwrap();

    let mut cmd = Command::cargo_bin("audiobookctl").unwrap();
    cmd.args(["lookup-all", temp.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicates::str::contains("No .m4b files found"));
}

#[test]
fn test_backups_list_help() {
    let mut cmd = Command::cargo_bin("audiobookctl").unwrap();
    cmd.args(["backups", "list", "--help"])
        .assert()
        .success()
        .stdout(predicates::str::contains("List all backup files"));
}

#[test]
fn test_backups_clean_help() {
    let mut cmd = Command::cargo_bin("audiobookctl").unwrap();
    cmd.args(["backups", "clean", "--help"])
        .assert()
        .success()
        .stdout(predicates::str::contains("Clean backup files"));
}
```

**Step 2: Run tests**

Run: `cargo test --test lookup_all_tests`
Expected: PASS

**Step 3: Commit**

```bash
git add tests/lookup_all_tests.rs
git commit -m "test: add integration tests for lookup-all and backups

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Task 9: Final Verification

**Step 1: Run full test suite**

Run: `cargo test`
Expected: All tests pass

**Step 2: Run clippy**

Run: `cargo clippy`
Expected: No warnings

**Step 3: Run fmt check**

Run: `cargo fmt --check`
Expected: No formatting issues

**Step 4: Build release**

Run: `cargo build --release`
Expected: Success

**Step 5: Manual smoke test**

Run: `./target/release/audiobookctl lookup-all --help`
Run: `./target/release/audiobookctl backups list .`
Expected: Help displays correctly, backups list works

**Step 6: Final commit if any fixes needed**

Only if changes were required from clippy/fmt.
