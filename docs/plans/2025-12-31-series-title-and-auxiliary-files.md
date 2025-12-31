# Series Title Field and Auxiliary Files Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add `{series_title}` template field and move auxiliary files (.cue, .pdf) with m4b during organize/fix.

**Architecture:** Two independent features: (1) new template field in format.rs renders "{position} - {title}" when series exists, (2) scanner discovers auxiliary files in m4b directories, planner creates operations for them, commands execute moves preserving relative structure.

**Tech Stack:** Rust, walkdir (already used), std::fs for file operations.

---

## Task 1: Add `{series_title}` to PLACEHOLDERS

**Files:**
- Modify: `src/organize/format.rs:13-28` (PLACEHOLDERS constant)

**Step 1: Add series_title to PLACEHOLDERS array**

In `src/organize/format.rs`, add the new placeholder to the PLACEHOLDERS array:

```rust
pub const PLACEHOLDERS: &[(&str, &str)] = &[
    ("author", "Author name"),
    ("title", "Book title"),
    ("series", "Series name"),
    (
        "series_position",
        "Position in series (supports :02 padding)",
    ),
    (
        "series_title",
        "Series position + title (e.g., '01 - Book Name')",
    ),
    ("narrator", "Narrator name"),
    ("year", "Publication year"),
    ("genre", "Genre"),
    ("publisher", "Publisher"),
    ("asin", "Amazon ASIN"),
    ("isbn", "ISBN"),
    ("filename", "Original filename"),
];
```

**Step 2: Run existing tests to verify no regression**

Run: `cargo test --lib format`
Expected: All existing tests PASS

**Step 3: Commit**

```bash
git add src/organize/format.rs
git commit -m "feat(format): add series_title placeholder to PLACEHOLDERS list"
```

---

## Task 2: Implement series_title in get_field_value

**Files:**
- Modify: `src/organize/format.rs:211-231` (get_field_value function)

**Step 1: Write the failing test**

Add to the tests module in `src/organize/format.rs`:

```rust
#[test]
fn test_series_title_with_position() {
    let template = FormatTemplate::parse("{author}/{series_title}/{filename}").unwrap();
    let metadata = AudiobookMetadata {
        title: Some("The Final Empire".to_string()),
        author: Some("Brandon Sanderson".to_string()),
        series: Some("Mistborn".to_string()),
        series_position: Some(1),
        ..Default::default()
    };
    let path = template.generate_path(&metadata, "book.m4b").unwrap();
    assert_eq!(path, PathBuf::from("Brandon Sanderson/01 - The Final Empire/book.m4b"));
}

#[test]
fn test_series_title_without_position() {
    let template = FormatTemplate::parse("{author}/{series_title}/{filename}").unwrap();
    let metadata = AudiobookMetadata {
        title: Some("Standalone Book".to_string()),
        author: Some("Author".to_string()),
        series: None,
        series_position: None,
        ..Default::default()
    };
    let path = template.generate_path(&metadata, "book.m4b").unwrap();
    assert_eq!(path, PathBuf::from("Author/Standalone Book/book.m4b"));
}

#[test]
fn test_series_title_missing_title() {
    let template = FormatTemplate::parse("{author}/{series_title}/{filename}").unwrap();
    let metadata = AudiobookMetadata {
        title: None,
        author: Some("Author".to_string()),
        series_position: Some(1),
        ..Default::default()
    };
    let result = template.generate_path(&metadata, "book.m4b");
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), vec!["series_title"]);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --lib format::tests::test_series_title`
Expected: FAIL - series_title returns None

**Step 3: Implement series_title in get_field_value**

Modify `get_field_value` in `src/organize/format.rs`:

