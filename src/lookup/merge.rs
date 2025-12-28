//! Merge logic for combining API results

use crate::lookup::LookupResult;
use crate::metadata::AudiobookMetadata;

/// Represents a field's merged state
#[derive(Debug, Clone)]
pub enum FieldValue {
    /// Both sources agree (or only one source has a value)
    Agreed(String),
    /// Sources disagree - first is selected, rest are alternatives
    Conflicting {
        selected: String,
        alternatives: Vec<(String, String)>, // (source_name, value)
    },
    /// No source has this field
    Empty,
}

/// Merged metadata with conflict information
#[derive(Debug)]
pub struct MergedMetadata {
    pub title: FieldValue,
    pub author: FieldValue,
    pub narrator: FieldValue,
    pub series: FieldValue,
    pub series_position: FieldValue,
    pub year: FieldValue,
    pub description: FieldValue,
    pub publisher: FieldValue,
    pub genre: FieldValue,
    pub isbn: FieldValue,
    pub asin: FieldValue,
}

/// Merge results from multiple sources, prioritizing existing metadata
pub fn merge_results(
    _existing: &AudiobookMetadata,
    _results: &[LookupResult],
) -> MergedMetadata {
    // TODO: Implement merge logic
    MergedMetadata {
        title: FieldValue::Empty,
        author: FieldValue::Empty,
        narrator: FieldValue::Empty,
        series: FieldValue::Empty,
        series_position: FieldValue::Empty,
        year: FieldValue::Empty,
        description: FieldValue::Empty,
        publisher: FieldValue::Empty,
        genre: FieldValue::Empty,
        isbn: FieldValue::Empty,
        asin: FieldValue::Empty,
    }
}
