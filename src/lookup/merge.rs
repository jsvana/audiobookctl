//! Merge logic for combining API results

use crate::lookup::LookupResult;
use crate::metadata::AudiobookMetadata;

/// Represents a field's merged state
#[derive(Debug, Clone, PartialEq)]
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

/// Merge a single string field from multiple sources
///
/// Priority:
/// 1. If existing metadata has a value, use it (Agreed)
/// 2. If all sources with values agree, use that value (Agreed)
/// 3. If sources disagree, first source's value is selected (Conflicting)
/// 4. If no source has a value, return Empty
fn merge_field(
    existing: &Option<String>,
    results: &[(String, Option<String>)], // (source_name, value)
) -> FieldValue {
    // Priority 1: Existing metadata takes precedence
    if let Some(value) = existing {
        return FieldValue::Agreed(value.clone());
    }

    // Collect all sources that have a value for this field
    let values_with_sources: Vec<(&String, &String)> = results
        .iter()
        .filter_map(|(source, value)| value.as_ref().map(|v| (source, v)))
        .collect();

    if values_with_sources.is_empty() {
        // Priority 4: No source has this field
        return FieldValue::Empty;
    }

    // Check if all sources agree
    let first_value = values_with_sources[0].1;
    let all_agree = values_with_sources.iter().all(|(_, v)| *v == first_value);

    if all_agree {
        // Priority 2: All sources agree (or only one source)
        FieldValue::Agreed(first_value.clone())
    } else {
        // Priority 3: Sources disagree - first source's value is selected
        let selected = first_value.clone();
        let alternatives: Vec<(String, String)> = values_with_sources
            .iter()
            .map(|(source, value)| ((*source).clone(), (*value).clone()))
            .collect();

        FieldValue::Conflicting {
            selected,
            alternatives,
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

/// Merge results from multiple sources, prioritizing existing metadata
///
/// Priority order:
/// 1. If existing metadata has a value, use it (user's data takes priority)
/// 2. If all sources agree, return Agreed
/// 3. If sources disagree, return Conflicting (first source's value selected)
/// 4. If no source has value, return Empty
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

    let year_values: Vec<(String, Option<u32>)> = results
        .iter()
        .map(|r| (r.source.clone(), r.year))
        .collect();

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
    fn test_merge_field_existing_takes_priority() {
        let existing = Some("The Martian".to_string());
        let results = vec![
            ("audnexus".to_string(), Some("Martian".to_string())),
            ("openlibrary".to_string(), Some("The Martian: A Novel".to_string())),
        ];

        let result = merge_field(&existing, &results);
        assert_eq!(result, FieldValue::Agreed("The Martian".to_string()));
    }

    #[test]
    fn test_merge_field_sources_agree() {
        let existing = None;
        let results = vec![
            ("audnexus".to_string(), Some("2014".to_string())),
            ("openlibrary".to_string(), Some("2014".to_string())),
        ];

        let result = merge_field(&existing, &results);
        assert_eq!(result, FieldValue::Agreed("2014".to_string()));
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
                assert_eq!(alternatives[0], ("audnexus".to_string(), "2014".to_string()));
                assert_eq!(alternatives[1], ("openlibrary".to_string(), "2011".to_string()));
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
        assert_eq!(result, FieldValue::Agreed("Andy Weir".to_string()));
    }

    #[test]
    fn test_merge_field_u32_converts_to_string() {
        let existing = None;
        let results = vec![
            ("audnexus".to_string(), Some(2014u32)),
            ("openlibrary".to_string(), Some(2014u32)),
        ];

        let result = merge_field_u32(&existing, &results);
        assert_eq!(result, FieldValue::Agreed("2014".to_string()));
    }

    #[test]
    fn test_merge_field_u32_existing_takes_priority() {
        let existing = Some(2015u32);
        let results = vec![
            ("audnexus".to_string(), Some(2014u32)),
            ("openlibrary".to_string(), Some(2014u32)),
        ];

        let result = merge_field_u32(&existing, &results);
        assert_eq!(result, FieldValue::Agreed("2015".to_string()));
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
    fn test_merge_results_existing_metadata_priority() {
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
        assert_eq!(merged.title, FieldValue::Agreed("My Title".to_string()));
        assert_eq!(merged.author, FieldValue::Agreed("My Author".to_string()));
        assert_eq!(merged.year, FieldValue::Agreed("2020".to_string()));
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
        assert_eq!(merged.narrator, FieldValue::Agreed("R.C. Bray".to_string()));

        // ISBN only from openlibrary
        assert_eq!(
            merged.isbn,
            FieldValue::Agreed("978-0553418026".to_string())
        );

        // ASIN only from audnexus
        assert_eq!(merged.asin, FieldValue::Agreed("B00B5HZGUG".to_string()));
    }
}
