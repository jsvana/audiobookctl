# Idempotent Organize & Local Search Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Make organize idempotent via SHA256 comparison and add SQLite-based local search.

**Architecture:** Add hash utility for SHA256 streaming, extend planner with AlreadyPresent variant for idempotent copies, create database module for SQLite operations, repurpose search command for local queries.

**Tech Stack:** rusqlite (bundled SQLite), sha2 (already present), existing clap/serde stack

---

## Task 1: Add SHA256 Hash Utility

**Files:**
- Create: `src/hash.rs`
- Modify: `src/main.rs:1-8` (add module declaration)

**Step 1: Create hash utility module**

Create `src/hash.rs`:

```rust
//! SHA256 file hashing utilities

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

/// Compute SHA256 hash of a file, streaming to avoid loading into memory
pub fn sha256_file(path: &Path) -> Result<String> {
    let file = File::open(path).with_context(|| format!("Failed to open {:?}", path))?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];

    loop {
        let bytes_read = reader
            .read(&mut buffer)
            .with_context(|| format!("Failed to read {:?}", path))?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(hex::encode(hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_sha256_known_content() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"hello world").unwrap();
        file.flush().unwrap();

        let hash = sha256_file(file.path()).unwrap();
        // SHA256 of "hello world"
        assert_eq!(
            hash,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn test_sha256_empty_file() {
        let file = NamedTempFile::new().unwrap();
        let hash = sha256_file(file.path()).unwrap();
        // SHA256 of empty string
        assert_eq!(
            hash,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }
}
```

**Step 2: Add module declaration to main.rs**

Add after line 7 in `src/main.rs`:

```rust
mod hash;
```

**Step 3: Run tests to verify**

Run: `cargo test hash::`
Expected: 2 tests pass

**Step 4: Commit**

```bash
git add src/hash.rs src/main.rs
git commit -m "feat: add SHA256 file hashing utility"
```

---

## Task 2: Extend Planner with AlreadyPresent Variant

**Files:**
- Modify: `src/organize/planner.rs`

**Step 1: Add AlreadyPresent struct after PlannedOperation (line 21)**

Add after `PlannedOperation` struct:

```rust
/// A file that already exists at destination with matching content
#[derive(Debug, Clone)]
pub struct AlreadyPresent {
    pub source: PathBuf,
    pub dest: PathBuf,
    pub hash: String,
}
```

**Step 2: Add already_present field to OrganizePlan struct (after line 46)**

Modify `OrganizePlan` to add field:

```rust
/// Result of planning an organize operation
#[derive(Debug)]
pub struct OrganizePlan {
    /// Operations to perform
    pub operations: Vec<PlannedOperation>,
    /// Files already present at destination (hash match)
    pub already_present: Vec<AlreadyPresent>,
    /// Files that couldn't be organized (missing metadata)
    pub uncategorized: Vec<UncategorizedFile>,
    /// Detected conflicts
    pub conflicts: Vec<Conflict>,
}
```

**Step 3: Update OrganizePlan::build to detect already-present files**

Replace the conflict detection logic (lines 92-105) and add hash import at top:

Add import at top of file:
```rust
use crate::hash::sha256_file;
```

Replace the conflict detection section starting at line 92:

```rust
        // Detect conflicts and already-present files
        let mut conflicts = Vec::new();
        let mut already_present = Vec::new();
        let mut ops_to_remove = Vec::new();

        for (dest, sources) in &dest_to_sources {
            let exists_on_disk = dest.exists();

            if sources.len() > 1 {
                // Multiple sources mapping to same dest - always a conflict
                conflicts.push(Conflict {
                    dest: dest.clone(),
                    sources: sources.clone(),
                    exists_on_disk,
                });
            } else if exists_on_disk {
                // Single source but dest exists - check hash
                let source = &sources[0];
                match (sha256_file(source), sha256_file(dest)) {
                    (Ok(src_hash), Ok(dest_hash)) if src_hash == dest_hash => {
                        // Same content - mark as already present
                        already_present.push(AlreadyPresent {
                            source: source.clone(),
                            dest: dest.clone(),
                            hash: src_hash,
                        });
                        ops_to_remove.push(source.clone());
                    }
                    (Ok(_), Ok(_)) => {
                        // Different content - conflict
                        conflicts.push(Conflict {
                            dest: dest.clone(),
                            sources: sources.clone(),
                            exists_on_disk: true,
                        });
                    }
                    (Err(_), _) | (_, Err(_)) => {
                        // Hash error - treat as conflict to be safe
                        conflicts.push(Conflict {
                            dest: dest.clone(),
                            sources: sources.clone(),
                            exists_on_disk: true,
                        });
                    }
                }
            }
        }

        // Remove already-present files from operations
        operations.retain(|op| !ops_to_remove.contains(&op.source));
```

