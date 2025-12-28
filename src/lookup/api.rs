//! API clients for Audnexus and Open Library

use anyhow::Result;

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

/// Fetch metadata from Audnexus API
pub async fn fetch_audnexus(
    _client: &reqwest::Client,
    _title: Option<&str>,
    _author: Option<&str>,
    _asin: Option<&str>,
) -> Result<Option<LookupResult>> {
    // TODO: Implement Audnexus API client
    Ok(None)
}

/// Fetch metadata from Open Library API
pub async fn fetch_openlibrary(
    _client: &reqwest::Client,
    _title: Option<&str>,
    _author: Option<&str>,
    _isbn: Option<&str>,
) -> Result<Option<LookupResult>> {
    // TODO: Implement Open Library API client
    Ok(None)
}