```rust
fn get_field_value(
    &self,
    metadata: &AudiobookMetadata,
    name: &str,
    original_filename: &str,
) -> Option<String> {
    match name {
        "author" => metadata.author.clone(),
        "title" => metadata.title.clone(),
        "series" => metadata.series.clone(),
        "series_position" => metadata.series_position.map(|n| n.to_string()),
        "series_title" => {
            let title = metadata.title.as_ref()?;
            match metadata.series_position {
                Some(pos) => Some(format!("{:02} - {}", pos, title)),
                None => Some(title.clone()),
            }
        }
        "narrator" => metadata.narrator.clone(),
        "year" => metadata.year.map(|n| n.to_string()),
        "genre" => metadata.genre.clone(),
        "publisher" => metadata.publisher.clone(),
        "asin" => metadata.asin.clone(),
        "isbn" => metadata.isbn.clone(),
        "filename" => Some(original_filename.to_string()),
        _ => None,
    }
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test --lib format`
Expected: All tests PASS

**Step 5: Commit**

```bash
git add src/organize/format.rs
git commit -m "feat(format): implement series_title field logic"
```

---

## Task 3: Add AuxiliaryFile struct to scanner

**Files:**
- Modify: `src/organize/scanner.rs:1-13` (add struct)
- Modify: `src/organize/scanner.rs:8-13` (update ScannedFile)

**Step 1: Add AuxiliaryFile struct and update ScannedFile**

Add after the imports in `src/organize/scanner.rs`:

```rust
/// Auxiliary file discovered alongside an m4b (e.g., .cue, .pdf)
#[derive(Debug, Clone)]
pub struct AuxiliaryFile {
    /// Absolute path on disk
    pub path: PathBuf,
    /// Path relative to the m4b's parent directory
    pub relative_path: PathBuf,
}

/// Extensions recognized as auxiliary files
const AUXILIARY_EXTENSIONS: &[&str] = &["cue", "pdf"];
```

Update `ScannedFile` struct:

```rust
/// Information about a scanned audiobook file
#[derive(Debug, Clone)]
pub struct ScannedFile {
    pub path: PathBuf,
    pub filename: String,
    pub metadata: AudiobookMetadata,
    /// Auxiliary files found in the same directory tree
    pub auxiliary_files: Vec<AuxiliaryFile>,
}
```

**Step 2: Run cargo check to verify it compiles**

Run: `cargo check`
Expected: Errors about missing `auxiliary_files` field in scanner and test code

**Step 3: Commit**

```bash
git add src/organize/scanner.rs
git commit -m "feat(scanner): add AuxiliaryFile struct and update ScannedFile"
```

---

## Task 4: Implement auxiliary file discovery

**Files:**
- Modify: `src/organize/scanner.rs:16-48` (scan_directory function)

**Step 1: Write the failing test**

Add to the tests module in `src/organize/scanner.rs`:

```rust
#[test]
fn test_is_auxiliary_file() {
    assert!(is_auxiliary_file(Path::new("/path/to/book.cue")));
    assert!(is_auxiliary_file(Path::new("/path/to/notes.pdf")));
    assert!(is_auxiliary_file(Path::new("/path/to/NOTES.PDF")));
    assert!(!is_auxiliary_file(Path::new("/path/to/book.m4b")));
    assert!(!is_auxiliary_file(Path::new("/path/to/book.mp3")));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --lib scanner::tests::test_is_auxiliary`
Expected: FAIL - function doesn't exist

**Step 3: Add is_auxiliary_file helper function**

Add after `is_m4b_file` in `src/organize/scanner.rs`:

```rust
/// Check if a path is an auxiliary file
fn is_auxiliary_file(path: &Path) -> bool {
    path.extension()
        .map(|ext| {
            let ext_lower = ext.to_string_lossy().to_lowercase();
            AUXILIARY_EXTENSIONS.contains(&ext_lower.as_str())
        })
        .unwrap_or(false)
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test --lib scanner::tests::test_is_auxiliary`
Expected: PASS

**Step 5: Commit**

```bash
git add src/organize/scanner.rs
git commit -m "feat(scanner): add is_auxiliary_file helper"
```

---