**Step 4: Update sort section (around line 107) to include already_present**

Add after sorting operations:

```rust
        already_present.sort_by(|a, b| a.source.cmp(&b.source));
```

**Step 5: Update return statement to include already_present**

```rust
        Self {
            operations,
            already_present,
            uncategorized,
            conflicts,
        }
```

**Step 6: Run tests**

Run: `cargo test organize::planner::`
Expected: existing tests pass

**Step 7: Commit**

```bash
git add src/organize/planner.rs
git commit -m "feat(organize): detect already-present files via SHA256"
```

---

## Task 3: Update Organize Command Display

**Files:**
- Modify: `src/commands/organize.rs`
- Modify: `src/organize/mod.rs` (export AlreadyPresent)

**Step 1: Export AlreadyPresent from organize module**

Check `src/organize/mod.rs` and add `AlreadyPresent` to exports:

```rust
pub use planner::{AlreadyPresent, ...};
```

**Step 2: Update imports in organize.rs**

Add `AlreadyPresent` to the import from organize module.

**Step 3: Add display function for already-present files**

Add after `print_conflicts` function:

```rust
fn print_already_present(already_present: &[crate::organize::AlreadyPresent]) {
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
            file.source.file_name().unwrap_or_default().to_string_lossy(),
            file.dest.display()
        );
    }
}
```

**Step 4: Call print_already_present in run function**

After displaying the plan (around line 80), add:

```rust
    // Show already-present files
    print_already_present(&plan.already_present);
```

**Step 5: Update file count in output messages**

Update the `print_tree_view` and `print_list_view` functions to optionally show already-present count in the summary.

**Step 6: Run manual test**

Run: `cargo build && cargo run -- organize --source /tmp/test --dest /tmp/out`

**Step 7: Commit**

```bash
git add src/commands/organize.rs src/organize/mod.rs
git commit -m "feat(organize): display already-present files in plan output"
```

---

## Task 4: Add rusqlite Dependency

**Files:**
- Modify: `Cargo.toml`

**Step 1: Add rusqlite to dependencies**

Add to `[dependencies]` section:

```toml
rusqlite = { version = "0.32", features = ["bundled"] }
```

**Step 2: Verify builds**

Run: `cargo build`
Expected: successful build

**Step 3: Commit**

```bash
git add Cargo.toml
git commit -m "chore: add rusqlite dependency"
```

---

## Task 5: Create Database Module

**Files:**
- Create: `src/database/mod.rs`
- Modify: `src/main.rs` (add module)

**Step 1: Create database module**

Create `src/database/mod.rs`:

