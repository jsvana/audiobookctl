use crate::metadata::AudiobookMetadata;
use anyhow::{bail, Result};
use std::path::PathBuf;

/// Available format placeholders with descriptions
pub const PLACEHOLDERS: &[(&str, &str)] = &[
    ("author", "Author name"),
    ("title", "Book title"),
    ("series", "Series name"),
    (
        "series_position",
        "Position in series (supports :02 padding)",
    ),
    ("narrator", "Narrator name"),
    ("year", "Publication year"),
    ("genre", "Genre"),
    ("publisher", "Publisher"),
    ("asin", "Amazon ASIN"),
    ("isbn", "ISBN"),
    ("filename", "Original filename"),
];

/// A parsed format string with placeholder segments
#[derive(Debug, Clone)]
pub struct FormatTemplate {
    segments: Vec<Segment>,
}

#[derive(Debug, Clone)]
enum Segment {
    Literal(String),
    Placeholder {
        name: String,
        padding: Option<usize>,
    },
}

impl FormatTemplate {
    /// Parse a format string like "{author}/{series}/{title}/{filename}"
    pub fn parse(format: &str) -> Result<Self> {
        let mut segments = Vec::new();
        let mut chars = format.chars().peekable();
        let mut literal = String::new();

        while let Some(c) = chars.next() {
            if c == '{' {
                // Save any accumulated literal
                if !literal.is_empty() {
                    segments.push(Segment::Literal(std::mem::take(&mut literal)));
                }

                // Parse placeholder name and optional padding
                let mut placeholder = String::new();
                let mut found_close = false;

                for inner in chars.by_ref() {
                    if inner == '}' {
                        found_close = true;
                        break;
                    }
                    placeholder.push(inner);
                }

                if !found_close {
                    bail!("Unclosed placeholder '{{' in format string");
                }

                // Parse optional padding (e.g., "series_position:02")
                let (name, padding) = if let Some(colon_pos) = placeholder.find(':') {
                    let name = placeholder[..colon_pos].to_string();
                    let pad_str = &placeholder[colon_pos + 1..];
                    let padding = pad_str.parse::<usize>().ok();
                    (name, padding)
                } else {
                    (placeholder, None)
                };

                // Validate placeholder name
                let valid_names: Vec<&str> = PLACEHOLDERS.iter().map(|(n, _)| *n).collect();
                if !valid_names.contains(&name.as_str()) {
                    bail!(
                        "Unknown placeholder '{}'. Valid placeholders: {}",
                        name,
                        valid_names.join(", ")
                    );
                }

                segments.push(Segment::Placeholder { name, padding });
            } else {
                literal.push(c);
            }
        }

        // Save any remaining literal
        if !literal.is_empty() {
            segments.push(Segment::Literal(literal));
        }

        Ok(Self { segments })
    }

    /// Get the list of required placeholder names (excluding 'filename' which is always available)
    pub fn required_fields(&self) -> Vec<String> {
        self.segments
            .iter()
            .filter_map(|s| {
                if let Segment::Placeholder { name, .. } = s {
                    if name != "filename" {
                        return Some(name.clone());
                    }
                }
                None
            })
            .collect()
    }