## Task 5: Add scan_auxiliary_files function

**Files:**
- Modify: `src/organize/scanner.rs` (add function)

**Step 1: Implement scan_auxiliary_files**

Add function to `src/organize/scanner.rs`:

```rust
/// Scan for auxiliary files in a directory and its subdirectories
fn scan_auxiliary_files(m4b_dir: &Path) -> Vec<AuxiliaryFile> {
    let mut auxiliary = Vec::new();

    for entry in WalkDir::new(m4b_dir)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();

        if path.is_file() && is_auxiliary_file(path) {
            if let Ok(relative) = path.strip_prefix(m4b_dir) {
                auxiliary.push(AuxiliaryFile {
                    path: path.to_path_buf(),
                    relative_path: relative.to_path_buf(),
                });
            }
        }
    }

    // Sort for consistent output
    auxiliary.sort_by(|a, b| a.path.cmp(&b.path));
    auxiliary
}
```

**Step 2: Run cargo check**

Run: `cargo check`
Expected: Compiles (function exists but not yet used)

**Step 3: Commit**

```bash
git add src/organize/scanner.rs
git commit -m "feat(scanner): add scan_auxiliary_files function"
```

---

## Task 6: Integrate auxiliary scanning into scan_directory

**Files:**
- Modify: `src/organize/scanner.rs:16-48` (scan_directory function)

**Step 1: Update scan_directory to discover auxiliary files**

Replace the `scan_directory` function:

```rust
/// Recursively scan a directory for .m4b files and read their metadata
pub fn scan_directory(dir: &Path) -> Result<Vec<ScannedFile>> {
    let mut files = Vec::new();

    for entry in WalkDir::new(dir)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();

        // Only process .m4b files
        if path.is_file() && is_m4b_file(path) {
            let metadata = read_metadata(path)
                .with_context(|| format!("Failed to read metadata from {:?}", path))?;

            let filename = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();

            // Scan for auxiliary files in the m4b's parent directory
            let auxiliary_files = path
                .parent()
                .map(scan_auxiliary_files)
                .unwrap_or_default();

            files.push(ScannedFile {
                path: path.to_path_buf(),
                filename,
                metadata,
                auxiliary_files,
            });
        }
    }

    // Sort by path for consistent output
    files.sort_by(|a, b| a.path.cmp(&b.path));

    Ok(files)
}
```

**Step 2: Run cargo check**

Run: `cargo check`
Expected: Errors in planner.rs tests (need auxiliary_files)

**Step 3: Commit**

```bash
git add src/organize/scanner.rs
git commit -m "feat(scanner): integrate auxiliary file discovery into scan_directory"
```

---

## Task 7: Export AuxiliaryFile from organize module

**Files:**
- Modify: `src/organize/mod.rs:8` (add export)

**Step 1: Update mod.rs exports**

```rust
pub use scanner::{scan_directory, AuxiliaryFile, ScannedFile};
```

**Step 2: Run cargo check**

Run: `cargo check`
Expected: Errors about ScannedFile not having auxiliary_files in tests

**Step 3: Commit**

```bash
git add src/organize/mod.rs
git commit -m "feat(organize): export AuxiliaryFile and ScannedFile from scanner"
```

---

## Task 8: Add AuxiliaryOperation to planner

**Files:**
- Modify: `src/organize/planner.rs:1-28` (add struct, update PlannedOperation)

**Step 1: Add AuxiliaryOperation struct**

Add after the imports in `src/organize/planner.rs`:

```rust
use super::scanner::AuxiliaryFile;

/// A planned auxiliary file operation
#[derive(Debug, Clone)]
pub struct AuxiliaryOperation {
    pub source: PathBuf,
    pub dest: PathBuf,
}
```

**Step 2: Update PlannedOperation struct**

```rust
/// A planned file operation (copy or move)
#[derive(Debug, Clone)]
pub struct PlannedOperation {
    pub source: PathBuf,
    pub dest: PathBuf,
    /// Auxiliary files to copy/move with this m4b
    pub auxiliary: Vec<AuxiliaryOperation>,
}
```