```rust
//! SQLite database for audiobook metadata indexing

use anyhow::{Context, Result};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::{Path, PathBuf};

use crate::metadata::AudiobookMetadata;

const DB_FILENAME: &str = ".audiobookctl.db";

/// Database handle for audiobook library
pub struct LibraryDb {
    conn: Connection,
    base_path: PathBuf,
}

/// A record from the database
#[derive(Debug, Clone)]
pub struct AudiobookRecord {
    pub id: i64,
    pub file_path: String,
    pub file_size: i64,
    pub sha256: String,
    pub indexed_at: String,
    pub title: Option<String>,
    pub author: Option<String>,
    pub narrator: Option<String>,
    pub series: Option<String>,
    pub series_position: Option<f64>,
    pub year: Option<i32>,
    pub description: Option<String>,
    pub publisher: Option<String>,
    pub genre: Option<String>,
    pub asin: Option<String>,
    pub isbn: Option<String>,
    pub duration_seconds: Option<i64>,
    pub chapter_count: Option<i32>,
}

impl LibraryDb {
    /// Open or create database in the given directory
    pub fn open(dir: &Path) -> Result<Self> {
        let db_path = dir.join(DB_FILENAME);
        let conn = Connection::open(&db_path)
            .with_context(|| format!("Failed to open database at {:?}", db_path))?;

        let db = Self {
            conn,
            base_path: dir.to_path_buf(),
        };
        db.init_schema()?;
        Ok(db)
    }

    /// Find database by walking up from current directory
    pub fn find_from(start: &Path) -> Result<Option<Self>> {
        let mut current = start.to_path_buf();
        loop {
            let db_path = current.join(DB_FILENAME);
            if db_path.exists() {
                return Ok(Some(Self::open(&current)?));
            }
            if !current.pop() {
                return Ok(None);
            }
        }
    }

    fn init_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS audiobooks (
                id INTEGER PRIMARY KEY,
                file_path TEXT NOT NULL UNIQUE,
                file_size INTEGER NOT NULL,
                sha256 TEXT NOT NULL,
                indexed_at TEXT NOT NULL,
                title TEXT,
                author TEXT,
                narrator TEXT,
                series TEXT,
                series_position REAL,
                year INTEGER,
                description TEXT,
                publisher TEXT,
                genre TEXT,
                asin TEXT,
                isbn TEXT,
                duration_seconds INTEGER,
                chapter_count INTEGER
            );

            CREATE INDEX IF NOT EXISTS idx_author ON audiobooks(author);
            CREATE INDEX IF NOT EXISTS idx_title ON audiobooks(title);
            CREATE INDEX IF NOT EXISTS idx_series ON audiobooks(series);
            CREATE INDEX IF NOT EXISTS idx_sha256 ON audiobooks(sha256);
            "#,
        )?;
        Ok(())
    }

    /// Insert or update an audiobook record
    pub fn upsert(
        &self,
        relative_path: &str,
        file_size: i64,
        sha256: &str,
        metadata: &AudiobookMetadata,
    ) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();

        self.conn.execute(
            r#"
            INSERT INTO audiobooks (
                file_path, file_size, sha256, indexed_at,
                title, author, narrator, series, series_position,
                year, description, publisher, genre, asin, isbn,
                duration_seconds, chapter_count
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)
            ON CONFLICT(file_path) DO UPDATE SET
                file_size = excluded.file_size,
                sha256 = excluded.sha256,
                indexed_at = excluded.indexed_at,
                title = excluded.title,
                author = excluded.author,
                narrator = excluded.narrator,
                series = excluded.series,
                series_position = excluded.series_position,
                year = excluded.year,
                description = excluded.description,
                publisher = excluded.publisher,
                genre = excluded.genre,
                asin = excluded.asin,
                isbn = excluded.isbn,
                duration_seconds = excluded.duration_seconds,
                chapter_count = excluded.chapter_count
            "#,
            params![
                relative_path,
                file_size,
                sha256,
                now,
                metadata.title,
                metadata.author,
                metadata.narrator,
                metadata.series,
                metadata.series_position.map(|p| p as f64),
                metadata.year.map(|y| y as i32),
                metadata.description,
                metadata.publisher,
                metadata.genre,
                metadata.asin,
                metadata.isbn,
                metadata.duration_seconds.map(|d| d as i64),
                metadata.chapter_count.map(|c| c as i32),
            ],
        )?;
        Ok(())
    }

    /// Update indexed_at timestamp for a file (for already-present files)
    pub fn touch(&self, relative_path: &str) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE audiobooks SET indexed_at = ?1 WHERE file_path = ?2",
            params![now, relative_path],
        )?;
        Ok(())
    }

    /// Search audiobooks by free text (searches title, author, narrator, series, description)
    pub fn search_text(&self, query: &str, limit: usize) -> Result<Vec<AudiobookRecord>> {
        let pattern = format!("%{}%", query);
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, file_path, file_size, sha256, indexed_at,
                   title, author, narrator, series, series_position,
                   year, description, publisher, genre, asin, isbn,
                   duration_seconds, chapter_count
            FROM audiobooks
            WHERE title LIKE ?1 OR author LIKE ?1 OR narrator LIKE ?1
                  OR series LIKE ?1 OR description LIKE ?1
            ORDER BY author, series, series_position, title
            LIMIT ?2
            "#,
        )?;

        self.collect_records(&mut stmt, params![pattern, limit as i64])
    }

    /// Search with field-specific filters
    pub fn search_filtered(
        &self,
        title: Option<&str>,
        author: Option<&str>,
        narrator: Option<&str>,
        series: Option<&str>,
        year: Option<i32>,
        asin: Option<&str>,
        limit: usize,
    ) -> Result<Vec<AudiobookRecord>> {
        let mut conditions = Vec::new();
        let mut values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(t) = title {
            conditions.push("title LIKE ?");
            values.push(Box::new(format!("%{}%", t)));
        }
        if let Some(a) = author {
            conditions.push("author LIKE ?");
            values.push(Box::new(format!("%{}%", a)));
        }
        if let Some(n) = narrator {
            conditions.push("narrator LIKE ?");
            values.push(Box::new(format!("%{}%", n)));
        }
        if let Some(s) = series {
            conditions.push("series LIKE ?");
            values.push(Box::new(format!("%{}%", s)));
        }
        if let Some(y) = year {
            conditions.push("year = ?");
            values.push(Box::new(y));
        }
        if let Some(a) = asin {
            conditions.push("asin = ?");
            values.push(Box::new(a.to_string()));
        }

        let where_clause = if conditions.is_empty() {
            "1=1".to_string()
        } else {
            conditions.join(" AND ")
        };

        let sql = format!(
            r#"
            SELECT id, file_path, file_size, sha256, indexed_at,
                   title, author, narrator, series, series_position,
                   year, description, publisher, genre, asin, isbn,
                   duration_seconds, chapter_count
            FROM audiobooks
            WHERE {}
            ORDER BY author, series, series_position, title
            LIMIT ?
            "#,
            where_clause
        );

        values.push(Box::new(limit as i64));

        let mut stmt = self.conn.prepare(&sql)?;
        let params: Vec<&dyn rusqlite::ToSql> = values.iter().map(|v| v.as_ref()).collect();
        self.collect_records_dyn(&mut stmt, params.as_slice())
    }

    /// Get record by file path
    pub fn get_by_path(&self, relative_path: &str) -> Result<Option<AudiobookRecord>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, file_path, file_size, sha256, indexed_at,
                   title, author, narrator, series, series_position,
                   year, description, publisher, genre, asin, isbn,
                   duration_seconds, chapter_count
            FROM audiobooks
            WHERE file_path = ?1
            "#,
        )?;

        stmt.query_row(params![relative_path], |row| self.row_to_record(row))
            .optional()
            .context("Failed to query by path")
    }

    /// Remove entries for files that no longer exist
    pub fn prune(&self) -> Result<usize> {
        let records = self.search_text("", 100000)?; // Get all
        let mut removed = 0;

        for record in records {
            let full_path = self.base_path.join(&record.file_path);
            if !full_path.exists() {
                self.conn.execute(
                    "DELETE FROM audiobooks WHERE id = ?1",
                    params![record.id],
                )?;
                removed += 1;
            }
        }

        Ok(removed)
    }

    /// Get total count of records
    pub fn count(&self) -> Result<i64> {
        self.conn
            .query_row("SELECT COUNT(*) FROM audiobooks", [], |row| row.get(0))
            .context("Failed to count records")
    }

    /// Get base path for this database
    pub fn base_path(&self) -> &Path {
        &self.base_path
    }

    fn collect_records(
        &self,
        stmt: &mut rusqlite::Statement,
        params: impl rusqlite::Params,
    ) -> Result<Vec<AudiobookRecord>> {
        let rows = stmt.query_map(params, |row| self.row_to_record(row))?;
        rows.collect::<Result<Vec<_>, _>>()
            .context("Failed to collect records")
    }

    fn collect_records_dyn(
        &self,
        stmt: &mut rusqlite::Statement,
        params: &[&dyn rusqlite::ToSql],
    ) -> Result<Vec<AudiobookRecord>> {
        let rows = stmt.query_map(params, |row| self.row_to_record(row))?;
        rows.collect::<Result<Vec<_>, _>>()
            .context("Failed to collect records")
    }

    fn row_to_record(&self, row: &rusqlite::Row) -> rusqlite::Result<AudiobookRecord> {
        Ok(AudiobookRecord {
            id: row.get(0)?,
            file_path: row.get(1)?,
            file_size: row.get(2)?,
            sha256: row.get(3)?,
            indexed_at: row.get(4)?,
            title: row.get(5)?,
            author: row.get(6)?,
            narrator: row.get(7)?,
            series: row.get(8)?,
            series_position: row.get(9)?,
            year: row.get(10)?,
            description: row.get(11)?,
            publisher: row.get(12)?,
            genre: row.get(13)?,
            asin: row.get(14)?,
            isbn: row.get(15)?,
            duration_seconds: row.get(16)?,
            chapter_count: row.get(17)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_create_and_query() {
        let dir = TempDir::new().unwrap();
        let db = LibraryDb::open(dir.path()).unwrap();

        let metadata = AudiobookMetadata {
            title: Some("Test Book".to_string()),
            author: Some("Test Author".to_string()),
            ..Default::default()
        };

        db.upsert("test/book.m4b", 1000, "abc123", &metadata)
            .unwrap();

        let results = db.search_text("Test", 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, Some("Test Book".to_string()));
    }

    #[test]
    fn test_upsert_updates() {
        let dir = TempDir::new().unwrap();
        let db = LibraryDb::open(dir.path()).unwrap();

        let metadata1 = AudiobookMetadata {
            title: Some("Original".to_string()),
            ..Default::default()
        };
        db.upsert("book.m4b", 1000, "abc", &metadata1).unwrap();

        let metadata2 = AudiobookMetadata {
            title: Some("Updated".to_string()),
            ..Default::default()
        };
        db.upsert("book.m4b", 1000, "abc", &metadata2).unwrap();

        assert_eq!(db.count().unwrap(), 1);
        let record = db.get_by_path("book.m4b").unwrap().unwrap();
        assert_eq!(record.title, Some("Updated".to_string()));
    }
}
```