    /// Generate a path from metadata and original filename
    /// Returns None for any missing required field, along with the list of missing fields
    pub fn generate_path(
        &self,
        metadata: &AudiobookMetadata,
        original_filename: &str,
    ) -> Result<PathBuf, Vec<String>> {
        let mut missing = Vec::new();
        let mut path_parts = Vec::new();
        let mut current_part = String::new();

        for segment in &self.segments {
            match segment {
                Segment::Literal(s) => {
                    if s.contains('/') || s.contains(std::path::MAIN_SEPARATOR) {
                        // Split on path separator
                        for (i, part) in s.split(['/', std::path::MAIN_SEPARATOR]).enumerate() {
                            if i > 0 {
                                // Push the current accumulated part as a path component
                                if !current_part.is_empty() {
                                    path_parts.push(std::mem::take(&mut current_part));
                                }
                            }
                            current_part.push_str(part);
                        }
                    } else {
                        current_part.push_str(s);
                    }
                }
                Segment::Placeholder { name, padding } => {
                    let value = self.get_field_value(metadata, name, original_filename);
                    match value {
                        Some(v) => {
                            let formatted = if let Some(pad) = padding {
                                format!("{:0>width$}", v, width = *pad)
                            } else {
                                v
                            };
                            // Sanitize for filesystem
                            let sanitized = sanitize_path_component(&formatted);
                            current_part.push_str(&sanitized);
                        }
                        None => {
                            if name != "filename" {
                                missing.push(name.clone());
                            }
                            // Use placeholder text for now (will fail later if missing)
                            current_part.push_str(&format!("{{{}}}", name));
                        }
                    }
                }
            }
        }

        // Push the final part
        if !current_part.is_empty() {
            path_parts.push(current_part);
        }

        if !missing.is_empty() {
            return Err(missing);
        }

        // Build the path
        let mut path = PathBuf::new();
        for part in path_parts {
            path.push(part);
        }

        Ok(path)
    }

    fn get_field_value(
        &self,
        metadata: &AudiobookMetadata,
        name: &str,
        original_filename: &str,
    ) -> Option<String> {
        match name {
            "author" => metadata.author.clone(),
            "title" => metadata.title.clone(),
            "series" => metadata.series.clone(),
            "series_position" => metadata.series_position.map(|n| n.to_string()),
            "narrator" => metadata.narrator.clone(),
            "year" => metadata.year.map(|n| n.to_string()),
            "genre" => metadata.genre.clone(),
            "publisher" => metadata.publisher.clone(),
            "asin" => metadata.asin.clone(),
            "isbn" => metadata.isbn.clone(),
            "filename" => Some(original_filename.to_string()),
            _ => None,
        }
    }
}

/// Sanitize a string for use as a path component
/// Removes/replaces characters that are problematic on filesystems
fn sanitize_path_component(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            // Replace problematic characters with safe alternatives
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            // Keep most other characters
            _ => c,
        })
        .collect::<String>()
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_metadata() -> AudiobookMetadata {
        AudiobookMetadata {
            title: Some("Project Hail Mary".to_string()),
            author: Some("Andy Weir".to_string()),
            series: Some("Standalone".to_string()),
            series_position: Some(1),
            ..Default::default()
        }
    }

    #[test]
    fn test_parse_simple_format() {
        let template = FormatTemplate::parse("{author}/{title}/{filename}").unwrap();
        assert_eq!(template.required_fields(), vec!["author", "title"]);
    }

    #[test]
    fn test_parse_with_padding() {
        let template = FormatTemplate::parse("{series}/{series_position:02}/{filename}").unwrap();
        let metadata = sample_metadata();
        let path = template.generate_path(&metadata, "book.m4b").unwrap();
        assert_eq!(path, PathBuf::from("Standalone/01/book.m4b"));
    }

    #[test]
    fn test_generate_path_basic() {
        let template = FormatTemplate::parse("{author}/{title}/{filename}").unwrap();
        let metadata = sample_metadata();
        let path = template.generate_path(&metadata, "book.m4b").unwrap();
        assert_eq!(path, PathBuf::from("Andy Weir/Project Hail Mary/book.m4b"));
    }

    #[test]
    fn test_missing_field() {
        let template = FormatTemplate::parse("{author}/{narrator}/{filename}").unwrap();
        let metadata = sample_metadata(); // narrator is None
        let result = template.generate_path(&metadata, "book.m4b");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), vec!["narrator"]);
    }

    #[test]
    fn test_invalid_placeholder() {
        let result = FormatTemplate::parse("{author}/{invalid}/{filename}");
        assert!(result.is_err());
    }

    #[test]
    fn test_sanitize_path_component() {
        assert_eq!(sanitize_path_component("Hello: World"), "Hello_ World");
        assert_eq!(sanitize_path_component("Book/Part 1"), "Book_Part 1");
    }
}
