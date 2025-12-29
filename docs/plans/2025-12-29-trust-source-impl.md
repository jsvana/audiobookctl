# `--trust-source` Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add `--trust-source <audible|audnexus|openlibrary>` flag to auto-accept values from a designated source without conflict resolution.

**Architecture:** Add a `TrustedSource` enum with clap value parsing, thread it through CLI to merge logic, and add `resolve_with_trusted_source()` function that converts conflicts to agreed values using the trusted source.

**Tech Stack:** Rust, clap 4 (with `ValueEnum`), existing merge infrastructure

---

### Task 1: Add TrustedSource enum

**Files:**
- Create: `src/lookup/trusted.rs`
- Modify: `src/lookup/mod.rs:1-10`

**Step 1: Write the failing test**

Create `src/lookup/trusted.rs`:

```rust
//! Trusted source handling for auto-accept lookups

use clap::ValueEnum;

/// Valid sources that can be trusted for auto-accept
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum TrustedSource {
    Audible,
    Audnexus,
    Openlibrary,
}

impl TrustedSource {
    /// Get the source name as it appears in LookupResult.source
    pub fn as_str(&self) -> &'static str {
        match self {
            TrustedSource::Audible => "audible",
            TrustedSource::Audnexus => "audnexus",
            TrustedSource::Openlibrary => "openlibrary",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trusted_source_as_str() {
        assert_eq!(TrustedSource::Audible.as_str(), "audible");
        assert_eq!(TrustedSource::Audnexus.as_str(), "audnexus");
        assert_eq!(TrustedSource::Openlibrary.as_str(), "openlibrary");
    }
}
```

**Step 2: Update mod.rs to export**

In `src/lookup/mod.rs`, add:

```rust
mod trusted;
pub use trusted::TrustedSource;
```

**Step 3: Run test to verify it passes**

Run: `cargo test lookup::trusted::tests::test_trusted_source_as_str`
Expected: PASS

**Step 4: Commit**

```bash
git add src/lookup/trusted.rs src/lookup/mod.rs
git commit -m "feat(lookup): add TrustedSource enum for --trust-source flag"
```

---

### Task 2: Add resolve_with_trusted_source function

**Files:**
- Modify: `src/lookup/merge.rs:1-20` (add function and tests)

**Step 1: Write the failing test**

Add to `src/lookup/merge.rs` tests module:

```rust
#[test]
fn test_resolve_trusted_source_wins_conflict() {
    use crate::lookup::TrustedSource;

    let merged = MergedMetadata {
        title: FieldValue::Conflicting {
            selected: "File Title".to_string(),
            alternatives: vec![
                (vec!["file".to_string()], "File Title".to_string()),
                (vec!["audible".to_string()], "Audible Title".to_string()),
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

    let resolved = resolve_with_trusted_source(&merged, TrustedSource::Audible);

    match &resolved.title {
        FieldValue::Agreed { value, sources } => {
            assert_eq!(value, "Audible Title");
            assert_eq!(sources, &vec!["audible".to_string()]);
        }
        _ => panic!("Expected Agreed, got {:?}", resolved.title),
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test lookup::merge::tests::test_resolve_trusted_source_wins_conflict`
Expected: FAIL with "cannot find function `resolve_with_trusted_source`"

**Step 3: Write the implementation**

Add to `src/lookup/merge.rs` before the tests module:

```rust
use crate::lookup::TrustedSource;

/// Resolve a single field using trusted source
fn resolve_field_with_trusted(field: &FieldValue, trusted: &str) -> FieldValue {
    match field {
        FieldValue::Conflicting { alternatives, .. } => {
            // Find the trusted source's value
            for (sources, value) in alternatives {
                if sources.iter().any(|s| s == trusted) {
                    return FieldValue::Agreed {
                        value: value.clone(),
                        sources: sources.clone(),
                    };
                }
            }
            // Trusted source not in alternatives, keep as-is
            field.clone()
        }
        // Non-conflicts pass through unchanged
        other => other.clone(),
    }
}

/// Resolve all conflicts in merged metadata using trusted source
///
/// Converts Conflicting fields to Agreed when the trusted source has a value.
/// Non-conflicting fields pass through unchanged.
pub fn resolve_with_trusted_source(merged: &MergedMetadata, trusted: TrustedSource) -> MergedMetadata {
    let trusted_str = trusted.as_str();

    MergedMetadata {
        title: resolve_field_with_trusted(&merged.title, trusted_str),
        author: resolve_field_with_trusted(&merged.author, trusted_str),
        narrator: resolve_field_with_trusted(&merged.narrator, trusted_str),
        series: resolve_field_with_trusted(&merged.series, trusted_str),
        series_position: resolve_field_with_trusted(&merged.series_position, trusted_str),
        year: resolve_field_with_trusted(&merged.year, trusted_str),
        description: resolve_field_with_trusted(&merged.description, trusted_str),
        publisher: resolve_field_with_trusted(&merged.publisher, trusted_str),
        genre: resolve_field_with_trusted(&merged.genre, trusted_str),
        isbn: resolve_field_with_trusted(&merged.isbn, trusted_str),
        asin: resolve_field_with_trusted(&merged.asin, trusted_str),
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test lookup::merge::tests::test_resolve_trusted_source_wins_conflict`
Expected: PASS

**Step 5: Commit**

```bash
git add src/lookup/merge.rs
git commit -m "feat(lookup): add resolve_with_trusted_source function"
```

---

### Task 3: Add more merge tests

**Files:**
- Modify: `src/lookup/merge.rs` (tests module)

**Step 1: Add test for preserving file-only values**

```rust
#[test]
fn test_resolve_trusted_preserves_file_only_values() {
    use crate::lookup::TrustedSource;

    let merged = MergedMetadata {
        title: FieldValue::Agreed {
            value: "File Title".to_string(),
            sources: vec!["file".to_string()],
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

    let resolved = resolve_with_trusted_source(&merged, TrustedSource::Audible);

    // File-only value should be preserved
    match &resolved.title {
        FieldValue::Agreed { value, sources } => {
            assert_eq!(value, "File Title");
            assert_eq!(sources, &vec!["file".to_string()]);
        }
        _ => panic!("Expected Agreed from file, got {:?}", resolved.title),
    }
}
```

**Step 2: Add test for trusted source not in conflict**

```rust
#[test]
fn test_resolve_trusted_not_in_conflict_keeps_original() {
    use crate::lookup::TrustedSource;

    // Conflict between file and openlibrary, but we trust audible
    let merged = MergedMetadata {
        title: FieldValue::Conflicting {
            selected: "File Title".to_string(),
            alternatives: vec![
                (vec!["file".to_string()], "File Title".to_string()),
                (vec!["openlibrary".to_string()], "OL Title".to_string()),
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

    let resolved = resolve_with_trusted_source(&merged, TrustedSource::Audible);

    // Audible not in conflict, so keep original conflict
    match &resolved.title {
        FieldValue::Conflicting { selected, .. } => {
            assert_eq!(selected, "File Title");
        }
        _ => panic!("Expected Conflicting (audible not present), got {:?}", resolved.title),
    }
}
```

**Step 3: Run all new tests**

Run: `cargo test lookup::merge::tests::test_resolve_trusted`
Expected: PASS

**Step 4: Commit**

```bash
git add src/lookup/merge.rs
git commit -m "test(lookup): add more trusted source resolution tests"
```

---

### Task 4: Add has_trusted_source_data function

