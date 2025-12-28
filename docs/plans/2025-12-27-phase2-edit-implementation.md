# Phase 2: Edit Command Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement `audiobookctl edit` command for editing m4b metadata via $EDITOR with TOML format, diff preview, and safety-first defaults.

**Architecture:** Edit command opens metadata in $EDITOR as TOML, shows side-by-side diff, saves to pending cache (dry-run) or applies with backup (--no-dry-run). Pending edits stored in ~/.cache/audiobookctl/pending/ keyed by path hash.

**Tech Stack:** Rust, toml (serialization), dirs (XDG paths), sha2 (path hashing), walkdir (recursive search), chrono (timestamps)

---

## Task 1: Add New Dependencies

**Files:**
- Modify: `Cargo.toml`

**Step 1: Add dependencies**

Add to `[dependencies]` section in `Cargo.toml`:
```toml
toml = "0.8"
dirs = "5"
sha2 = "0.10"
walkdir = "2"
chrono = { version = "0.4", default-features = false, features = ["std", "clock"] }
```

**Step 2: Verify dependencies resolve**

Run: `cargo check`
Expected: Compiles successfully

**Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "feat: add dependencies for edit command"
```

---

## Task 2: Create Pending Edits Cache Module

**Files:**
- Create: `src/safety/mod.rs`
- Create: `src/safety/pending.rs`
- Modify: `src/main.rs` (add module declaration)

**Step 1: Create safety module structure**

Create `src/safety/mod.rs`:
```rust
pub mod pending;

pub use pending::{PendingEdit, PendingEditsCache};
```

**Step 2: Implement pending edits cache**

Create `src/safety/pending.rs`:
```rust
use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

/// Represents a pending edit waiting to be applied
#[derive(Debug)]
pub struct PendingEdit {
    pub original_path: PathBuf,
    pub toml_content: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Manages the pending edits cache directory
pub struct PendingEditsCache {
    cache_dir: PathBuf,
}

impl PendingEditsCache {
    /// Create a new cache, initializing the directory if needed
    pub fn new() -> Result<Self> {
        let cache_dir = dirs::cache_dir()
            .context("Could not determine cache directory")?
            .join("audiobookctl")
            .join("pending");

        fs::create_dir_all(&cache_dir)
            .with_context(|| format!("Failed to create cache directory: {}", cache_dir.display()))?;

        Ok(Self { cache_dir })
    }

    /// Get the cache file path for a given m4b file
    pub fn cache_path_for(&self, file_path: &Path) -> Result<PathBuf> {
        let abs_path = file_path.canonicalize()
            .with_context(|| format!("Failed to get absolute path for: {}", file_path.display()))?;

        let hash = Self::hash_path(&abs_path);
        Ok(self.cache_dir.join(format!("{}.toml", hash)))
    }

    /// Hash a path to a 16-char hex string
    fn hash_path(path: &Path) -> String {
        let mut hasher = Sha256::new();
        hasher.update(path.to_string_lossy().as_bytes());
        let result = hasher.finalize();
        hex::encode(&result[..8]) // First 8 bytes = 16 hex chars
    }

    /// Check if a pending edit exists for a file
    pub fn has_pending(&self, file_path: &Path) -> Result<bool> {
        let cache_path = self.cache_path_for(file_path)?;
        Ok(cache_path.exists())
    }

    /// Load a pending edit for a file
    pub fn load(&self, file_path: &Path) -> Result<Option<PendingEdit>> {
        let cache_path = self.cache_path_for(file_path)?;

        if !cache_path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&cache_path)
            .with_context(|| format!("Failed to read pending edit: {}", cache_path.display()))?;

        // Parse header comments for metadata
        let mut original_path = file_path.to_path_buf();
        let mut created_at = chrono::Utc::now();
        let mut toml_start = 0;

        for (i, line) in content.lines().enumerate() {
            if line.starts_with("# Pending edit for: ") {
                let path_str = line.trim_start_matches("# Pending edit for: ");
                original_path = PathBuf::from(path_str);
            } else if line.starts_with("# Created: ") {
                let ts_str = line.trim_start_matches("# Created: ");
                if let Ok(ts) = chrono::DateTime::parse_from_rfc3339(ts_str) {
                    created_at = ts.with_timezone(&chrono::Utc);
                }
            } else if !line.starts_with('#') && !line.is_empty() {
                toml_start = content.lines().take(i).map(|l| l.len() + 1).sum();
                break;
            }
        }

        let toml_content = content[toml_start..].to_string();

