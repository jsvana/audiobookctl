//! API clients for Audible, Audnexus, and Open Library

use anyhow::{Context, Result};
use serde::Deserialize;
use tracing::warn;

const USER_AGENT: &str = "audiobookctl/0.1.0";

// ============================================================================
// Audible API Response Structs
// ============================================================================

/// Search response from Audible API
#[derive(Debug, Deserialize)]
struct AudibleSearchResponse {
    #[serde(default)]
    products: Vec<AudibleProduct>,
}

/// Single product from Audible search
#[derive(Debug, Deserialize)]
struct AudibleProduct {
    asin: Option<String>,
    title: Option<String>,
    #[serde(default)]
    authors: Vec<AudiblePerson>,
    #[serde(default)]
    narrators: Vec<AudiblePerson>,
    publisher_name: Option<String>,
    publisher_summary: Option<String>,
    release_date: Option<String>,
    #[allow(dead_code)]
    runtime_length_min: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct AudiblePerson {
    name: Option<String>,
}

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
/// Requires ASIN for lookup - Audnexus does not support title/author search.
/// Returns Ok(None) if no ASIN provided or not found, Err only for actual errors.
pub async fn fetch_audnexus(
    client: &reqwest::Client,
    _title: Option<&str>,
    _author: Option<&str>,
    asin: Option<&str>,
) -> Result<Option<LookupResult>> {
    // Audnexus only supports ASIN lookup, no search endpoint
    let Some(asin) = asin else {
        return Ok(None);
    };

    let url = format!("https://api.audnex.us/books/{}", asin);
    let response = client
        .get(&url)
        .header("User-Agent", USER_AGENT)
        .send()
        .await
        .context("Failed to send request to Audnexus")?;

    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }

    if response.status() == reqwest::StatusCode::INTERNAL_SERVER_ERROR {
        // Audnexus returns 500 when item not in their cache
        warn!("ASIN {} not available in Audnexus", asin);
        return Ok(None);
    }

    if !response.status().is_success() {
        warn!("Audnexus ASIN lookup returned status {}", response.status());
        return Ok(None);
    }

    let book: AudnexusBook = response
        .json()
        .await
        .context("Failed to parse Audnexus response")?;
    Ok(Some(audnexus_book_to_result(book)))
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

/// Fetch metadata from Audible API
///
/// Searches by title/author keywords. Returns first result only.
/// This is the primary source for audiobook metadata including narrator info.
pub async fn fetch_audible(
    client: &reqwest::Client,
    title: Option<&str>,
    author: Option<&str>,
) -> Result<Option<LookupResult>> {
    // Build search keywords
    let mut keywords = Vec::new();
    if let Some(title) = title {
        keywords.push(title.to_string());
    }
    if let Some(author) = author {
        keywords.push(author.to_string());
    }

    if keywords.is_empty() {
        return Ok(None);
    }

    let query = keywords.join(" ");
    let url = format!(
        "https://api.audible.com/1.0/catalog/products?response_groups=contributors,product_desc,product_extended_attrs,product_attrs,media&keywords={}&num_results=5&products_sort_by=Relevance",
        urlencoding::encode(&query)
    );

    let response = client
        .get(&url)
        .header("User-Agent", USER_AGENT)
        .send()
        .await
        .context("Failed to send request to Audible")?;

    if !response.status().is_success() {
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        warn!("Audible search returned status {}", response.status());
        return Ok(None);
    }

    let search_response: AudibleSearchResponse = response
        .json()
        .await
        .context("Failed to parse Audible response")?;

    // Take first result only
    if let Some(product) = search_response.products.into_iter().next() {
        Ok(Some(audible_product_to_result(product)))
    } else {
        Ok(None)
    }
}

/// Convert Audible product to LookupResult
fn audible_product_to_result(product: AudibleProduct) -> LookupResult {
    // Join authors/narrators with ", "
    let author = if product.authors.is_empty() {
        None
    } else {
        Some(
            product
                .authors
                .iter()
                .filter_map(|a| a.name.as_ref())
                .cloned()
                .collect::<Vec<_>>()
                .join(", "),
        )
    };

    let narrator = if product.narrators.is_empty() {
        None
    } else {
        Some(
            product
                .narrators
                .iter()
                .filter_map(|n| n.name.as_ref())
                .cloned()
                .collect::<Vec<_>>()
                .join(", "),
        )
    };

    // Extract year from release_date (format: "YYYY-MM-DD")
    let year = product
        .release_date
        .as_ref()
        .and_then(|d| d.split('-').next()?.parse().ok());

    // Strip HTML from description
    let description = product.publisher_summary.map(|s| strip_html_tags(&s));

    LookupResult {
        source: "audible".to_string(),
        title: product.title,
        author,
        narrator,
        series: None, // TODO: Parse from title if present
        series_position: None,
        year,
        description,
        publisher: product.publisher_name,
        genre: None, // Audible search doesn't return genres
        isbn: None,
        asin: product.asin,
    }
}

/// Simple HTML tag stripper
fn strip_html_tags(html: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;

    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => result.push(c),
            _ => {}
        }
    }

    result
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