**Step 2: Add module to main.rs**

Add after other module declarations:

```rust
mod database;
```

**Step 3: Run tests**

Run: `cargo test database::`
Expected: 2 tests pass

**Step 4: Commit**

```bash
git add src/database/mod.rs src/main.rs
git commit -m "feat: add SQLite database module for audiobook indexing"
```

---

## Task 6: Integrate Database into Organize Command

**Files:**
- Modify: `src/commands/organize.rs`

**Step 1: Add database import**

Add to imports:

```rust
use crate::database::LibraryDb;
use crate::hash::sha256_file;
use crate::metadata::AudiobookMetadata;
```

**Step 2: Update execute_plan to write to database**

Modify `execute_plan` function signature to accept metadata map and dest:

```rust
fn execute_plan(
    operations: &[PlannedOperation],
    already_present: &[crate::organize::AlreadyPresent],
    uncategorized: &[UncategorizedFile],
    dest: &Path,
    allow_uncategorized: bool,
    file_metadata: &HashMap<PathBuf, AudiobookMetadata>,
) -> Result<()> {
```

After copying files successfully, add database writes. Add this section after the copy loop:

```rust
    // Update database
    println!();
    println!("{}", "Updating database...".cyan());

    let db = LibraryDb::open(dest)?;
    let mut db_count = 0;

    for op in operations {
        if let Some(metadata) = file_metadata.get(&op.source) {
            let relative = op.dest.strip_prefix(dest).unwrap_or(&op.dest);
            let file_size = std::fs::metadata(&op.dest)?.len() as i64;
            let hash = sha256_file(&op.dest)?;
            db.upsert(
                &relative.to_string_lossy(),
                file_size,
                &hash,
                metadata,
            )?;
            db_count += 1;
        }
    }

    // Touch already-present files
    for ap in already_present {
        let relative = ap.dest.strip_prefix(dest).unwrap_or(&ap.dest);
        db.touch(&relative.to_string_lossy())?;
        db_count += 1;
    }

    println!("  {} record(s) updated", db_count);
```