        Ok(Some(PendingEdit {
            original_path,
            toml_content,
            created_at,
        }))
    }

    /// Save a pending edit for a file
    pub fn save(&self, file_path: &Path, toml_content: &str) -> Result<PathBuf> {
        let cache_path = self.cache_path_for(file_path)?;
        let abs_path = file_path.canonicalize()?;
        let now = chrono::Utc::now();

        let header = format!(
            "# Pending edit for: {}\n# Created: {}\n# Run: audiobookctl edit \"{}\" --no-dry-run\n\n",
            abs_path.display(),
            now.to_rfc3339(),
            abs_path.display()
        );

        let full_content = format!("{}{}", header, toml_content);

        fs::write(&cache_path, full_content)
            .with_context(|| format!("Failed to write pending edit: {}", cache_path.display()))?;

        Ok(cache_path)
    }

    /// Clear pending edit for a specific file
    pub fn clear(&self, file_path: &Path) -> Result<bool> {
        let cache_path = self.cache_path_for(file_path)?;

        if cache_path.exists() {
            fs::remove_file(&cache_path)
                .with_context(|| format!("Failed to remove pending edit: {}", cache_path.display()))?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Clear all pending edits
    pub fn clear_all(&self) -> Result<usize> {
        let mut count = 0;

        if self.cache_dir.exists() {
            for entry in fs::read_dir(&self.cache_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().map_or(false, |e| e == "toml") {
                    fs::remove_file(&path)?;
                    count += 1;
                }
            }
        }

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_hash_path_consistent() {
        let hash1 = PendingEditsCache::hash_path(Path::new("/home/user/book.m4b"));
        let hash2 = PendingEditsCache::hash_path(Path::new("/home/user/book.m4b"));
        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 16);
    }

    #[test]
    fn test_hash_path_different() {
        let hash1 = PendingEditsCache::hash_path(Path::new("/home/user/book1.m4b"));
        let hash2 = PendingEditsCache::hash_path(Path::new("/home/user/book2.m4b"));
        assert_ne!(hash1, hash2);
    }
}
```

**Step 3: Add hex dependency for hash encoding**

Add to `Cargo.toml` dependencies:
```toml
hex = "0.4"
```

**Step 4: Add module to main.rs**

Add after other module declarations in `src/main.rs`:
```rust
mod safety;
```

**Step 5: Verify it compiles**

Run: `cargo check`
Expected: Compiles successfully

**Step 6: Run tests**

Run: `cargo test pending`
Expected: 2 tests pass

**Step 7: Commit**

```bash
git add src/safety/ Cargo.toml Cargo.lock
git commit -m "feat: add pending edits cache module"
```

---

## Task 3: Create Backup Module

**Files:**
- Create: `src/safety/backup.rs`
- Modify: `src/safety/mod.rs`

**Step 1: Implement backup module**

Create `src/safety/backup.rs`:
```rust
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Create a backup of a file before modifying it
pub fn create_backup(file_path: &Path) -> Result<PathBuf> {
    let backup_path = backup_path_for(file_path);

    fs::copy(file_path, &backup_path)
        .with_context(|| format!(
            "Failed to create backup: {} -> {}",
            file_path.display(),
            backup_path.display()
        ))?;

    Ok(backup_path)
}

/// Get the backup path for a file
pub fn backup_path_for(file_path: &Path) -> PathBuf {
    let mut backup = file_path.to_path_buf();
    let mut name = backup.file_name().unwrap_or_default().to_os_string();
    name.push(".bak");
    backup.set_file_name(name);
    backup
}

/// Check if a backup exists for a file
pub fn has_backup(file_path: &Path) -> bool {
    backup_path_for(file_path).exists()
}

/// Delete the backup for a specific file
pub fn delete_backup(file_path: &Path) -> Result<bool> {
    let backup_path = backup_path_for(file_path);

    if backup_path.exists() {
        fs::remove_file(&backup_path)
            .with_context(|| format!("Failed to delete backup: {}", backup_path.display()))?;
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Information about a backup file
#[derive(Debug)]
pub struct BackupInfo {
    pub backup_path: PathBuf,
    pub original_path: PathBuf,
    pub size_bytes: u64,
}

/// Find all backup files in a directory recursively
pub fn find_all_backups(dir: &Path) -> Result<Vec<BackupInfo>> {
    let mut backups = Vec::new();

    for entry in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();

        if path.extension().map_or(false, |e| e == "bak") {
            // Check if it's an m4b backup
            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            if stem.ends_with(".m4b") {
                let original = path.with_file_name(stem);
                let size = fs::metadata(path).map(|m| m.len()).unwrap_or(0);

                backups.push(BackupInfo {
                    backup_path: path.to_path_buf(),
                    original_path: original,
                    size_bytes: size,
                });
            }
        }
    }

    Ok(backups)
}

/// Format bytes as human-readable size
pub fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.0} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.0} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} bytes", bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backup_path_for() {
        let path = Path::new("/home/user/book.m4b");
        let backup = backup_path_for(path);
        assert_eq!(backup, PathBuf::from("/home/user/book.m4b.bak"));
    }

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(500), "500 bytes");
        assert_eq!(format_size(1024), "1 KB");
        assert_eq!(format_size(1024 * 1024), "1 MB");
        assert_eq!(format_size(1024 * 1024 * 1024), "1.0 GB");
        assert_eq!(format_size(523 * 1024 * 1024), "523 MB");
    }
}
```

**Step 2: Update safety/mod.rs**

Replace `src/safety/mod.rs`:
```rust
pub mod backup;
pub mod pending;

