//! Search command - query local audiobook database

use anyhow::{bail, Result};
use colored::Colorize;
use std::path::Path;

use crate::database::{AudiobookRecord, LibraryDb};

/// Maximum records to fetch when combining text search with filters.
/// The text search results are filtered in-memory, so we fetch a larger set.
const COMBINED_SEARCH_LIMIT: usize = 10_000;

/// Run the search command
#[allow(clippy::too_many_arguments)]
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
        LibraryDb::find_from(&cwd)?.ok_or_else(|| {
            anyhow::anyhow!(
                "No database found. Run 'audiobookctl index <dir>' first, or specify --db"
            )
        })?
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
            let text_results = db.search_text(q, COMBINED_SEARCH_LIMIT)?;
            filter_results(
                text_results,
                title,
                author,
                narrator,
                series,
                year,
                asin,
                limit,
            )
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

#[allow(clippy::too_many_arguments)]
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
                if !r
                    .title
                    .as_ref()
                    .map(|v| v.to_lowercase().contains(&t.to_lowercase()))
                    .unwrap_or(false)
                {
                    return false;
                }
            }
            if let Some(a) = author {
                if !r
                    .author
                    .as_ref()
                    .map(|v| v.to_lowercase().contains(&a.to_lowercase()))
                    .unwrap_or(false)
                {
                    return false;
                }
            }
            if let Some(n) = narrator {
                if !r
                    .narrator
                    .as_ref()
                    .map(|v| v.to_lowercase().contains(&n.to_lowercase()))
                    .unwrap_or(false)
                {
                    return false;
                }
            }
            if let Some(s) = series {
                if !r
                    .series
                    .as_ref()
                    .map(|v| v.to_lowercase().contains(&s.to_lowercase()))
                    .unwrap_or(false)
                {
                    return false;
                }
            }
            if let Some(y) = year {
                if r.year != Some(y) {
                    return false;
                }
            }
            if let Some(a) = asin {
                if r.asin.as_deref() != Some(a) {
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
