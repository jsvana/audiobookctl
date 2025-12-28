use crate::metadata::AudiobookMetadata;
use anyhow::Result;

/// Convert metadata to TOML string with comments for empty/read-only fields
pub fn metadata_to_toml(metadata: &AudiobookMetadata) -> String {
    let mut lines = Vec::new();

    lines.push("# Audiobook Metadata - Edit and save to apply changes".to_string());
    lines.push("# Commented fields are empty - uncomment and fill to add values".to_string());
    lines.push(String::new());

    // Helper to add field
    fn add_field(lines: &mut Vec<String>, name: &str, value: &Option<String>) {
        match value {
            Some(v) => lines.push(format!("{} = \"{}\"", name, escape_toml_string(v))),
            None => lines.push(format!("# {} = \"\"", name)),
        }
    }

    fn add_field_u32(lines: &mut Vec<String>, name: &str, value: &Option<u32>) {
        match value {
            Some(v) => lines.push(format!("{} = {}", name, v)),
            None => lines.push(format!("# {} = 0", name)),
        }
    }

    add_field(&mut lines, "title", &metadata.title);
    add_field(&mut lines, "author", &metadata.author);
    add_field(&mut lines, "narrator", &metadata.narrator);
    add_field(&mut lines, "series", &metadata.series);
    add_field_u32(&mut lines, "series_position", &metadata.series_position);
    add_field_u32(&mut lines, "year", &metadata.year);
    add_field(&mut lines, "description", &metadata.description);
    add_field(&mut lines, "publisher", &metadata.publisher);
    add_field(&mut lines, "genre", &metadata.genre);
    add_field(&mut lines, "isbn", &metadata.isbn);
    add_field(&mut lines, "asin", &metadata.asin);

    // Read-only section
    lines.push(String::new());
    lines.push("# Read-only (cannot be edited)".to_string());

    if let Some(duration) = metadata.duration_seconds {
        let hours = duration / 3600;
        let minutes = (duration % 3600) / 60;
        let seconds = duration % 60;
        lines.push(format!("# duration = \"{:02}:{:02}:{:02}\"", hours, minutes, seconds));
    } else {
        lines.push("# duration = \"\"".to_string());
    }

    if let Some(chapters) = metadata.chapter_count {
        lines.push(format!("# chapters = {}", chapters));
    } else {
        lines.push("# chapters = 0".to_string());
    }

    if let Some(ref cover) = metadata.cover_info {
        lines.push(format!("# cover = \"{}\"", cover));
    } else {
        lines.push("# cover = \"\"".to_string());
    }

    lines.push(String::new());
    lines.join("\n")
}

/// Parse TOML string back to metadata
pub fn toml_to_metadata(toml_str: &str) -> Result<AudiobookMetadata> {
    // Filter out comment lines and parse remaining as TOML
    let filtered: String = toml_str
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n");

    // Parse into a toml::Value first
    let value: toml::Value = toml::from_str(&filtered)?;
    let table = value.as_table().ok_or_else(|| anyhow::anyhow!("Invalid TOML structure"))?;

    fn get_string(table: &toml::map::Map<String, toml::Value>, key: &str) -> Option<String> {
        table.get(key).and_then(|v| v.as_str()).map(String::from)
    }

    fn get_u32(table: &toml::map::Map<String, toml::Value>, key: &str) -> Option<u32> {
        table.get(key).and_then(|v| v.as_integer()).map(|n| n as u32)
    }

    Ok(AudiobookMetadata {
        title: get_string(table, "title"),
        author: get_string(table, "author"),
        narrator: get_string(table, "narrator"),
        series: get_string(table, "series"),
        series_position: get_u32(table, "series_position"),
        year: get_u32(table, "year"),
        description: get_string(table, "description"),
        publisher: get_string(table, "publisher"),
        genre: get_string(table, "genre"),
        isbn: get_string(table, "isbn"),
        asin: get_string(table, "asin"),
        // Read-only fields preserved as None (will be kept from original when writing)
        duration_seconds: None,
        chapter_count: None,
        cover_info: None,
    })
}

/// Escape special characters in TOML strings
fn escape_toml_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata_to_toml_with_values() {
        let metadata = AudiobookMetadata {
            title: Some("Test Book".to_string()),
            author: Some("Test Author".to_string()),
            narrator: None,
            series: Some("Test Series".to_string()),
            series_position: Some(1),
            year: Some(2024),
            description: Some("A test description".to_string()),
            publisher: None,
            genre: Some("Fiction".to_string()),
            isbn: None,
            asin: None,
            duration_seconds: Some(3661),
            chapter_count: Some(10),
            cover_info: Some("embedded (1000 bytes, JPEG)".to_string()),
        };

        let toml = metadata_to_toml(&metadata);

        assert!(toml.contains("title = \"Test Book\""));
        assert!(toml.contains("author = \"Test Author\""));
        assert!(toml.contains("# narrator = \"\""));
        assert!(toml.contains("series = \"Test Series\""));
        assert!(toml.contains("series_position = 1"));
        assert!(toml.contains("# duration = \"01:01:01\""));
    }

    #[test]
    fn test_toml_to_metadata() {
        let toml = r#"
title = "Parsed Book"
author = "Parsed Author"
year = 2023
"#;

        let metadata = toml_to_metadata(toml).unwrap();

        assert_eq!(metadata.title, Some("Parsed Book".to_string()));
        assert_eq!(metadata.author, Some("Parsed Author".to_string()));
        assert_eq!(metadata.year, Some(2023));
        assert_eq!(metadata.narrator, None);
    }

    #[test]
    fn test_roundtrip() {
        let original = AudiobookMetadata {
            title: Some("Roundtrip Test".to_string()),
            author: Some("Test Author".to_string()),
            narrator: Some("Test Narrator".to_string()),
            series: None,
            series_position: None,
            year: Some(2024),
            description: Some("Description with \"quotes\"".to_string()),
            publisher: None,
            genre: None,
            isbn: Some("123-456".to_string()),
            asin: None,
            duration_seconds: None,
            chapter_count: None,
            cover_info: None,
        };

        let toml = metadata_to_toml(&original);
        let parsed = toml_to_metadata(&toml).unwrap();

        assert_eq!(parsed.title, original.title);
        assert_eq!(parsed.author, original.author);
        assert_eq!(parsed.narrator, original.narrator);
        assert_eq!(parsed.year, original.year);
        assert_eq!(parsed.isbn, original.isbn);
    }
}