pub use backup::{create_backup, delete_backup, find_all_backups, has_backup, BackupInfo, format_size};
pub use pending::{PendingEdit, PendingEditsCache};
```

**Step 3: Verify and test**

Run: `cargo test backup`
Expected: 2 tests pass

**Step 4: Commit**

```bash
git add src/safety/
git commit -m "feat: add backup module for .bak file handling"
```

---

## Task 4: Create TOML Serialization Module

**Files:**
- Create: `src/editor/mod.rs`
- Create: `src/editor/toml.rs`
- Modify: `src/main.rs`

**Step 1: Create editor module structure**

Create `src/editor/mod.rs`:
```rust
pub mod toml;

pub use toml::{metadata_to_toml, toml_to_metadata};
```

**Step 2: Implement TOML serialization**

Create `src/editor/toml.rs`:
```rust
use crate::metadata::AudiobookMetadata;
use anyhow::{bail, Result};

/// Convert metadata to TOML string with comments for empty/read-only fields
pub fn metadata_to_toml(metadata: &AudiobookMetadata) -> String {
    let mut lines = Vec::new();

    lines.push("# Audiobook Metadata - Edit and save to apply changes".to_string());
    lines.push("# Commented fields are empty - uncomment and fill to add values".to_string());
    lines.push(String::new());

    // Helper to add field
    fn add_field(lines: &mut Vec<String>, name: &str, value: &Option<String>) {
        match value {
            Some(v) => lines.push(format!("{} = \"{}\"", name, escape_toml_string(v))),
            None => lines.push(format!("# {} = \"\"", name)),
        }
    }

    fn add_field_u32(lines: &mut Vec<String>, name: &str, value: &Option<u32>) {
        match value {
            Some(v) => lines.push(format!("{} = {}", name, v)),
            None => lines.push(format!("# {} = 0", name)),
        }
    }

    add_field(&mut lines, "title", &metadata.title);
    add_field(&mut lines, "author", &metadata.author);
    add_field(&mut lines, "narrator", &metadata.narrator);
    add_field(&mut lines, "series", &metadata.series);
    add_field_u32(&mut lines, "series_position", &metadata.series_position);
    add_field_u32(&mut lines, "year", &metadata.year);
    add_field(&mut lines, "description", &metadata.description);
    add_field(&mut lines, "publisher", &metadata.publisher);
    add_field(&mut lines, "genre", &metadata.genre);
    add_field(&mut lines, "isbn", &metadata.isbn);
    add_field(&mut lines, "asin", &metadata.asin);

    // Read-only section
    lines.push(String::new());
    lines.push("# Read-only (cannot be edited)".to_string());

    if let Some(duration) = metadata.duration_seconds {
        let hours = duration / 3600;
        let minutes = (duration % 3600) / 60;
        let seconds = duration % 60;
        lines.push(format!("# duration = \"{:02}:{:02}:{:02}\"", hours, minutes, seconds));
    } else {
        lines.push("# duration = \"\"".to_string());
    }

    if let Some(chapters) = metadata.chapter_count {
        lines.push(format!("# chapters = {}", chapters));
    } else {
        lines.push("# chapters = 0".to_string());
    }

    if let Some(ref cover) = metadata.cover_info {
        lines.push(format!("# cover = \"{}\"", cover));
    } else {
        lines.push("# cover = \"\"".to_string());
    }

    lines.push(String::new());
    lines.join("\n")
}