**Step 3: Update run function to pass metadata to execute_plan**

Build a metadata map from scanned files before planning:

```rust
    // Build metadata map for database writes
    let file_metadata: HashMap<PathBuf, AudiobookMetadata> = files
        .iter()
        .map(|f| (f.path.clone(), f.metadata.clone()))
        .collect();
```

Pass it to execute_plan.

**Step 4: Run integration test**

Run: `cargo build && cargo run -- organize --source /tmp/test --dest /tmp/out --no-dry-run`

**Step 5: Commit**

```bash
git add src/commands/organize.rs
git commit -m "feat(organize): write to SQLite database after copying files"
```

---

## Task 7: Create Index Command

**Files:**
- Create: `src/commands/index.rs`
- Modify: `src/commands/mod.rs`
- Modify: `src/cli.rs`
- Modify: `src/main.rs`

**Step 1: Create index command**

Create `src/commands/index.rs`:

```rust
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
```

**Step 2: Add to commands/mod.rs**

Add:

```rust
pub mod index;
```

**Step 3: Add CLI definition in cli.rs**

Add to `Commands` enum:

```rust
    /// Index audiobooks in a directory for local search
    Index {
        /// Directory to index
        dir: PathBuf,

        /// Re-index all files (ignore existing entries)
        #[arg(long)]
        full: bool,

        /// Remove entries for files that no longer exist
        #[arg(long)]
        prune: bool,
    },
```

