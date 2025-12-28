//! API clients for Audnexus and Open Library

use anyhow::{Context, Result};
use serde::Deserialize;
use tracing::warn;

const USER_AGENT: &str = "audiobookctl/0.1.0";

/// Result from a single API source
#[derive(Debug, Clone)]
pub struct LookupResult {
    pub source: String,
    pub title: Option<String>,
    pub author: Option<String>,
    pub narrator: Option<String>,
    pub series: Option<String>,
    pub series_position: Option<u32>,
    pub year: Option<u32>,
    pub description: Option<String>,
    pub publisher: Option<String>,
    pub genre: Option<String>,
    pub isbn: Option<String>,
    pub asin: Option<String>,
}

// ============================================================================
// Audnexus API Response Structs
// ============================================================================

/// Single book result from Audnexus
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AudnexusBook {
    asin: Option<String>,
    title: Option<String>,
    #[serde(default)]
    authors: Vec<AudnexusPerson>,
    #[serde(default)]
    narrators: Vec<AudnexusPerson>,
    series_primary: Option<AudnexusSeries>,
    publisher_name: Option<String>,
    release_date: Option<String>,
    #[serde(default)]
    genres: Vec<AudnexusGenre>,
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AudnexusPerson {
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AudnexusSeries {
    name: Option<String>,
    position: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AudnexusGenre {
    name: Option<String>,
}

/// Search results from Audnexus (array of books)
type AudnexusSearchResults = Vec<AudnexusBook>;

// ============================================================================
// Open Library API Response Structs
// ============================================================================

/// Search response from Open Library
#[derive(Debug, Deserialize)]
struct OpenLibrarySearchResponse {
    #[serde(default)]
    docs: Vec<OpenLibraryDoc>,
}

/// Single document from Open Library search
#[derive(Debug, Deserialize)]
struct OpenLibraryDoc {
    title: Option<String>,
    #[serde(default)]
    author_name: Vec<String>,
    first_publish_year: Option<u32>,
    #[serde(default)]
    publisher: Vec<String>,
    #[serde(default)]
    isbn: Vec<String>,
    #[serde(default)]
    subject: Vec<String>,
}

// ============================================================================
// API Client Functions
// ============================================================================

/// Fetch metadata from Audnexus API
///
/// Prefers ASIN lookup if provided, otherwise searches by title/author.
/// Returns Ok(None) if no results found, Err only for actual errors.
pub async fn fetch_audnexus(
    client: &reqwest::Client,
    title: Option<&str>,
    author: Option<&str>,
    asin: Option<&str>,
) -> Result<Option<LookupResult>> {
    // If ASIN is provided, use direct lookup (more reliable)
    if let Some(asin) = asin {
        let url = format!("https://api.audnex.us/books/{}", asin);
        let response = client
            .get(&url)
            .header("User-Agent", USER_AGENT)
            .send()
            .await
            .context("Failed to send request to Audnexus")?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            // ASIN not found, fall through to search
            warn!("ASIN {} not found in Audnexus, trying search", asin);
        } else if response.status().is_success() {
            let book: AudnexusBook = response
                .json()
                .await
                .context("Failed to parse Audnexus response")?;
            return Ok(Some(audnexus_book_to_result(book)));
        } else {
            warn!(
                "Audnexus ASIN lookup returned status {}, trying search",
                response.status()
            );
        }
    }

    // Search by title and/or author
    if title.is_none() && author.is_none() {
        return Ok(None);
    }

    let mut url = String::from("https://api.audnex.us/books?");
    let mut params = Vec::new();

    if let Some(title) = title {
        params.push(format!("title={}", urlencoding::encode(title)));
    }
    if let Some(author) = author {
        params.push(format!("author={}", urlencoding::encode(author)));
    }

    url.push_str(&params.join("&"));

    let response = client
        .get(&url)
        .header("User-Agent", USER_AGENT)
        .send()
        .await
        .context("Failed to send search request to Audnexus")?;

    if !response.status().is_success() {
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        anyhow::bail!("Audnexus search returned status {}", response.status());
    }

    let results: AudnexusSearchResults = response
        .json()
        .await
        .context("Failed to parse Audnexus search response")?;

    if let Some(book) = results.into_iter().next() {
        Ok(Some(audnexus_book_to_result(book)))
    } else {
        Ok(None)
    }
}

/// Convert Audnexus book response to LookupResult
fn audnexus_book_to_result(book: AudnexusBook) -> LookupResult {
    // Join authors/narrators with ", "
    let author = if book.authors.is_empty() {
        None
    } else {
        Some(
            book.authors
                .iter()
                .filter_map(|a| a.name.as_ref())
                .cloned()
                .collect::<Vec<_>>()
                .join(", "),
        )
    };

    let narrator = if book.narrators.is_empty() {
        None
    } else {
        Some(
            book.narrators
                .iter()
                .filter_map(|n| n.name.as_ref())
                .cloned()
                .collect::<Vec<_>>()
                .join(", "),
        )
    };

    // Extract series info
    let (series, series_position) = if let Some(ref s) = book.series_primary {
        let position = s.position.as_ref().and_then(|p| {
            // Handle "1", "1.5", etc - parse as float then truncate
            p.parse::<f32>().ok().map(|f| f as u32)
        });
        (s.name.clone(), position)
    } else {
        (None, None)
    };

    // Extract first genre
    let genre = book.genres.first().and_then(|g| g.name.clone());

    // Extract year from release_date (format: "YYYY-MM-DD" or similar)
    let year = book
        .release_date
        .as_ref()
        .and_then(|d| d.split('-').next()?.parse().ok());

    LookupResult {
        source: "audnexus".to_string(),
        title: book.title,
        author,
        narrator,
        series,
        series_position,
        year,
        description: book.description,
        publisher: book.publisher_name,
        genre,
        isbn: None, // Audnexus doesn't provide ISBN
        asin: book.asin,
    }
}

/// Fetch metadata from Open Library API
///
/// Searches by title/author or ISBN. Returns first result only.
/// Returns Ok(None) if no results found, Err only for actual errors.
pub async fn fetch_openlibrary(
    client: &reqwest::Client,
    title: Option<&str>,
    author: Option<&str>,
    isbn: Option<&str>,
) -> Result<Option<LookupResult>> {
    // Build search URL
    let url = if let Some(isbn) = isbn {
        // ISBN search is more specific
        format!(
            "https://openlibrary.org/search.json?isbn={}",
            urlencoding::encode(isbn)
        )
    } else if title.is_some() || author.is_some() {
        // Search by title and/or author
        let mut params = Vec::new();
        if let Some(title) = title {
            params.push(format!("title={}", urlencoding::encode(title)));
        }
        if let Some(author) = author {
            params.push(format!("author={}", urlencoding::encode(author)));
        }
        format!("https://openlibrary.org/search.json?{}", params.join("&"))
    } else {
        return Ok(None);
    };

    let response = client
        .get(&url)
        .header("User-Agent", USER_AGENT)
        .send()
        .await
        .context("Failed to send request to Open Library")?;

    if !response.status().is_success() {
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        anyhow::bail!("Open Library returned status {}", response.status());
    }

    let search_response: OpenLibrarySearchResponse = response
        .json()
        .await
        .context("Failed to parse Open Library response")?;

    // Take first result only
    if let Some(doc) = search_response.docs.into_iter().next() {
        Ok(Some(openlibrary_doc_to_result(doc)))
    } else {
        Ok(None)
    }
}

/// Convert Open Library document to LookupResult
fn openlibrary_doc_to_result(doc: OpenLibraryDoc) -> LookupResult {
    // Join authors with ", "
    let author = if doc.author_name.is_empty() {
        None
    } else {
        Some(doc.author_name.join(", "))
    };

    // Take first publisher
    let publisher = doc.publisher.into_iter().next();

    // Take first ISBN
    let isbn = doc.isbn.into_iter().next();

    // Take first subject as genre
    let genre = doc.subject.into_iter().next();

    LookupResult {
        source: "openlibrary".to_string(),
        title: doc.title,
        author,
        narrator: None, // Open Library doesn't have narrator info
        series: None,   // Open Library doesn't have structured series info
        series_position: None,
        year: doc.first_publish_year,
        description: None, // Search results don't include description
        publisher,
        genre,
        isbn,
        asin: None, // Open Library doesn't provide ASIN
    }
}