/// Parse TOML string back to metadata
pub fn toml_to_metadata(toml_str: &str) -> Result<AudiobookMetadata> {
    // Filter out comment lines and parse remaining as TOML
    let filtered: String = toml_str
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n");

    // Parse into a toml::Value first
    let value: toml::Value = toml::from_str(&filtered)?;
    let table = value.as_table().ok_or_else(|| anyhow::anyhow!("Invalid TOML structure"))?;

    fn get_string(table: &toml::map::Map<String, toml::Value>, key: &str) -> Option<String> {
        table.get(key).and_then(|v| v.as_str()).map(String::from)
    }

    fn get_u32(table: &toml::map::Map<String, toml::Value>, key: &str) -> Option<u32> {
        table.get(key).and_then(|v| v.as_integer()).map(|n| n as u32)
    }

    Ok(AudiobookMetadata {
        title: get_string(table, "title"),
        author: get_string(table, "author"),
        narrator: get_string(table, "narrator"),
        series: get_string(table, "series"),
        series_position: get_u32(table, "series_position"),
        year: get_u32(table, "year"),
        description: get_string(table, "description"),
        publisher: get_string(table, "publisher"),
        genre: get_string(table, "genre"),
        isbn: get_string(table, "isbn"),
        asin: get_string(table, "asin"),
        // Read-only fields preserved as None (will be kept from original when writing)
        duration_seconds: None,
        chapter_count: None,
        cover_info: None,
    })
}

/// Escape special characters in TOML strings
fn escape_toml_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata_to_toml_with_values() {
        let metadata = AudiobookMetadata {
            title: Some("Test Book".to_string()),
            author: Some("Test Author".to_string()),
            narrator: None,
            series: Some("Test Series".to_string()),
            series_position: Some(1),
            year: Some(2024),
            description: Some("A test description".to_string()),
            publisher: None,
            genre: Some("Fiction".to_string()),
            isbn: None,
            asin: None,
            duration_seconds: Some(3661),
            chapter_count: Some(10),
            cover_info: Some("embedded (1000 bytes, JPEG)".to_string()),
        };

        let toml = metadata_to_toml(&metadata);

        assert!(toml.contains("title = \"Test Book\""));
        assert!(toml.contains("author = \"Test Author\""));
        assert!(toml.contains("# narrator = \"\""));
        assert!(toml.contains("series = \"Test Series\""));
        assert!(toml.contains("series_position = 1"));
        assert!(toml.contains("# duration = \"01:01:01\""));
    }

    #[test]
    fn test_toml_to_metadata() {
        let toml = r#"
title = "Parsed Book"
author = "Parsed Author"
year = 2023
"#;

        let metadata = toml_to_metadata(toml).unwrap();

        assert_eq!(metadata.title, Some("Parsed Book".to_string()));
        assert_eq!(metadata.author, Some("Parsed Author".to_string()));
        assert_eq!(metadata.year, Some(2023));
        assert_eq!(metadata.narrator, None);
    }

    #[test]
    fn test_roundtrip() {
        let original = AudiobookMetadata {
            title: Some("Roundtrip Test".to_string()),
            author: Some("Test Author".to_string()),
            narrator: Some("Test Narrator".to_string()),
            series: None,
            series_position: None,
            year: Some(2024),
            description: Some("Description with \"quotes\"".to_string()),
            publisher: None,
            genre: None,
            isbn: Some("123-456".to_string()),
            asin: None,
            duration_seconds: None,
            chapter_count: None,
            cover_info: None,
        };

        let toml = metadata_to_toml(&original);
        let parsed = toml_to_metadata(&toml).unwrap();

        assert_eq!(parsed.title, original.title);
        assert_eq!(parsed.author, original.author);
        assert_eq!(parsed.narrator, original.narrator);
        assert_eq!(parsed.year, original.year);
        assert_eq!(parsed.isbn, original.isbn);
    }
}
```

**Step 3: Add module to main.rs**

Add after other module declarations:
```rust
mod editor;
```

**Step 4: Verify and test**

Run: `cargo test toml`
Expected: 3 tests pass

**Step 5: Commit**

```bash
git add src/editor/
git commit -m "feat: add TOML serialization for metadata editing"
```

---

## Task 5: Create Diff Display Module

**Files:**
- Create: `src/editor/diff.rs`
- Modify: `src/editor/mod.rs`

**Step 1: Implement diff display**

Create `src/editor/diff.rs`:
```rust
use crate::metadata::AudiobookMetadata;
use std::fmt::Write;

/// A single field change
#[derive(Debug, PartialEq)]
pub struct FieldChange {
    pub field: String,
    pub old_value: String,
    pub new_value: String,
}