**Files:**
- Modify: `src/lookup/merge.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn test_has_trusted_source_data_returns_true_when_present() {
    use crate::lookup::TrustedSource;

    let merged = MergedMetadata {
        title: FieldValue::Agreed {
            value: "Title".to_string(),
            sources: vec!["audible".to_string()],
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

    assert!(has_trusted_source_data(&merged, TrustedSource::Audible));
}

#[test]
fn test_has_trusted_source_data_returns_false_when_missing() {
    use crate::lookup::TrustedSource;

    let merged = MergedMetadata {
        title: FieldValue::Agreed {
            value: "Title".to_string(),
            sources: vec!["openlibrary".to_string()],
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

    assert!(!has_trusted_source_data(&merged, TrustedSource::Audible));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test lookup::merge::tests::test_has_trusted_source_data`
Expected: FAIL with "cannot find function"

**Step 3: Write the implementation**

Add to `src/lookup/merge.rs`:

```rust
/// Check if trusted source provided any data in the merged result
///
/// Returns true if the trusted source appears in any field's sources.
/// Used to skip files when trusted source returned no results.
pub fn has_trusted_source_data(merged: &MergedMetadata, trusted: TrustedSource) -> bool {
    let trusted_str = trusted.as_str();

    fn field_has_source(field: &FieldValue, source: &str) -> bool {
        match field {
            FieldValue::Agreed { sources, .. } => sources.iter().any(|s| s == source),
            FieldValue::Conflicting { alternatives, .. } => {
                alternatives.iter().any(|(sources, _)| sources.iter().any(|s| s == source))
            }
            FieldValue::Empty => false,
        }
    }

    field_has_source(&merged.title, trusted_str)
        || field_has_source(&merged.author, trusted_str)
        || field_has_source(&merged.narrator, trusted_str)
        || field_has_source(&merged.series, trusted_str)
        || field_has_source(&merged.series_position, trusted_str)
        || field_has_source(&merged.year, trusted_str)
        || field_has_source(&merged.description, trusted_str)
        || field_has_source(&merged.publisher, trusted_str)
        || field_has_source(&merged.genre, trusted_str)
        || field_has_source(&merged.isbn, trusted_str)
        || field_has_source(&merged.asin, trusted_str)
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test lookup::merge::tests::test_has_trusted_source_data`
Expected: PASS

**Step 5: Commit**

```bash
git add src/lookup/merge.rs
git commit -m "feat(lookup): add has_trusted_source_data check function"
```

---

### Task 5: Update mod.rs exports

**Files:**
- Modify: `src/lookup/mod.rs`

**Step 1: Update exports**

Current `src/lookup/mod.rs` likely has:
```rust
mod api;
mod merge;

pub use api::{fetch_audible, fetch_audnexus, fetch_openlibrary, LookupResult};
pub use merge::{merge_results, FieldValue, MergedMetadata};
```

Update to:
```rust
mod api;
mod merge;
mod trusted;

pub use api::{fetch_audible, fetch_audnexus, fetch_openlibrary, LookupResult};
pub use merge::{merge_results, resolve_with_trusted_source, has_trusted_source_data, FieldValue, MergedMetadata};
pub use trusted::TrustedSource;
```

**Step 2: Verify compilation**

Run: `cargo check`
Expected: SUCCESS

**Step 3: Commit**

```bash
git add src/lookup/mod.rs
git commit -m "feat(lookup): export trusted source functions"
```

---

### Task 6: Add --trust-source to CLI

**Files:**
- Modify: `src/cli.rs:63-80` (Lookup command)
- Modify: `src/cli.rs:81-102` (LookupAll command)

**Step 1: Add import and update Lookup command**

At top of `src/cli.rs`, add:
```rust
use crate::lookup::TrustedSource;
```

Update `Lookup` variant:
```rust
    /// Look up metadata from online sources (Audnexus, Open Library)
    Lookup {
        /// Path to the m4b file
        file: PathBuf,

        /// Actually apply changes (default: dry-run)
        #[arg(long)]
        no_dry_run: bool,

        /// Skip confirmation prompt
        #[arg(long)]
        yes: bool,

        /// Skip creating backup file
        #[arg(long = "no-backup-i-void-my-warranty")]
        no_backup: bool,

        /// Trust this source and auto-accept its values (skip editor for conflicts)
        #[arg(long, value_enum)]
        trust_source: Option<TrustedSource>,
    },
```

