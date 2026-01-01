//! Search command - query APIs for audiobook metadata without a file

use crate::lookup::{fetch_audible, fetch_audnexus, fetch_openlibrary, LookupResult};
use anyhow::{bail, Context, Result};
use std::io::{self, Write};

/// Run the search command
pub fn run(
    title: Option<&str>,
    author: Option<&str>,
    asin: Option<&str>,
    json: bool,
) -> Result<()> {
    // Validate that at least one search criterion is provided
    if title.is_none() && author.is_none() && asin.is_none() {
        bail!("Please provide at least one search criterion: --title, --author, or --asin");
    }

    let results = query_apis_sync(title, author, asin)?;

    if results.is_empty() {
        println!("No results found.");
        return Ok(());
    }

    if json {
        print_json(&results)?;
    } else {
        print_results(&results);
    }

    Ok(())
}

/// Synchronous wrapper for async API queries
fn query_apis_sync(
    title: Option<&str>,
    author: Option<&str>,
    asin: Option<&str>,
) -> Result<Vec<LookupResult>> {
    let rt = tokio::runtime::Runtime::new().context("Failed to create tokio runtime")?;
    rt.block_on(query_apis(title, author, asin))
}

/// Query APIs concurrently
async fn query_apis(
    title: Option<&str>,
    author: Option<&str>,
    asin: Option<&str>,
) -> Result<Vec<LookupResult>> {
    let client = reqwest::Client::new();
    let mut results = Vec::new();

    // If ASIN provided, query Audnexus first (most accurate)
    if let Some(asin) = asin {
        print!("Querying Audnexus (ASIN: {})... ", asin);
        io::stdout().flush()?;

        match fetch_audnexus(&client, title, author, Some(asin)).await {
            Ok(Some(result)) => {
                println!("found \"{}\"", result.title.as_deref().unwrap_or("Unknown"));
                results.push(result);
            }
            Ok(None) => {
                println!("no results");
            }
            Err(e) => {
                eprintln!("error - {}", e);
            }
        }
    }

    // Query Audible and Open Library concurrently
    print!("Querying Audible... ");
    io::stdout().flush()?;

    let audible_future = fetch_audible(&client, title, author);

    print!("Querying Open Library... ");
    io::stdout().flush()?;

    let openlibrary_future = fetch_openlibrary(&client, title, author, None);

    let (audible_result, openlibrary_result) = tokio::join!(audible_future, openlibrary_future);

    println!();

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

/// Print results in human-readable format
fn print_results(results: &[LookupResult]) {
    println!();
    println!("=== Search Results ===");
    println!();

    for result in results {
        println!("Source: {}", result.source);
        println!("─────────────────────────────────────");

        if let Some(ref title) = result.title {
            println!("  Title:    {}", title);
        }
        if let Some(ref author) = result.author {
            println!("  Author:   {}", author);
        }
        if let Some(ref narrator) = result.narrator {
            println!("  Narrator: {}", narrator);
        }
        if let Some(ref series) = result.series {
            if let Some(pos) = result.series_position {
                println!("  Series:   {} #{}", series, pos);
            } else {
                println!("  Series:   {}", series);
            }
        }
        if let Some(year) = result.year {
            println!("  Year:     {}", year);
        }
        if let Some(ref publisher) = result.publisher {
            println!("  Publisher: {}", publisher);
        }
        if let Some(ref genre) = result.genre {
            println!("  Genre:    {}", genre);
        }
        if let Some(ref asin) = result.asin {
            println!("  ASIN:     {}", asin);
        }
        if let Some(ref isbn) = result.isbn {
            println!("  ISBN:     {}", isbn);
        }
        if let Some(ref desc) = result.description {
            // Truncate long descriptions
            let truncated = if desc.len() > 200 {
                format!("{}...", &desc[..200])
            } else {
                desc.clone()
            };
            println!("  Description: {}", truncated);
        }

        println!();
    }
}

/// Print results as JSON
fn print_json(results: &[LookupResult]) -> Result<()> {
    #[derive(serde::Serialize)]
    struct JsonResult {
        source: String,
        title: Option<String>,
        author: Option<String>,
        narrator: Option<String>,
        series: Option<String>,
        series_position: Option<u32>,
        year: Option<u32>,
        description: Option<String>,
        publisher: Option<String>,
        genre: Option<String>,
        isbn: Option<String>,
        asin: Option<String>,
    }

    let json_results: Vec<JsonResult> = results
        .iter()
        .map(|r| JsonResult {
            source: r.source.clone(),
            title: r.title.clone(),
            author: r.author.clone(),
            narrator: r.narrator.clone(),
            series: r.series.clone(),
            series_position: r.series_position,
            year: r.year,
            description: r.description.clone(),
            publisher: r.publisher.clone(),
            genre: r.genre.clone(),
            isbn: r.isbn.clone(),
            asin: r.asin.clone(),
        })
        .collect();

    let json = serde_json::to_string_pretty(&json_results)?;
    println!("{}", json);

    Ok(())
}