/// Compute changes between two metadata structs
pub fn compute_changes(old: &AudiobookMetadata, new: &AudiobookMetadata) -> Vec<FieldChange> {
    let mut changes = Vec::new();

    fn check_string(changes: &mut Vec<FieldChange>, field: &str, old: &Option<String>, new: &Option<String>) {
        let old_val = old.as_deref().unwrap_or("");
        let new_val = new.as_deref().unwrap_or("");
        if old_val != new_val {
            changes.push(FieldChange {
                field: field.to_string(),
                old_value: if old_val.is_empty() { "(empty)".to_string() } else { old_val.to_string() },
                new_value: if new_val.is_empty() { "(empty)".to_string() } else { new_val.to_string() },
            });
        }
    }

    fn check_u32(changes: &mut Vec<FieldChange>, field: &str, old: &Option<u32>, new: &Option<u32>) {
        if old != new {
            changes.push(FieldChange {
                field: field.to_string(),
                old_value: old.map_or("(empty)".to_string(), |v| v.to_string()),
                new_value: new.map_or("(empty)".to_string(), |v| v.to_string()),
            });
        }
    }

    check_string(&mut changes, "title", &old.title, &new.title);
    check_string(&mut changes, "author", &old.author, &new.author);
    check_string(&mut changes, "narrator", &old.narrator, &new.narrator);
    check_string(&mut changes, "series", &old.series, &new.series);
    check_u32(&mut changes, "series_position", &old.series_position, &new.series_position);
    check_u32(&mut changes, "year", &old.year, &new.year);
    check_string(&mut changes, "description", &old.description, &new.description);
    check_string(&mut changes, "publisher", &old.publisher, &new.publisher);
    check_string(&mut changes, "genre", &old.genre, &new.genre);
    check_string(&mut changes, "isbn", &old.isbn, &new.isbn);
    check_string(&mut changes, "asin", &old.asin, &new.asin);

    changes
}

/// Format changes as a side-by-side diff table
pub fn format_diff(file_path: &str, changes: &[FieldChange]) -> String {
    if changes.is_empty() {
        return "No changes detected.".to_string();
    }

    let mut output = String::new();

    writeln!(output, "Changes to {}:", file_path).unwrap();
    writeln!(output).unwrap();

    // Calculate column widths
    let field_width = changes.iter().map(|c| c.field.len()).max().unwrap_or(10).max(10);
    let value_width = 24;

    // Header
    writeln!(
        output,
        "  {:width$} | {:vw$} | {:vw$}",
        "Field",
        "Current",
        "New",
        width = field_width,
        vw = value_width
    ).unwrap();

    // Separator
    writeln!(
        output,
        " {:->width$}-+-{:->vw$}-+-{:->vw$}",
        "",
        "",
        "",
        width = field_width + 1,
        vw = value_width
    ).unwrap();

    // Changes
    for change in changes {
        let old_display = truncate_value(&change.old_value, value_width);
        let new_display = truncate_value(&change.new_value, value_width);

        writeln!(
            output,
            "  {:width$} | {:vw$} | {:vw$}",
            change.field,
            old_display,
            new_display,
            width = field_width,
            vw = value_width
        ).unwrap();
    }

    output
}