**Step 4: Add handler in main.rs**

Add match arm:

```rust
        Commands::Index { dir, full, prune } => {
            commands::index::run(&dir, full, prune)?;
        }
```

**Step 5: Run test**

Run: `cargo build && cargo run -- index /tmp/out`

**Step 6: Commit**

```bash
git add src/commands/index.rs src/commands/mod.rs src/cli.rs src/main.rs
git commit -m "feat: add index command for manual library indexing"
```

---

## Task 8: Rewrite Search Command for Local Queries

**Files:**
- Modify: `src/commands/search.rs`
- Modify: `src/cli.rs`

**Step 1: Update CLI definition for search**

Replace the Search variant in `cli.rs`:

```rust
    /// Search local audiobook database
    Search {
        /// Free-text search query
        query: Option<String>,

        /// Filter by title
        #[arg(long)]
        title: Option<String>,

        /// Filter by author
        #[arg(long)]
        author: Option<String>,

        /// Filter by narrator
        #[arg(long)]
        narrator: Option<String>,

        /// Filter by series
        #[arg(long)]
        series: Option<String>,

        /// Filter by year
        #[arg(long)]
        year: Option<i32>,

        /// Filter by ASIN
        #[arg(long)]
        asin: Option<String>,

        /// Path to database (auto-detected if not specified)
        #[arg(long)]
        db: Option<PathBuf>,

        /// Maximum results to show
        #[arg(long, default_value = "50")]
        limit: usize,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
```

**Step 2: Rewrite search.rs**

Replace entire file:

```rust
//! Search command - query local audiobook database

use anyhow::{bail, Result};
use colored::Colorize;
use std::path::Path;

use crate::database::{AudiobookRecord, LibraryDb};

/// Run the search command
pub fn run(
    query: Option<&str>,
    title: Option<&str>,
    author: Option<&str>,
    narrator: Option<&str>,
    series: Option<&str>,
    year: Option<i32>,
    asin: Option<&str>,
    db_path: Option<&Path>,
    limit: usize,
    json: bool,
) -> Result<()> {
    // Open database
    let db = if let Some(path) = db_path {
        LibraryDb::open(path)?
    } else {
        let cwd = std::env::current_dir()?;
        LibraryDb::find_from(&cwd)?
            .ok_or_else(|| anyhow::anyhow!(
                "No database found. Run 'audiobookctl index <dir>' first, or specify --db"
            ))?
    };

    // Determine search mode
    let has_filters = title.is_some()
        || author.is_some()
        || narrator.is_some()
        || series.is_some()
        || year.is_some()
        || asin.is_some();

    let results = if let Some(q) = query {
        if has_filters {
            // Combined: free-text AND filters
            let text_results = db.search_text(q, 10000)?;
            filter_results(text_results, title, author, narrator, series, year, asin, limit)
        } else {
            db.search_text(q, limit)?
        }
    } else if has_filters {
        db.search_filtered(title, author, narrator, series, year, asin, limit)?
    } else {
        bail!("Please provide a search query or filter (--title, --author, etc.)");
    };

    if results.is_empty() {
        println!("No results found.");
        return Ok(());
    }

    if json {
        print_json(&results)?;
    } else {
        print_results(&results, db.base_path());
    }

    Ok(())
}

fn filter_results(
    results: Vec<AudiobookRecord>,
    title: Option<&str>,
    author: Option<&str>,
    narrator: Option<&str>,
    series: Option<&str>,
    year: Option<i32>,
    asin: Option<&str>,
    limit: usize,
) -> Vec<AudiobookRecord> {
    results
        .into_iter()
        .filter(|r| {
            if let Some(t) = title {
                if !r.title.as_ref().map(|v| v.to_lowercase().contains(&t.to_lowercase())).unwrap_or(false) {
                    return false;
                }
            }
            if let Some(a) = author {
                if !r.author.as_ref().map(|v| v.to_lowercase().contains(&a.to_lowercase())).unwrap_or(false) {
                    return false;
                }
            }
            if let Some(n) = narrator {
                if !r.narrator.as_ref().map(|v| v.to_lowercase().contains(&n.to_lowercase())).unwrap_or(false) {
                    return false;
                }
            }
            if let Some(s) = series {
                if !r.series.as_ref().map(|v| v.to_lowercase().contains(&s.to_lowercase())).unwrap_or(false) {
                    return false;
                }
            }
            if let Some(y) = year {
                if r.year != Some(y) {
                    return false;
                }
            }
            if let Some(a) = asin {
                if r.asin.as_ref() != Some(&a.to_string()) {
                    return false;
                }
            }
            true
        })
        .take(limit)
        .collect()
}

fn print_results(results: &[AudiobookRecord], base_path: &Path) {
    println!();
    println!("Found {} result(s):", results.len());
    println!();

    for record in results {
        // Title line
        let title = record.title.as_deref().unwrap_or("Unknown Title");
        println!("{}", title.bold());

        // Author/Narrator
        if let Some(ref author) = record.author {
            print!("  by {}", author.cyan());
            if let Some(ref narrator) = record.narrator {
                print!(", read by {}", narrator);
            }
            println!();
        }

        // Series
        if let Some(ref series) = record.series {
            if let Some(pos) = record.series_position {
                println!("  {} #{}", series.yellow(), pos);
            } else {
                println!("  {}", series.yellow());
            }
        }

        // File path
        let full_path = base_path.join(&record.file_path);
        println!("  {}", full_path.display().to_string().dimmed());

        println!();
    }
}

fn print_json(results: &[AudiobookRecord]) -> Result<()> {
    #[derive(serde::Serialize)]
    struct JsonResult {
        file_path: String,
        title: Option<String>,
        author: Option<String>,
        narrator: Option<String>,
        series: Option<String>,
        series_position: Option<f64>,
        year: Option<i32>,
        description: Option<String>,
        publisher: Option<String>,
        genre: Option<String>,
        asin: Option<String>,
        isbn: Option<String>,
        duration_seconds: Option<i64>,
        chapter_count: Option<i32>,
        sha256: String,
    }

    let json_results: Vec<JsonResult> = results
        .iter()
        .map(|r| JsonResult {
            file_path: r.file_path.clone(),
            title: r.title.clone(),
            author: r.author.clone(),
            narrator: r.narrator.clone(),
            series: r.series.clone(),
            series_position: r.series_position,
            year: r.year,
            description: r.description.clone(),
            publisher: r.publisher.clone(),
            genre: r.genre.clone(),
            asin: r.asin.clone(),
            isbn: r.isbn.clone(),
            duration_seconds: r.duration_seconds,
            chapter_count: r.chapter_count,
            sha256: r.sha256.clone(),
        })
        .collect();

    let json = serde_json::to_string_pretty(&json_results)?;
    println!("{}", json);

    Ok(())
}
```

**Step 3: Update main.rs handler**

Replace the Search match arm:

```rust
        Commands::Search {
            query,
            title,
            author,
            narrator,
            series,
            year,
            asin,
            db,
            limit,
            json,
        } => {
            commands::search::run(
                query.as_deref(),
                title.as_deref(),
                author.as_deref(),
                narrator.as_deref(),
                series.as_deref(),
                year,
                asin.as_deref(),
                db.as_deref(),
                limit,
                json,
            )?;
        }
```

**Step 4: Run tests**

Run: `cargo build && cargo run -- search --help`

**Step 5: Commit**

```bash
git add src/commands/search.rs src/cli.rs src/main.rs
git commit -m "feat(search): repurpose for local SQLite queries"
```

---

## Task 9: Final Integration & Testing

**Step 1: Run full build**

Run: `cargo build --release`

**Step 2: Run all tests**

Run: `cargo test`

**Step 3: Run clippy**

Run: `cargo clippy`

**Step 4: Format check**

Run: `cargo fmt --check`

**Step 5: Manual integration test**

```bash
# Create test directory with some m4b files
mkdir -p /tmp/audiotest/source
# (copy some test m4b files)

# Organize
cargo run -- organize --source /tmp/audiotest/source --dest /tmp/audiotest/library --no-dry-run

# Verify database created
ls /tmp/audiotest/library/.audiobookctl.db

# Search
cd /tmp/audiotest/library
cargo run -- search "Author Name"

# Index existing library
cargo run -- index /tmp/audiotest/library

# Search with filters
cargo run -- search --author "Name" --series "Series"
```

**Step 6: Final commit**

```bash
git add -A
git commit -m "chore: final cleanup and integration"
```