**Step 3: Run cargo check**

Run: `cargo check`
Expected: Errors about missing `auxiliary` field in planner builds

**Step 4: Commit**

```bash
git add src/organize/planner.rs
git commit -m "feat(planner): add AuxiliaryOperation struct"
```

---

## Task 9: Update OrganizePlan::build for auxiliary files

**Files:**
- Modify: `src/organize/planner.rs:43-68` (OrganizePlan::build)

**Step 1: Update build function to include auxiliary operations**

Replace the OrganizePlan::build function:

```rust
/// Build a plan for organizing files
pub fn build(files: &[ScannedFile], template: &FormatTemplate, dest_dir: &Path) -> Self {
    let mut operations = Vec::new();
    let mut uncategorized = Vec::new();
    let mut dest_to_sources: HashMap<PathBuf, Vec<PathBuf>> = HashMap::new();

    for file in files {
        match template.generate_path(&file.metadata, &file.filename) {
            Ok(relative_path) => {
                let dest = dest_dir.join(&relative_path);
                let dest_parent = dest.parent().unwrap_or(dest_dir);

                // Build auxiliary operations preserving relative structure
                let auxiliary: Vec<AuxiliaryOperation> = file
                    .auxiliary_files
                    .iter()
                    .map(|aux| AuxiliaryOperation {
                        source: aux.path.clone(),
                        dest: dest_parent.join(&aux.relative_path),
                    })
                    .collect();

                operations.push(PlannedOperation {
                    source: file.path.clone(),
                    dest: dest.clone(),
                    auxiliary,
                });
                dest_to_sources
                    .entry(dest)
                    .or_default()
                    .push(file.path.clone());
            }
            Err(missing) => {
                uncategorized.push(UncategorizedFile {
                    source: file.path.clone(),
                    missing_fields: missing,
                });
            }
        }
    }

    // Detect conflicts
    let mut conflicts = Vec::new();

    for (dest, sources) in dest_to_sources {
        let exists_on_disk = dest.exists();

        if sources.len() > 1 || exists_on_disk {
            conflicts.push(Conflict {
                dest,
                sources,
                exists_on_disk,
            });
        }
    }

    // Sort for consistent output
    operations.sort_by(|a, b| a.source.cmp(&b.source));
    uncategorized.sort_by(|a, b| a.source.cmp(&b.source));
    conflicts.sort_by(|a, b| a.dest.cmp(&b.dest));

    Self {
        operations,
        uncategorized,
        conflicts,
    }
}
```

**Step 2: Run cargo check**

Run: `cargo check`
Expected: Errors in FixPlan::build and tests

**Step 3: Commit**

```bash
git add src/organize/planner.rs
git commit -m "feat(planner): update OrganizePlan::build for auxiliary files"
```

---

## Task 10: Update FixPlan::build for auxiliary files

**Files:**
- Modify: `src/organize/planner.rs:111-175` (FixPlan::build)

**Step 1: Update FixPlan::build**

Replace the FixPlan::build function:

```rust
/// Build a plan for fixing non-compliant files in an organized library
pub fn build(files: &[ScannedFile], template: &FormatTemplate, dest_dir: &Path) -> Self {
    let mut needs_fix = Vec::new();
    let mut compliant = Vec::new();
    let mut uncategorized = Vec::new();
    let mut dest_to_sources: HashMap<PathBuf, Vec<PathBuf>> = HashMap::new();

    for file in files {
        match template.generate_path(&file.metadata, &file.filename) {
            Ok(relative_path) => {
                let expected_dest = dest_dir.join(&relative_path);
                let dest_parent = expected_dest.parent().unwrap_or(dest_dir);

                // Check if file is already at the correct location
                if file.path == expected_dest {
                    compliant.push(file.path.clone());
                } else {
                    // Build auxiliary operations preserving relative structure
                    let auxiliary: Vec<AuxiliaryOperation> = file
                        .auxiliary_files
                        .iter()
                        .map(|aux| AuxiliaryOperation {
                            source: aux.path.clone(),
                            dest: dest_parent.join(&aux.relative_path),
                        })
                        .collect();

                    needs_fix.push(PlannedOperation {
                        source: file.path.clone(),
                        dest: expected_dest.clone(),
                        auxiliary,
                    });
                    dest_to_sources
                        .entry(expected_dest)
                        .or_default()
                        .push(file.path.clone());
                }
            }
            Err(missing) => {
                uncategorized.push(UncategorizedFile {
                    source: file.path.clone(),
                    missing_fields: missing,
                });
            }
        }
    }

    // Detect conflicts (only for files that need fixing)
    let mut conflicts = Vec::new();

    for (dest, sources) in dest_to_sources {
        // For fix, also check if dest already exists (and isn't one of the sources)
        let exists_on_disk = dest.exists() && !sources.contains(&dest);

        if sources.len() > 1 || exists_on_disk {
            conflicts.push(Conflict {
                dest,
                sources,
                exists_on_disk,
            });
        }
    }

    // Sort for consistent output
    needs_fix.sort_by(|a, b| a.source.cmp(&b.source));
    compliant.sort();
    uncategorized.sort_by(|a, b| a.source.cmp(&b.source));
    conflicts.sort_by(|a, b| a.dest.cmp(&b.dest));

    Self {
        needs_fix,
        compliant,
        uncategorized,
        conflicts,
    }
}
```

**Step 2: Run cargo check**

Run: `cargo check`
Expected: Errors in planner tests

**Step 3: Commit**

```bash
git add src/organize/planner.rs
git commit -m "feat(planner): update FixPlan::build for auxiliary files"
```

---

## Task 11: Fix planner tests

**Files:**
- Modify: `src/organize/planner.rs:178-238` (tests module)

**Step 1: Update make_scanned_file helper**

```rust
fn make_scanned_file(path: &str, author: &str, title: &str) -> ScannedFile {
    ScannedFile {
        path: PathBuf::from(path),
        filename: PathBuf::from(path)
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string(),
        metadata: AudiobookMetadata {
            author: Some(author.to_string()),
            title: Some(title.to_string()),
            ..Default::default()
        },
        auxiliary_files: Vec::new(),
    }
}
```

**Step 2: Update test_detect_missing_metadata test**

```rust
#[test]
fn test_detect_missing_metadata() {
    let files = vec![ScannedFile {
        path: PathBuf::from("/source/book.m4b"),
        filename: "book.m4b".to_string(),
        metadata: AudiobookMetadata {
            author: None,
            title: Some("Title".to_string()),
            ..Default::default()
        },
        auxiliary_files: Vec::new(),
    }];

    let template = FormatTemplate::parse("{author}/{title}/{filename}").unwrap();
    let plan = OrganizePlan::build(&files, &template, Path::new("/dest"));

    assert!(plan.operations.is_empty());
    assert_eq!(plan.uncategorized.len(), 1);
    assert_eq!(plan.uncategorized[0].missing_fields, vec!["author"]);
}
```

**Step 3: Run tests to verify they pass**

Run: `cargo test --lib planner`
Expected: All tests PASS

**Step 4: Commit**

```bash
git add src/organize/planner.rs
git commit -m "test(planner): fix tests for auxiliary_files field"
```

---

## Task 12: Export AuxiliaryOperation from organize module

**Files:**
- Modify: `src/organize/mod.rs:7` (update export)

**Step 1: Update mod.rs exports**

```rust
pub use planner::{AuxiliaryOperation, Conflict, FixPlan, OrganizePlan, PlannedOperation, UncategorizedFile};
```

**Step 2: Run cargo check**

Run: `cargo check`
Expected: Errors in tree.rs tests (PlannedOperation needs auxiliary)