/// Truncate a value to fit in the column width
fn truncate_value(value: &str, max_width: usize) -> String {
    // Replace newlines with spaces for display
    let single_line = value.replace('\n', " ");

    if single_line.len() <= max_width {
        single_line
    } else {
        format!("{}...", &single_line[..max_width - 3])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_changes_no_changes() {
        let old = AudiobookMetadata::default();
        let new = AudiobookMetadata::default();

        let changes = compute_changes(&old, &new);
        assert!(changes.is_empty());
    }

    #[test]
    fn test_compute_changes_with_changes() {
        let old = AudiobookMetadata {
            title: Some("Old Title".to_string()),
            author: Some("Author".to_string()),
            ..Default::default()
        };

        let new = AudiobookMetadata {
            title: Some("New Title".to_string()),
            author: Some("Author".to_string()),
            narrator: Some("New Narrator".to_string()),
            ..Default::default()
        };

        let changes = compute_changes(&old, &new);

        assert_eq!(changes.len(), 2);
        assert_eq!(changes[0].field, "title");
        assert_eq!(changes[0].old_value, "Old Title");
        assert_eq!(changes[0].new_value, "New Title");
        assert_eq!(changes[1].field, "narrator");
        assert_eq!(changes[1].old_value, "(empty)");
        assert_eq!(changes[1].new_value, "New Narrator");
    }

    #[test]
    fn test_format_diff_empty() {
        let output = format_diff("book.m4b", &[]);
        assert_eq!(output, "No changes detected.");
    }

    #[test]
    fn test_format_diff_with_changes() {
        let changes = vec![
            FieldChange {
                field: "title".to_string(),
                old_value: "Old".to_string(),
                new_value: "New".to_string(),
            },
        ];

        let output = format_diff("book.m4b", &changes);
        assert!(output.contains("Changes to book.m4b:"));
        assert!(output.contains("title"));
        assert!(output.contains("Old"));
        assert!(output.contains("New"));
    }
}
```

**Step 2: Update editor/mod.rs**

```rust
pub mod diff;
pub mod toml;

pub use diff::{compute_changes, format_diff, FieldChange};
pub use toml::{metadata_to_toml, toml_to_metadata};
```

**Step 3: Verify and test**

Run: `cargo test diff`
Expected: 4 tests pass

**Step 4: Commit**

```bash
git add src/editor/
git commit -m "feat: add side-by-side diff display for metadata changes"
```

---

## Task 6: Create Metadata Writer Module

**Files:**
- Create: `src/metadata/writer.rs`
- Modify: `src/metadata/mod.rs`

**Step 1: Implement metadata writer**

Create `src/metadata/writer.rs`:
```rust
use crate::metadata::AudiobookMetadata;
use anyhow::{Context, Result};
use std::path::Path;

/// Write metadata to an m4b file
pub fn write_metadata(path: &Path, metadata: &AudiobookMetadata) -> Result<()> {
    let mut tag = mp4ameta::Tag::read_from_path(path)
        .with_context(|| format!("Failed to read m4b file for writing: {}", path.display()))?;

    // Helper to set or remove string fields
    fn set_string<F, G>(value: &Option<String>, setter: F, remover: G)
    where
        F: FnOnce(&str),
        G: FnOnce(),
    {
        match value {
            Some(v) if !v.is_empty() => setter(v),
            _ => remover(),
        }
    }

    // Title
    if let Some(ref title) = metadata.title {
        tag.set_title(title);
    } else {
        tag.remove_title();
    }

    // Author (artist)
    if let Some(ref author) = metadata.author {
        tag.set_artist(author);
    } else {
        tag.remove_artist();
    }

    // Narrator (freeform iTunes atom)
    let narrator_ident = mp4ameta::FreeformIdent::new("com.apple.iTunes", "NARRATOR");
    if let Some(ref narrator) = metadata.narrator {
        tag.set_data(narrator_ident, mp4ameta::Data::Utf8(narrator.clone()));
    } else {
        tag.remove_data_of(&narrator_ident);
    }

    // Series (TV show name)
    if let Some(ref series) = metadata.series {
        tag.set_tv_show_name(series);
    } else {
        tag.remove_tv_show_name();
    }

    // Series position (TV episode)
    if let Some(pos) = metadata.series_position {
        tag.set_tv_episode(pos);
    } else {
        tag.remove_tv_episode();
    }

    // Year
    if let Some(year) = metadata.year {
        tag.set_year(year.to_string());
    } else {
        tag.remove_year();
    }

    // Description
    if let Some(ref desc) = metadata.description {
        tag.set_description(desc);
    } else {
        tag.remove_description();
    }

    // Genre
    if let Some(ref genre) = metadata.genre {
        tag.set_genre(genre);
    } else {
        tag.remove_genre();
    }

    // ISBN (freeform iTunes atom)
    let isbn_ident = mp4ameta::FreeformIdent::new("com.apple.iTunes", "ISBN");
    if let Some(ref isbn) = metadata.isbn {
        tag.set_data(isbn_ident, mp4ameta::Data::Utf8(isbn.clone()));
    } else {
        tag.remove_data_of(&isbn_ident);
    }

    // ASIN (freeform iTunes atom)
    let asin_ident = mp4ameta::FreeformIdent::new("com.apple.iTunes", "ASIN");
    if let Some(ref asin) = metadata.asin {
        tag.set_data(asin_ident, mp4ameta::Data::Utf8(asin.clone()));
    } else {
        tag.remove_data_of(&asin_ident);
    }

    // Note: We don't write duration, chapter_count, or cover_info as they are read-only
    // Publisher is also not written as mp4ameta doesn't support it directly

    tag.write_to_path(path)
        .with_context(|| format!("Failed to write metadata to: {}", path.display()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: Full integration tests require actual m4b files
    // These tests just verify the module compiles correctly

    #[test]
    fn test_write_to_nonexistent_fails() {
        let metadata = AudiobookMetadata::default();
        let result = write_metadata(Path::new("/nonexistent/file.m4b"), &metadata);
        assert!(result.is_err());
    }
}
```

**Step 2: Update metadata/mod.rs**

```rust
#![allow(dead_code, unused_imports)]

mod fields;
mod reader;
mod writer;

pub use fields::AudiobookMetadata;
pub use reader::read_metadata;
pub use writer::write_metadata;
```

**Step 3: Verify and test**

Run: `cargo test writer`
Expected: 1 test passes

**Step 4: Commit**

```bash
git add src/metadata/
git commit -m "feat: add metadata writer module"
```

---

## Task 7: Update CLI with Edit Command

**Files:**
- Modify: `src/cli.rs`

**Step 1: Add Edit command to CLI**

Replace `src/cli.rs`:
```rust
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "audiobookctl")]
#[command(about = "CLI tool for reading, editing, and organizing m4b audiobook metadata")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Increase output verbosity
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Suppress non-essential output
    #[arg(short, long, global = true)]
    pub quiet: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Display metadata for an m4b file
    Show {
        /// Path to the m4b file
        file: PathBuf,

        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// Show only a specific field
        #[arg(long)]
        field: Option<String>,
    },

    /// Edit metadata in $EDITOR with diff preview
    Edit {
        /// Path to the m4b file
        file: Option<PathBuf>,

        /// Actually apply changes (default: dry-run)
        #[arg(long)]
        no_dry_run: bool,

        /// Skip confirmation prompt
        #[arg(long)]
        yes: bool,

        /// Skip creating backup file
        #[arg(long = "no-backup-i-void-my-warranty")]
        no_backup: bool,

        /// Clear pending edit(s)
        #[arg(long)]
        clear: bool,

        /// Delete backup after verifying changes
        #[arg(long)]
        commit: bool,

        /// Delete all backup files recursively
        #[arg(long)]
        commit_all: bool,
    },
}
```

**Step 2: Verify it compiles**

Run: `cargo check`
Expected: Compiles (with warning about unused Edit variant)

**Step 3: Commit**

```bash
git add src/cli.rs
git commit -m "feat: add Edit command to CLI"
```

---

## Task 8: Implement Edit Command

**Files:**
- Create: `src/commands/edit.rs`
- Modify: `src/commands/mod.rs`
- Modify: `src/main.rs`

**Step 1: Implement edit command**

Create `src/commands/edit.rs`:
```rust
use crate::editor::{compute_changes, format_diff, metadata_to_toml, toml_to_metadata};
use crate::metadata::{read_metadata, write_metadata};
use crate::safety::{create_backup, delete_backup, find_all_backups, format_size, has_backup, PendingEditsCache};
use anyhow::{bail, Context, Result};
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;

