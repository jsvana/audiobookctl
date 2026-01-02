//! SQLite database for audiobook metadata indexing

// Allow dead code during development - these will be used by commands
#![allow(dead_code)]

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
    #[allow(clippy::too_many_arguments)]
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
        let records = self.list_all()?;
        let mut removed = 0;

        for record in records {
            let full_path = self.base_path.join(&record.file_path);
            if !full_path.exists() {
                self.conn
                    .execute("DELETE FROM audiobooks WHERE id = ?1", params![record.id])?;
                removed += 1;
            }
        }

        Ok(removed)
    }

    /// List all records (for prune operation)
    fn list_all(&self) -> Result<Vec<AudiobookRecord>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, file_path, file_size, sha256, indexed_at,
                   title, author, narrator, series, series_position,
                   year, description, publisher, genre, asin, isbn,
                   duration_seconds, chapter_count
            FROM audiobooks
            "#,
        )?;
        self.collect_records(&mut stmt, [])
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
