//! Merge logic for combining API results

use crate::lookup::LookupResult;
use crate::metadata::AudiobookMetadata;

/// Represents a field's merged state
#[derive(Debug, Clone, PartialEq)]
pub enum FieldValue {
    /// All sources agree on this value
    Agreed { value: String, sources: Vec<String> },
    /// Sources disagree - alternatives grouped by value
    Conflicting {
        selected: String,
        alternatives: Vec<(Vec<String>, String)>, // (source_names, value)
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

/// Merge a single string field from multiple sources
///
/// Existing metadata is treated as a source ("file") and included in conflict detection.
/// If existing value differs from API values, it's shown as a conflict so user can choose.
///
/// Priority:
/// 1. If all sources (including file) agree, use that value (Agreed)
/// 2. If sources disagree, existing file value is selected (Conflicting)
/// 3. If no source has a value, return Empty
fn merge_field(existing: &Option<String>, results: &[(String, Option<String>)]) -> FieldValue {
    use std::collections::HashMap;

    // Build list of all sources including existing file metadata
    let mut all_sources: Vec<(String, Option<String>)> = Vec::new();

    if existing.is_some() {
        all_sources.push(("file".to_string(), existing.clone()));
    }
    all_sources.extend(results.iter().cloned());

    // Group sources by value
    let mut value_to_sources: HashMap<String, Vec<String>> = HashMap::new();
    for (source, value) in &all_sources {
        if let Some(v) = value {
            value_to_sources
                .entry(v.clone())
                .or_default()
                .push(source.clone());
        }
    }

    if value_to_sources.is_empty() {
        return FieldValue::Empty;
    }

    // Convert to ordered list (preserve insertion order via all_sources)
    let mut seen_values: Vec<String> = Vec::new();
    for (_, value) in &all_sources {
        if let Some(v) = value {
            if !seen_values.contains(v) {
                seen_values.push(v.clone());
            }
        }
    }

    let grouped: Vec<(Vec<String>, String)> = seen_values
        .iter()
        .map(|v| (value_to_sources.get(v).unwrap().clone(), v.clone()))
        .collect();

    if grouped.len() == 1 {
        let (sources, value) = grouped.into_iter().next().unwrap();
        FieldValue::Agreed { value, sources }
    } else {
        // Select existing value if present, otherwise first value
        let selected = if let Some(existing_val) = existing {
            existing_val.clone()
        } else {
            grouped[0].1.clone()
        };

        FieldValue::Conflicting {
            selected,
            alternatives: grouped,
        }
    }
}

/// Merge a single u32 field from multiple sources
///
/// Same logic as merge_field but converts u32 to String for FieldValue
fn merge_field_u32(
    existing: &Option<u32>,
    results: &[(String, Option<u32>)], // (source_name, value)
) -> FieldValue {
    // Convert to string options for merge_field
    let existing_str = existing.map(|v| v.to_string());
    let results_str: Vec<(String, Option<String>)> = results
        .iter()
        .map(|(source, value)| (source.clone(), value.map(|v| v.to_string())))
        .collect();

    merge_field(&existing_str, &results_str)
}

/// Merge results from multiple sources, showing conflicts when values differ
///
/// Existing file metadata is treated as a source and compared with API results.
/// This allows users to see and choose between different values.
///
/// Priority order:
/// 1. If all sources (file + APIs) agree, return Agreed
/// 2. If sources disagree, return Conflicting (file value selected by default)
/// 3. If no source has value, return Empty
pub fn merge_results(existing: &AudiobookMetadata, results: &[LookupResult]) -> MergedMetadata {
    // Build (source_name, value) tuples for each field

    // String fields
    let title_values: Vec<(String, Option<String>)> = results
        .iter()
        .map(|r| (r.source.clone(), r.title.clone()))
        .collect();

    let author_values: Vec<(String, Option<String>)> = results
        .iter()
        .map(|r| (r.source.clone(), r.author.clone()))
        .collect();

    let narrator_values: Vec<(String, Option<String>)> = results
        .iter()
        .map(|r| (r.source.clone(), r.narrator.clone()))
        .collect();

    let series_values: Vec<(String, Option<String>)> = results
        .iter()
        .map(|r| (r.source.clone(), r.series.clone()))
        .collect();

    let description_values: Vec<(String, Option<String>)> = results
        .iter()
        .map(|r| (r.source.clone(), r.description.clone()))
        .collect();

    let publisher_values: Vec<(String, Option<String>)> = results
        .iter()
        .map(|r| (r.source.clone(), r.publisher.clone()))
        .collect();

    let genre_values: Vec<(String, Option<String>)> = results
        .iter()
        .map(|r| (r.source.clone(), r.genre.clone()))
        .collect();

    let isbn_values: Vec<(String, Option<String>)> = results
        .iter()
        .map(|r| (r.source.clone(), r.isbn.clone()))
        .collect();

    let asin_values: Vec<(String, Option<String>)> = results
        .iter()
        .map(|r| (r.source.clone(), r.asin.clone()))
        .collect();

    // u32 fields
    let series_position_values: Vec<(String, Option<u32>)> = results
        .iter()
        .map(|r| (r.source.clone(), r.series_position))
        .collect();

    let year_values: Vec<(String, Option<u32>)> =
        results.iter().map(|r| (r.source.clone(), r.year)).collect();

    MergedMetadata {
        title: merge_field(&existing.title, &title_values),
        author: merge_field(&existing.author, &author_values),
        narrator: merge_field(&existing.narrator, &narrator_values),
        series: merge_field(&existing.series, &series_values),
        series_position: merge_field_u32(&existing.series_position, &series_position_values),
        year: merge_field_u32(&existing.year, &year_values),
        description: merge_field(&existing.description, &description_values),
        publisher: merge_field(&existing.publisher, &publisher_values),
        genre: merge_field(&existing.genre, &genre_values),
        isbn: merge_field(&existing.isbn, &isbn_values),
        asin: merge_field(&existing.asin, &asin_values),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_field_groups_agreeing_sources() {
        // When multiple sources have the same value, they should be grouped together
        let existing = None;
        let results = vec![
            ("audible".to_string(), Some("The Martian".to_string())),
            ("openlibrary".to_string(), Some("The Martian".to_string())),
            ("audnexus".to_string(), Some("The Martian".to_string())),
        ];

        let result = merge_field(&existing, &results);
        match result {
            FieldValue::Agreed { value, sources } => {
                assert_eq!(value, "The Martian");
                assert_eq!(sources, vec!["audible", "openlibrary", "audnexus"]);
            }
            _ => panic!("Expected Agreed with sources, got {:?}", result),
        }
    }

    #[test]
    fn test_merge_field_groups_conflicting_by_value() {
        // When sources disagree, group by value
        let existing = None;
        let results = vec![
            ("audible".to_string(), Some("The Martian".to_string())),
            ("audnexus".to_string(), Some("The Martian".to_string())),
            (
                "openlibrary".to_string(),
                Some("The Martian: A Novel".to_string()),
            ),
        ];

        let result = merge_field(&existing, &results);
        match result {
            FieldValue::Conflicting {
                selected,
                alternatives,
            } => {
                assert_eq!(selected, "The Martian");
                // Alternatives should be grouped: (sources, value)
                assert_eq!(alternatives.len(), 2);
                assert_eq!(
                    alternatives[0],
                    (
                        vec!["audible".to_string(), "audnexus".to_string()],
                        "The Martian".to_string()
                    )
                );
                assert_eq!(
                    alternatives[1],
                    (
                        vec!["openlibrary".to_string()],
                        "The Martian: A Novel".to_string()
                    )
                );
            }
            _ => panic!("Expected Conflicting, got {:?}", result),
        }
    }

    fn make_lookup_result(source: &str) -> LookupResult {
        LookupResult {
            source: source.to_string(),
            title: None,
            author: None,
            narrator: None,
            series: None,
            series_position: None,
            year: None,
            description: None,
            publisher: None,
            genre: None,
            isbn: None,
            asin: None,
        }
    }

    #[test]
    fn test_merge_field_existing_shows_conflict() {
        // When existing value differs from API values, show conflict with existing as default
        let existing = Some("The Martian".to_string());
        let results = vec![
            ("audnexus".to_string(), Some("Martian".to_string())),
            (
                "openlibrary".to_string(),
                Some("The Martian: A Novel".to_string()),
            ),
        ];

        let result = merge_field(&existing, &results);
        match result {
            FieldValue::Conflicting {
                selected,
                alternatives,
            } => {
                assert_eq!(selected, "The Martian"); // Existing is selected by default
                assert_eq!(alternatives.len(), 3); // 3 different values
                assert_eq!(alternatives[0].0, vec!["file".to_string()]);
                assert_eq!(alternatives[0].1, "The Martian");
            }
            _ => panic!("Expected Conflicting, got {:?}", result),
        }
    }

    #[test]
    fn test_merge_field_sources_agree() {
        let existing = None;
        let results = vec![
            ("audnexus".to_string(), Some("2014".to_string())),
            ("openlibrary".to_string(), Some("2014".to_string())),
        ];

        let result = merge_field(&existing, &results);
        match result {
            FieldValue::Agreed { value, sources } => {
                assert_eq!(value, "2014");
                assert_eq!(sources, vec!["audnexus", "openlibrary"]);
            }
            _ => panic!("Expected Agreed, got {:?}", result),
        }
    }

    #[test]
    fn test_merge_field_sources_disagree() {
        let existing = None;
        let results = vec![
            ("audnexus".to_string(), Some("2014".to_string())),
            ("openlibrary".to_string(), Some("2011".to_string())),
        ];

        let result = merge_field(&existing, &results);
        match result {
            FieldValue::Conflicting {
                selected,
                alternatives,
            } => {
                assert_eq!(selected, "2014");
                assert_eq!(alternatives.len(), 2);
                assert_eq!(
                    alternatives[0],
                    (vec!["audnexus".to_string()], "2014".to_string())
                );
                assert_eq!(
                    alternatives[1],
                    (vec!["openlibrary".to_string()], "2011".to_string())
                );
            }
            _ => panic!("Expected Conflicting, got {:?}", result),
        }
    }

    #[test]
    fn test_merge_field_no_values() {
        let existing = None;
        let results: Vec<(String, Option<String>)> = vec![
            ("audnexus".to_string(), None),
            ("openlibrary".to_string(), None),
        ];

        let result = merge_field(&existing, &results);
        assert_eq!(result, FieldValue::Empty);
    }

    #[test]
    fn test_merge_field_single_source() {
        let existing = None;
        let results = vec![
            ("audnexus".to_string(), Some("Andy Weir".to_string())),
            ("openlibrary".to_string(), None),
        ];

        let result = merge_field(&existing, &results);
        match result {
            FieldValue::Agreed { value, sources } => {
                assert_eq!(value, "Andy Weir");
                assert_eq!(sources, vec!["audnexus"]);
            }
            _ => panic!("Expected Agreed, got {:?}", result),
        }
    }

    #[test]
    fn test_merge_field_u32_converts_to_string() {
        let existing = None;
        let results = vec![
            ("audnexus".to_string(), Some(2014u32)),
            ("openlibrary".to_string(), Some(2014u32)),
        ];

        let result = merge_field_u32(&existing, &results);
        match result {
            FieldValue::Agreed { value, sources } => {
                assert_eq!(value, "2014");
                assert_eq!(sources, vec!["audnexus", "openlibrary"]);
            }
            _ => panic!("Expected Agreed, got {:?}", result),
        }
    }

    #[test]
    fn test_merge_field_u32_existing_shows_conflict() {
        // When existing value differs from API values, show conflict with existing as default
        let existing = Some(2015u32);
        let results = vec![
            ("audnexus".to_string(), Some(2014u32)),
            ("openlibrary".to_string(), Some(2014u32)),
        ];

        let result = merge_field_u32(&existing, &results);
        match result {
            FieldValue::Conflicting {
                selected,
                alternatives,
            } => {
                assert_eq!(selected, "2015"); // Existing is selected by default
                                              // With grouping: file has 2015, audnexus+openlibrary share 2014
                assert_eq!(alternatives.len(), 2);
                assert_eq!(
                    alternatives[0],
                    (vec!["file".to_string()], "2015".to_string())
                );
                assert_eq!(
                    alternatives[1],
                    (
                        vec!["audnexus".to_string(), "openlibrary".to_string()],
                        "2014".to_string()
                    )
                );
            }
            _ => panic!("Expected Conflicting, got {:?}", result),
        }
    }

    #[test]
    fn test_merge_results_all_empty() {
        let existing = AudiobookMetadata::default();
        let results: Vec<LookupResult> = vec![];

        let merged = merge_results(&existing, &results);
        assert_eq!(merged.title, FieldValue::Empty);
        assert_eq!(merged.author, FieldValue::Empty);
        assert_eq!(merged.year, FieldValue::Empty);
    }

    #[test]
    fn test_merge_results_existing_metadata_shows_conflicts() {
        // When existing metadata differs from API, show as conflict with existing selected
        let existing = AudiobookMetadata {
            title: Some("My Title".to_string()),
            author: Some("My Author".to_string()),
            year: Some(2020),
            ..Default::default()
        };

        let mut audnexus = make_lookup_result("audnexus");
        audnexus.title = Some("Different Title".to_string());
        audnexus.author = Some("Different Author".to_string());
        audnexus.year = Some(2019);

        let results = vec![audnexus];

        let merged = merge_results(&existing, &results);

        // All fields should be Conflicting since existing differs from API
        match &merged.title {
            FieldValue::Conflicting { selected, .. } => {
                assert_eq!(selected, "My Title"); // Existing selected by default
            }
            _ => panic!("Expected title to be Conflicting"),
        }
        match &merged.author {
            FieldValue::Conflicting { selected, .. } => {
                assert_eq!(selected, "My Author");
            }
            _ => panic!("Expected author to be Conflicting"),
        }
        match &merged.year {
            FieldValue::Conflicting { selected, .. } => {
                assert_eq!(selected, "2020");
            }
            _ => panic!("Expected year to be Conflicting"),
        }
    }

    #[test]
    fn test_merge_results_conflict_detection() {
        let existing = AudiobookMetadata::default();

        let mut audnexus = make_lookup_result("audnexus");
        audnexus.title = Some("The Martian".to_string());
        audnexus.year = Some(2014);

        let mut openlibrary = make_lookup_result("openlibrary");
        openlibrary.title = Some("The Martian: A Novel".to_string());
        openlibrary.year = Some(2011);

        let results = vec![audnexus, openlibrary];

        let merged = merge_results(&existing, &results);

        // Title should be conflicting
        match &merged.title {
            FieldValue::Conflicting {
                selected,
                alternatives,
            } => {
                assert_eq!(selected, "The Martian");
                assert_eq!(alternatives.len(), 2);
            }
            _ => panic!("Expected title to be Conflicting"),
        }

        // Year should also be conflicting
        match &merged.year {
            FieldValue::Conflicting {
                selected,
                alternatives,
            } => {
                assert_eq!(selected, "2014");
                assert_eq!(alternatives.len(), 2);
            }
            _ => panic!("Expected year to be Conflicting"),
        }
    }

    #[test]
    fn test_merge_results_mixed_availability() {
        let existing = AudiobookMetadata::default();

        let mut audnexus = make_lookup_result("audnexus");
        audnexus.narrator = Some("R.C. Bray".to_string()); // Only audnexus has narrator
        audnexus.asin = Some("B00B5HZGUG".to_string());

        let mut openlibrary = make_lookup_result("openlibrary");
        openlibrary.isbn = Some("978-0553418026".to_string()); // Only openlibrary has ISBN

        let results = vec![audnexus, openlibrary];

        let merged = merge_results(&existing, &results);

        // Narrator only from audnexus
        match &merged.narrator {
            FieldValue::Agreed { value, sources } => {
                assert_eq!(value, "R.C. Bray");
                assert_eq!(sources, &vec!["audnexus".to_string()]);
            }
            _ => panic!("Expected narrator to be Agreed"),
        }

        // ISBN only from openlibrary
        match &merged.isbn {
            FieldValue::Agreed { value, sources } => {
                assert_eq!(value, "978-0553418026");
                assert_eq!(sources, &vec!["openlibrary".to_string()]);
            }
            _ => panic!("Expected isbn to be Agreed"),
        }

        // ASIN only from audnexus
        match &merged.asin {
            FieldValue::Agreed { value, sources } => {
                assert_eq!(value, "B00B5HZGUG");
                assert_eq!(sources, &vec!["audnexus".to_string()]);
            }
            _ => panic!("Expected asin to be Agreed"),
        }
    }
}
