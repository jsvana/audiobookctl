//! ASIN extraction from filenames
//!
//! Audible audiobook files often contain the ASIN in their filename,
//! which can be used for more accurate API lookups.

use std::path::Path;

/// Extract ASIN from a filename if present.
///
/// Supports common patterns:
/// - `B08G9PRS1K_name.m4b` (ASIN at start with underscore)
/// - `[B08G9PRS1K] name.m4b` (ASIN in brackets)
/// - `name-B08G9PRS1K.m4b` (ASIN before extension with hyphen)
///
/// ASINs are 10 characters, alphanumeric, and typically start with "B0" for audiobooks.
pub fn extract_asin_from_filename(path: &Path) -> Option<String> {
    let filename = path.file_stem()?.to_str()?;

    // Pattern 1: ASIN at start with underscore (B08G9PRS1K_...)
    if let Some(asin) = extract_asin_prefix(filename, '_') {
        return Some(asin);
    }

    // Pattern 2: ASIN in brackets ([B08G9PRS1K] ...)
    if let Some(asin) = extract_asin_brackets(filename) {
        return Some(asin);
    }

    // Pattern 3: ASIN at end with hyphen (...-B08G9PRS1K)
    if let Some(asin) = extract_asin_suffix(filename, '-') {
        return Some(asin);
    }

    None
}

/// Extract ASIN from start of string followed by separator
fn extract_asin_prefix(s: &str, sep: char) -> Option<String> {
    let parts: Vec<&str> = s.splitn(2, sep).collect();
    if parts.len() == 2 && is_valid_asin(parts[0]) {
        Some(parts[0].to_string())
    } else {
        None
    }
}

/// Extract ASIN from brackets at start of string
fn extract_asin_brackets(s: &str) -> Option<String> {
    if !s.starts_with('[') {
        return None;
    }

    let end = s.find(']')?;
    let candidate = &s[1..end];

    if is_valid_asin(candidate) {
        Some(candidate.to_string())
    } else {
        None
    }
}

/// Extract ASIN from end of string preceded by separator
fn extract_asin_suffix(s: &str, sep: char) -> Option<String> {
    let parts: Vec<&str> = s.rsplitn(2, sep).collect();
    if parts.len() == 2 && is_valid_asin(parts[0]) {
        Some(parts[0].to_string())
    } else {
        None
    }
}

/// Check if a string is a valid ASIN
///
/// ASINs are exactly 10 alphanumeric characters, typically starting with "B0"
fn is_valid_asin(s: &str) -> bool {
    s.len() == 10 && s.starts_with("B0") && s.chars().all(|c| c.is_ascii_alphanumeric())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_extract_asin_prefix_underscore() {
        let path = PathBuf::from("B08G9PRS1K_The_Martian.m4b");
        assert_eq!(
            extract_asin_from_filename(&path),
            Some("B08G9PRS1K".to_string())
        );
    }

    #[test]
    fn test_extract_asin_brackets() {
        let path = PathBuf::from("[B08G9PRS1K] The Martian.m4b");
        assert_eq!(
            extract_asin_from_filename(&path),
            Some("B08G9PRS1K".to_string())
        );
    }

    #[test]
    fn test_extract_asin_suffix_hyphen() {
        let path = PathBuf::from("The Martian-B08G9PRS1K.m4b");
        assert_eq!(
            extract_asin_from_filename(&path),
            Some("B08G9PRS1K".to_string())
        );
    }

    #[test]
    fn test_no_asin_in_filename() {
        let path = PathBuf::from("The Martian.m4b");
        assert_eq!(extract_asin_from_filename(&path), None);
    }

    #[test]
    fn test_invalid_asin_wrong_length() {
        let path = PathBuf::from("B08G9_The_Martian.m4b");
        assert_eq!(extract_asin_from_filename(&path), None);
    }

    #[test]
    fn test_invalid_asin_wrong_prefix() {
        let path = PathBuf::from("A08G9PRS1K_The_Martian.m4b");
        assert_eq!(extract_asin_from_filename(&path), None);
    }

    #[test]
    fn test_is_valid_asin() {
        assert!(is_valid_asin("B08G9PRS1K"));
        assert!(is_valid_asin("B00DEKJ8QM"));
        assert!(!is_valid_asin("12345")); // too short
        assert!(!is_valid_asin("A08G9PRS1K")); // wrong prefix
        assert!(!is_valid_asin("B08G9PRS1K!")); // non-alphanumeric
    }
}