**Step 3: Commit**

```bash
git add src/organize/mod.rs
git commit -m "feat(organize): export AuxiliaryOperation"
```

---

## Task 13: Fix tree.rs tests

**Files:**
- Modify: `src/organize/tree.rs:111-150` (tests module)

**Step 1: Update test fixtures to include auxiliary field**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_tree() {
        let operations = vec![
            PlannedOperation {
                source: PathBuf::from("/source/book1.m4b"),
                dest: PathBuf::from("/dest/Author A/Title 1/book1.m4b"),
                auxiliary: Vec::new(),
            },
            PlannedOperation {
                source: PathBuf::from("/source/book2.m4b"),
                dest: PathBuf::from("/dest/Author A/Title 2/book2.m4b"),
                auxiliary: Vec::new(),
            },
            PlannedOperation {
                source: PathBuf::from("/source/book3.m4b"),
                dest: PathBuf::from("/dest/Author B/Title 3/book3.m4b"),
                auxiliary: Vec::new(),
            },
        ];

        let tree = render_tree(&operations, Path::new("/dest"));

        assert!(tree.contains("Author A/"));
        assert!(tree.contains("Author B/"));
        assert!(tree.contains("Title 1/"));
        assert!(tree.contains("book1.m4b"));
    }

    #[test]
    fn test_render_list() {
        let operations = vec![PlannedOperation {
            source: PathBuf::from("/source/book.m4b"),
            dest: PathBuf::from("/dest/Author/Title/book.m4b"),
            auxiliary: Vec::new(),
        }];

        let list = render_list(&operations);
        assert!(list.contains("/source/book.m4b → /dest/Author/Title/book.m4b"));
    }
}
```

**Step 2: Run tests**

Run: `cargo test --lib tree`
Expected: All tests PASS

**Step 3: Commit**

```bash
git add src/organize/tree.rs
git commit -m "test(tree): fix tests for PlannedOperation auxiliary field"
```

---

## Task 14: Update render_list to show auxiliary files

**Files:**
- Modify: `src/organize/tree.rs:75-88` (render_list function)

**Step 1: Update render_list to display auxiliary files**

```rust
/// Render a list view of planned operations (source → dest pairs)
pub fn render_list(operations: &[PlannedOperation]) -> String {
    let mut output = String::new();

    for op in operations {
        output.push_str(&format!(
            "{} → {}\n",
            op.source.display(),
            op.dest.display()
        ));

        // Show auxiliary files indented under the m4b
        for aux in &op.auxiliary {
            output.push_str(&format!(
                "  + {} → {}\n",
                aux.source.file_name().unwrap_or_default().to_string_lossy(),
                aux.dest.file_name().unwrap_or_default().to_string_lossy()
            ));
        }
    }

    output
}
```

**Step 2: Add test for auxiliary display**

```rust
#[test]
fn test_render_list_with_auxiliary() {
    use crate::organize::AuxiliaryOperation;

    let operations = vec![PlannedOperation {
        source: PathBuf::from("/source/book.m4b"),
        dest: PathBuf::from("/dest/Author/Title/book.m4b"),
        auxiliary: vec![
            AuxiliaryOperation {
                source: PathBuf::from("/source/book.cue"),
                dest: PathBuf::from("/dest/Author/Title/book.cue"),
            },
            AuxiliaryOperation {
                source: PathBuf::from("/source/notes.pdf"),
                dest: PathBuf::from("/dest/Author/Title/notes.pdf"),
            },
        ],
    }];

    let list = render_list(&operations);
    assert!(list.contains("/source/book.m4b → /dest/Author/Title/book.m4b"));
    assert!(list.contains("  + book.cue → book.cue"));
    assert!(list.contains("  + notes.pdf → notes.pdf"));
}
```

**Step 3: Run tests**

Run: `cargo test --lib tree`
Expected: All tests PASS

**Step 4: Commit**

```bash
git add src/organize/tree.rs
git commit -m "feat(tree): display auxiliary files in list view"
```

---

## Task 15: Update organize command to copy auxiliary files

**Files:**
- Modify: `src/commands/organize.rs:236-290` (execute_plan function)

**Step 1: Update execute_plan to handle auxiliary files**

```rust
fn execute_plan(
    operations: &[PlannedOperation],
    uncategorized: &[UncategorizedFile],
    dest: &Path,
    allow_uncategorized: bool,
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

        // Copy m4b file
        std::fs::copy(&op.source, &op.dest)
            .with_context(|| format!("Failed to copy {:?} to {:?}", op.source, op.dest))?;

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
        println!(
            "{} {} file(s) copied.",
            "Done!".green().bold(),
            total_m4b
        );
    }

    Ok(())
}
```

**Step 2: Run cargo check**

Run: `cargo check`
Expected: Compiles

**Step 3: Commit**

```bash
git add src/commands/organize.rs
git commit -m "feat(organize): copy auxiliary files during execution"
```

---

## Task 16: Update fix command to move auxiliary files

**Files:**
- Modify: `src/commands/fix.rs:148-177` (execute_fix function)

**Step 1: Update execute_fix to handle auxiliary files**

```rust
fn execute_fix(plan: &FixPlan) -> Result<()> {
    println!();
    println!("{}", "Moving files...".green());

    let mut aux_count = 0;

    for op in &plan.needs_fix {
        // Create parent directories
        if let Some(parent) = op.dest.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory {:?}", parent))?;
        }

        // Move m4b file (rename)
        std::fs::rename(&op.source, &op.dest)
            .with_context(|| format!("Failed to move {:?} to {:?}", op.source, op.dest))?;

        println!("  {} {}", "✓".green(), op.dest.display());

        // Move auxiliary files
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

            std::fs::rename(&aux.source, &aux.dest)
                .with_context(|| format!("Failed to move {:?} to {:?}", aux.source, aux.dest))?;

            println!(
                "    {} {}",
                "+".cyan(),
                aux.dest.file_name().unwrap_or_default().to_string_lossy()
            );
            aux_count += 1;
        }

        // Try to remove empty parent directories
        cleanup_empty_dirs(&op.source);
    }

    println!();
    if aux_count > 0 {
        println!(
            "{} {} audiobook(s) + {} auxiliary file(s) moved.",
            "Done!".green().bold(),
            plan.needs_fix.len(),
            aux_count
        );
    } else {
        println!(
            "{} {} file(s) moved.",
            "Done!".green().bold(),
            plan.needs_fix.len()
        );
    }

    Ok(())
}
```

**Step 2: Run cargo check**

Run: `cargo check`
Expected: Compiles

**Step 3: Commit**

```bash
git add src/commands/fix.rs
git commit -m "feat(fix): move auxiliary files during execution"
```

---

## Task 17: Run full test suite and clippy

**Files:** None (verification only)

**Step 1: Run all tests**

Run: `cargo test`
Expected: All tests PASS

**Step 2: Run clippy**

Run: `cargo clippy -- -D warnings`
Expected: No warnings

**Step 3: Run format check**

Run: `cargo fmt --check`
Expected: No formatting issues (or run `cargo fmt` to fix)

**Step 4: Commit any fixes if needed**

```bash
git add -A
git commit -m "chore: fix clippy warnings and formatting"
```

---

## Task 18: Final commit and summary

**Step 1: Verify git status is clean**

Run: `git status`
Expected: Clean working tree

**Step 2: View commit log**

Run: `git log --oneline -15`
Expected: See all feature commits

---

## Summary

Features implemented:
1. `{series_title}` template field - renders as "01 - Title" when series_position exists
2. Auxiliary file discovery - scans m4b directory tree for .cue/.pdf files
3. Auxiliary file operations - copies/moves auxiliary files preserving relative structure
4. Display updates - shows auxiliary files in dry-run output
