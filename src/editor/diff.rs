use crate::metadata::AudiobookMetadata;
use std::fmt::Write;

/// A single field change
#[derive(Debug, PartialEq)]
pub struct FieldChange {
    pub field: String,
    pub old_value: String,
    pub new_value: String,
}

/// Compute changes between two metadata structs
pub fn compute_changes(old: &AudiobookMetadata, new: &AudiobookMetadata) -> Vec<FieldChange> {
    let mut changes = Vec::new();

    fn check_string(
        changes: &mut Vec<FieldChange>,
        field: &str,
        old: &Option<String>,
        new: &Option<String>,
    ) {
        let old_val = old.as_deref().unwrap_or("");
        let new_val = new.as_deref().unwrap_or("");
        if old_val != new_val {
            changes.push(FieldChange {
                field: field.to_string(),
                old_value: if old_val.is_empty() {
                    "(empty)".to_string()
                } else {
                    old_val.to_string()
                },
                new_value: if new_val.is_empty() {
                    "(empty)".to_string()
                } else {
                    new_val.to_string()
                },
            });
        }
    }

    fn check_u32(
        changes: &mut Vec<FieldChange>,
        field: &str,
        old: &Option<u32>,
        new: &Option<u32>,
    ) {
        if old != new {
            changes.push(FieldChange {
                field: field.to_string(),
                old_value: old.map_or("(empty)".to_string(), |v| v.to_string()),
                new_value: new.map_or("(empty)".to_string(), |v| v.to_string()),
            });
        }
    }

    check_string(&mut changes, "title", &old.title, &new.title);
    check_string(&mut changes, "author", &old.author, &new.author);
    check_string(&mut changes, "narrator", &old.narrator, &new.narrator);
    check_string(&mut changes, "series", &old.series, &new.series);
    check_u32(
        &mut changes,
        "series_position",
        &old.series_position,
        &new.series_position,
    );
    check_u32(&mut changes, "year", &old.year, &new.year);
    check_string(
        &mut changes,
        "description",
        &old.description,
        &new.description,
    );
    check_string(&mut changes, "publisher", &old.publisher, &new.publisher);
    check_string(&mut changes, "genre", &old.genre, &new.genre);
    check_string(&mut changes, "isbn", &old.isbn, &new.isbn);
    check_string(&mut changes, "asin", &old.asin, &new.asin);

    changes
}

/// Format changes as a side-by-side diff table
pub fn format_diff(file_path: &str, changes: &[FieldChange]) -> String {
    if changes.is_empty() {
        return "No changes detected.".to_string();
    }

    let mut output = String::new();

    writeln!(output, "Changes to {}:", file_path).unwrap();
    writeln!(output).unwrap();

    // Calculate column widths
    let field_width = changes
        .iter()
        .map(|c| c.field.len())
        .max()
        .unwrap_or(10)
        .max(10);
    let value_width = 24;

    // Header
    writeln!(
        output,
        "  {:width$} | {:vw$} | {:vw$}",
        "Field",
        "Current",
        "New",
        width = field_width,
        vw = value_width
    )
    .unwrap();

    // Separator
    writeln!(
        output,
        " {:->width$}-+-{:->vw$}-+-{:->vw$}",
        "",
        "",
        "",
        width = field_width + 1,
        vw = value_width
    )
    .unwrap();

    // Changes
    for change in changes {
        let old_display = truncate_value(&change.old_value, value_width);
        let new_display = truncate_value(&change.new_value, value_width);

        writeln!(
            output,
            "  {:width$} | {:vw$} | {:vw$}",
            change.field,
            old_display,
            new_display,
            width = field_width,
            vw = value_width
        )
        .unwrap();
    }

    output
}

/// Truncate a value to fit in the column width
fn truncate_value(value: &str, max_width: usize) -> String {
    // Replace newlines with spaces for display
    let single_line = value.replace('\n', " ");

    let char_count = single_line.chars().count();

    if char_count <= max_width {
        single_line
    } else {
        let truncated: String = single_line.chars().take(max_width - 3).collect();
        format!("{}...", truncated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_changes_no_changes() {
        let old = AudiobookMetadata::default();
        let new = AudiobookMetadata::default();

        let changes = compute_changes(&old, &new);
        assert!(changes.is_empty());
    }

    #[test]
    fn test_compute_changes_with_changes() {
        let old = AudiobookMetadata {
            title: Some("Old Title".to_string()),
            author: Some("Author".to_string()),
            ..Default::default()
        };

        let new = AudiobookMetadata {
            title: Some("New Title".to_string()),
            author: Some("Author".to_string()),
            narrator: Some("New Narrator".to_string()),
            ..Default::default()
        };

        let changes = compute_changes(&old, &new);

        assert_eq!(changes.len(), 2);
        assert_eq!(changes[0].field, "title");
        assert_eq!(changes[0].old_value, "Old Title");
        assert_eq!(changes[0].new_value, "New Title");
        assert_eq!(changes[1].field, "narrator");
        assert_eq!(changes[1].old_value, "(empty)");
        assert_eq!(changes[1].new_value, "New Narrator");
    }

    #[test]
    fn test_format_diff_empty() {
        let output = format_diff("book.m4b", &[]);
        assert_eq!(output, "No changes detected.");
    }

    #[test]
    fn test_format_diff_with_changes() {
        let changes = vec![FieldChange {
            field: "title".to_string(),
            old_value: "Old".to_string(),
            new_value: "New".to_string(),
        }];

        let output = format_diff("book.m4b", &changes);
        assert!(output.contains("Changes to book.m4b:"));
        assert!(output.contains("title"));
        assert!(output.contains("Old"));
        assert!(output.contains("New"));
    }

    #[test]
    fn test_truncate_value_with_multibyte_utf8() {
        // The bullet character '•' is a 3-byte UTF-8 character
        let value = "NATIONAL BESTSELLER • The definitive history";

        // Should not panic when truncating through multi-byte characters
        let result = super::truncate_value(value, 24);
        assert!(result.ends_with("..."));
        assert_eq!(result.chars().count(), 24);
    }
}
