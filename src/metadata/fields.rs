use serde::{Deserialize, Serialize};

/// Comprehensive audiobook metadata from m4b files
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AudiobookMetadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub narrator: Option<String>,
    pub series: Option<String>,
    pub series_position: Option<u32>,
    pub year: Option<u32>,
    pub description: Option<String>,
    pub publisher: Option<String>,
    pub genre: Option<String>,
    pub duration_seconds: Option<u64>,
    pub chapter_count: Option<u32>,
    pub isbn: Option<String>,
    pub asin: Option<String>,
    /// Cover art info (not the bytes - just format and dimensions if available)
    pub cover_info: Option<String>,
}