pub fn run(
    file: Option<&Path>,
    no_dry_run: bool,
    yes: bool,
    no_backup: bool,
    clear: bool,
    commit: bool,
    commit_all: bool,
) -> Result<()> {
    // Handle --clear (no file needed for --clear without file)
    if clear {
        return handle_clear(file);
    }

    // Handle --commit-all (no file needed)
    if commit_all {
        return handle_commit_all();
    }

    // All other operations require a file
    let file = file.ok_or_else(|| anyhow::anyhow!("No file specified. Use: audiobookctl edit <file>"))?;

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
    let new_metadata = toml_to_metadata(&edited_toml)
        .context("Failed to parse edited TOML")?;

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
        let cache_path = cache.save(file, &edited_toml)?;
        println!();
        println!("Changes saved to pending cache.");
        println!("To apply: audiobookctl edit \"{}\" --no-dry-run", file.display());
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

    std::fs::write(&temp_path, content)
        .context("Failed to create temp file for editing")?;

    // Open editor
    let status = Command::new(&editor)
        .arg(&temp_path)
        .status()
        .with_context(|| format!("Failed to open editor: {}", editor))?;

    if !status.success() {
        bail!("Editor exited with error");
    }

    // Read back
    let edited = std::fs::read_to_string(&temp_path)
        .context("Failed to read edited file")?;

    // Clean up
    let _ = std::fs::remove_file(&temp_path);

    Ok(edited)
}