**Step 2: Update LookupAll command**

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

        /// Trust this source and auto-accept its values (skip editor for conflicts)
        #[arg(long, value_enum)]
        trust_source: Option<TrustedSource>,
    },
```

**Step 3: Verify compilation**

Run: `cargo check`
Expected: FAIL (main.rs doesn't handle new field yet)

**Step 4: Commit partial progress**

```bash
git add src/cli.rs
git commit -m "feat(cli): add --trust-source argument to lookup commands"
```

---

### Task 7: Update main.rs to pass trust_source

**Files:**
- Modify: `src/main.rs:38-54`

**Step 1: Update Lookup match arm**

```rust
        Commands::Lookup {
            file,
            no_dry_run,
            yes,
            no_backup,
            trust_source,
        } => {
            commands::lookup::run(&file, no_dry_run, yes, no_backup, trust_source)?;
        }
```

**Step 2: Update LookupAll match arm**

```rust
        Commands::LookupAll {
            dir,
            auto_accept,
            no_dry_run,
            yes,
            no_backup,
            trust_source,
        } => {
            commands::lookup_all::run(&dir, auto_accept, no_dry_run, yes, no_backup, trust_source)?;
        }
```

**Step 3: Verify compilation**

Run: `cargo check`
Expected: FAIL (command functions don't accept trust_source yet)

**Step 4: Commit**

```bash
git add src/main.rs
git commit -m "feat(main): pass trust_source to lookup commands"
```

---

### Task 8: Update lookup command

**Files:**
- Modify: `src/commands/lookup.rs:77-96`

**Step 1: Update run function signature**

Change:
```rust
pub fn run(file: &Path, no_dry_run: bool, yes: bool, no_backup: bool) -> Result<()> {
```

To:
```rust
use crate::lookup::TrustedSource;

pub fn run(file: &Path, no_dry_run: bool, yes: bool, no_backup: bool, trust_source: Option<TrustedSource>) -> Result<()> {
```

**Step 2: Add trusted source logic**

Replace the function body:

```rust
pub fn run(file: &Path, no_dry_run: bool, yes: bool, no_backup: bool, trust_source: Option<TrustedSource>) -> Result<()> {
    use crate::lookup::{has_trusted_source_data, resolve_with_trusted_source};

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

    // Handle trusted source mode
    if let Some(trusted) = trust_source {
        if !has_trusted_source_data(&merged, trusted) {
            println!(
                "Skipping {}: trusted source '{}' returned no results",
                file.display(),
                trusted.as_str()
            );
            return Ok(());
        }

        let resolved = resolve_with_trusted_source(&merged, trusted);
        return process_trusted_lookup(file, &original, &resolved, no_dry_run, no_backup, trusted);
    }

    process_lookup(file, &original, &merged, no_dry_run, yes, no_backup)?;

    Ok(())
}
```

**Step 3: Add process_trusted_lookup function**

Add before the `run` function:

```rust
/// Process lookup with trusted source (no editor, auto-apply)
fn process_trusted_lookup(
    file: &Path,
    original: &AudiobookMetadata,
    resolved: &MergedMetadata,
    no_dry_run: bool,
    no_backup: bool,
    trusted: crate::lookup::TrustedSource,
) -> Result<()> {
    use crate::editor::compute_changes;
    use crate::safety::create_backup;

    // Generate metadata from resolved merge
    let toml = merged_to_toml(resolved);
    let new_metadata = toml_to_metadata(&toml)?;
    let changes = compute_changes(original, &new_metadata);

    if changes.is_empty() {
        println!("No changes from trusted source '{}'.", trusted.as_str());
        return Ok(());
    }

    // Show what will be applied
    let fields: Vec<&str> = changes.iter().map(|c| c.field.as_str()).collect();
    println!("Trusted source '{}': applying {}", trusted.as_str(), fields.join(", "));

    if no_dry_run {
        if !no_backup {
            let backup = create_backup(file)?;
            println!("  Created backup: {}", backup.display());
        }
        write_metadata(file, &new_metadata)?;
        println!("  Applied.");
    } else {
        // Save to pending cache
        let cache = PendingEditsCache::new()?;
        cache.save(file, &toml)?;
        println!("  (dry-run) Saved to pending. Use --no-dry-run to apply.");
    }

    Ok(())
}
```

**Step 4: Verify compilation**

Run: `cargo check`
Expected: FAIL (lookup_all doesn't accept trust_source yet)

**Step 5: Commit**

```bash
git add src/commands/lookup.rs
git commit -m "feat(lookup): implement --trust-source for single file lookup"
```

---

### Task 9: Update lookup_all command

**Files:**
- Modify: `src/commands/lookup_all.rs:25-133`

**Step 1: Update run function signature**

Change:
```rust
pub fn run(
    dir: &Path,
    auto_accept: bool,
    no_dry_run: bool,
    yes: bool,
    no_backup: bool,
) -> Result<()> {
```

To:
```rust
use crate::lookup::TrustedSource;

pub fn run(
    dir: &Path,
    auto_accept: bool,
    no_dry_run: bool,
    yes: bool,
    no_backup: bool,
    trust_source: Option<TrustedSource>,
) -> Result<()> {
```

**Step 2: Add trusted source logic in processing loop**

Find the section around line 51-75 that queries APIs, and add this after `query_and_merge`:

```rust
            Ok((original, merged, sources)) => {
                // Check if trusted source has data
                if let Some(trusted) = trust_source {
                    if !crate::lookup::has_trusted_source_data(&merged, trusted) {
                        println!("skipped (trusted source '{}' has no data)", trusted.as_str());
                        skipped += 1;
                        continue;
                    }
                }

                if let Some(matched_sources) = merged.matches_file() {
                    println!("matches [{}] - skipping", matched_sources.join(", "));
                    skipped += 1;
                } else {
                    // ... rest of the logic
```

**Step 3: Update processing section**

Around line 102-130, update the processing loop:

```rust
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
```

**Step 4: Add process_trusted_accept function**

```rust
/// Auto-accept using trusted source values
fn process_trusted_accept(
    file: &Path,
    original: &AudiobookMetadata,
    resolved: &MergedMetadata,
    no_dry_run: bool,
    no_backup: bool,
    trusted: TrustedSource,
) -> Result<()> {
    use crate::commands::lookup::merged_to_toml;
    use crate::editor::{compute_changes, toml_to_metadata};

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
```

**Step 5: Verify compilation**

Run: `cargo check`
Expected: SUCCESS

**Step 6: Commit**

```bash
git add src/commands/lookup_all.rs
git commit -m "feat(lookup-all): implement --trust-source for batch lookup"
```

---

### Task 10: Run all tests

**Step 1: Run full test suite**

Run: `cargo test`
Expected: PASS

**Step 2: Run clippy**

Run: `cargo clippy`
Expected: No warnings (fix any that appear)

**Step 3: Format code**

Run: `cargo fmt`

**Step 4: Final commit**

```bash
git add -A
git commit -m "chore: format and cleanup"
```

---

### Task 11: Manual testing

**Step 1: Test help output**

Run: `cargo run -- lookup --help`
Expected: Shows `--trust-source <TRUST_SOURCE>` option with valid values

**Step 2: Test invalid source**

Run: `cargo run -- lookup test.m4b --trust-source invalid`
Expected: Error about invalid value

**Step 3: Test with real file (if available)**

Run: `cargo run -- lookup /path/to/test.m4b --trust-source audible`
Expected: Either applies Audible data or skips with "trusted source 'audible' returned no results"
