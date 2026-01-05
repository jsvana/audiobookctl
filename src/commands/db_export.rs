//! Export audiobook database as CSV

use anyhow::{anyhow, Result};
use std::io;

use crate::database::LibraryDb;

pub fn run(full: bool) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let db = LibraryDb::find_from(&cwd)?
        .ok_or_else(|| anyhow!("No database found. Run 'audiobookctl index <dir>' first."))?;

    let records = db.list_all()?;
    let mut writer = csv::Writer::from_writer(io::stdout());

    // Write header
    if full {
        writer.write_record([
            "title",
            "author",
            "narrator",
            "series",
            "series_position",
            "year",
            "genre",
            "publisher",
            "description",
            "duration_seconds",
            "chapter_count",
            "isbn",
            "asin",
            "file_path",
            "file_size",
            "sha256",
            "indexed_at",
        ])?;
    } else {
        writer.write_record([
            "title",
            "author",
            "narrator",
            "series",
            "series_position",
            "year",
            "genre",
            "publisher",
        ])?;
    }

    // Write records
    for record in records {
        if full {
            writer.write_record([
                record.title.as_deref().unwrap_or(""),
                record.author.as_deref().unwrap_or(""),
                record.narrator.as_deref().unwrap_or(""),
                record.series.as_deref().unwrap_or(""),
                &record
                    .series_position
                    .map(|p| p.to_string())
                    .unwrap_or_default(),
                &record.year.map(|y| y.to_string()).unwrap_or_default(),
                record.genre.as_deref().unwrap_or(""),
                record.publisher.as_deref().unwrap_or(""),
                record.description.as_deref().unwrap_or(""),
                &record
                    .duration_seconds
                    .map(|d| d.to_string())
                    .unwrap_or_default(),
                &record
                    .chapter_count
                    .map(|c| c.to_string())
                    .unwrap_or_default(),
                record.isbn.as_deref().unwrap_or(""),
                record.asin.as_deref().unwrap_or(""),
                &record.file_path,
                &record.file_size.to_string(),
                &record.sha256,
                &record.indexed_at,
            ])?;
        } else {
            writer.write_record([
                record.title.as_deref().unwrap_or(""),
                record.author.as_deref().unwrap_or(""),
                record.narrator.as_deref().unwrap_or(""),
                record.series.as_deref().unwrap_or(""),
                &record
                    .series_position
                    .map(|p| p.to_string())
                    .unwrap_or_default(),
                &record.year.map(|y| y.to_string()).unwrap_or_default(),
                record.genre.as_deref().unwrap_or(""),
                record.publisher.as_deref().unwrap_or(""),
            ])?;
        }
    }

    writer.flush()?;
    Ok(())
}