fn apply_changes(
    file: &Path,
    new_metadata: &crate::metadata::AudiobookMetadata,
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

fn handle_clear(file: Option<&Path>) -> Result<()> {
    let cache = PendingEditsCache::new()?;

    if let Some(file) = file {
        if cache.clear(file)? {
            println!("Cleared pending edit for: {}", file.display());
        } else {
            println!("No pending edit found for: {}", file.display());
        }
    } else {
        let count = cache.clear_all()?;
        println!("Cleared {} pending edit(s).", count);
    }

    Ok(())
}

fn handle_commit(file: &Path) -> Result<()> {
    if !has_backup(file) {
        bail!("No backup found for: {}", file.display());
    }

    let backup_path = crate::safety::backup::backup_path_for(file);
    let size = std::fs::metadata(&backup_path)
        .map(|m| m.len())
        .unwrap_or(0);

    print!("Delete backup {} ({})? [y/N] ", backup_path.display(), format_size(size));
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

    println!("Found {} backup files ({} total):", backups.len(), format_size(total_size));
    for backup in &backups {
        println!("  {} ({})", backup.backup_path.display(), format_size(backup.size_bytes));
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
```

**Step 2: Update commands/mod.rs**

```rust
pub mod edit;
pub mod show;
```

**Step 3: Update main.rs**

Replace `src/main.rs`:
```rust
mod cli;
mod commands;
mod editor;
mod metadata;
mod safety;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Show { file, json, field } => {
            commands::show::run(&file, json, field.as_deref(), cli.quiet)?;
        }
        Commands::Edit {
            file,
            no_dry_run,
            yes,
            no_backup,
            clear,
            commit,
            commit_all,
        } => {
            commands::edit::run(
                file.as_deref(),
                no_dry_run,
                yes,
                no_backup,
                clear,
                commit,
                commit_all,
            )?;
        }
    }

    Ok(())
}
```

**Step 4: Verify it compiles**

Run: `cargo check`
Expected: Compiles successfully

**Step 5: Commit**

```bash
git add src/
git commit -m "feat: implement edit command"
```

---

## Task 9: Add Integration Tests

**Files:**
- Modify: `tests/cli_tests.rs`

**Step 1: Add edit command tests**

Add to `tests/cli_tests.rs`:
```rust
#[test]
fn test_edit_help() {
    let mut cmd = Command::cargo_bin("audiobookctl").unwrap();
    cmd.args(["edit", "--help"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Edit metadata"))
        .stdout(predicate::str::contains("--no-dry-run"))
        .stdout(predicate::str::contains("--no-backup-i-void-my-warranty"));
}

#[test]
fn test_edit_missing_file() {
    let mut cmd = Command::cargo_bin("audiobookctl").unwrap();
    cmd.args(["edit", "/nonexistent/file.m4b"]);
    cmd.assert()
        .failure();
}

#[test]
fn test_edit_clear_no_file() {
    let mut cmd = Command::cargo_bin("audiobookctl").unwrap();
    cmd.args(["edit", "--clear"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Cleared"));
}

#[test]
fn test_edit_commit_all_no_backups() {
    let mut cmd = Command::cargo_bin("audiobookctl").unwrap();
    cmd.args(["edit", "--commit-all"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("No backup files found"));
}
```

**Step 2: Run tests**

Run: `cargo test`
Expected: All tests pass (9 total: 5 original + 4 new)

**Step 3: Commit**

```bash
git add tests/
git commit -m "test: add edit command integration tests"
```

---

## Task 10: Run Full CI Checks

**Step 1: Format check**

Run: `cargo fmt --check`
If issues: `cargo fmt`

**Step 2: Clippy check**

Run: `cargo clippy -- -D warnings`
Fix any warnings

**Step 3: Run all tests**

Run: `cargo test`
Expected: All tests pass

**Step 4: Commit any fixes**

```bash
cargo fmt
git add -u
git commit -m "style: apply rustfmt and clippy fixes"
```

---

## Task 11: Update Beads Issue

**Step 1: Close Phase 2 issue**

Run: `bd close audiobookctl-8vh -r "Phase 2 complete: edit command with TOML editing, diff preview, pending cache, and backup safety"`

**Step 2: Commit beads update**

```bash
git add .beads/
git commit -m "chore: close Phase 2 issue"
```

---

## Summary

After completing all tasks, you will have:
- `audiobookctl edit file.m4b` - Edit in $EDITOR, show diff, save to pending
- `audiobookctl edit file.m4b --no-dry-run` - Apply pending/new changes with backup
- `audiobookctl edit file.m4b --no-dry-run --yes` - Apply without confirmation
- `audiobookctl edit --clear` - Clear all pending edits
- `audiobookctl edit file.m4b --commit` - Delete backup for verified file
- `audiobookctl edit --commit-all` - Delete all backups in directory
- Full test coverage
- Passing CI (fmt, clippy, test)
