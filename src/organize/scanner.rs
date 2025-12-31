use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::metadata::{read_metadata, AudiobookMetadata};

/// Auxiliary file discovered alongside an m4b (e.g., .cue, .pdf)
#[derive(Debug, Clone)]
pub struct AuxiliaryFile {
    /// Absolute path on disk
    pub path: PathBuf,
    /// Path relative to the m4b's parent directory
    pub relative_path: PathBuf,
}

/// Extensions recognized as auxiliary files
const AUXILIARY_EXTENSIONS: &[&str] = &["cue", "pdf"];

/// Information about a scanned audiobook file
#[derive(Debug, Clone)]
pub struct ScannedFile {
    pub path: PathBuf,
    pub filename: String,
    pub metadata: AudiobookMetadata,
    /// Auxiliary files found in the same directory tree
    pub auxiliary_files: Vec<AuxiliaryFile>,
}

/// Recursively scan a directory for .m4b files and read their metadata
pub fn scan_directory(dir: &Path) -> Result<Vec<ScannedFile>> {
    let mut files = Vec::new();

    for entry in WalkDir::new(dir)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();

        // Only process .m4b files
        if path.is_file() && is_m4b_file(path) {
            let metadata = read_metadata(path)
                .with_context(|| format!("Failed to read metadata from {:?}", path))?;

            let filename = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();

            files.push(ScannedFile {
                path: path.to_path_buf(),
                filename,
                metadata,
                auxiliary_files: Vec::new(),
            });
        }
    }

    // Sort by path for consistent output
    files.sort_by(|a, b| a.path.cmp(&b.path));

    Ok(files)
}

/// Check if a path is an m4b file
fn is_m4b_file(path: &Path) -> bool {
    path.extension()
        .map(|ext| ext.to_string_lossy().to_lowercase() == "m4b")
        .unwrap_or(false)
}

/// Check if a path is an auxiliary file
fn is_auxiliary_file(path: &Path) -> bool {
    path.extension()
        .map(|ext| {
            let ext_lower = ext.to_string_lossy().to_lowercase();
            AUXILIARY_EXTENSIONS.contains(&ext_lower.as_str())
        })
        .unwrap_or(false)
}

/// Scan for auxiliary files in a directory and its subdirectories
fn scan_auxiliary_files(m4b_dir: &Path) -> Vec<AuxiliaryFile> {
    let mut auxiliary = Vec::new();

    for entry in WalkDir::new(m4b_dir)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();

        if path.is_file() && is_auxiliary_file(path) {
            if let Ok(relative) = path.strip_prefix(m4b_dir) {
                auxiliary.push(AuxiliaryFile {
                    path: path.to_path_buf(),
                    relative_path: relative.to_path_buf(),
                });
            }
        }
    }

    // Sort for consistent output
    auxiliary.sort_by(|a, b| a.path.cmp(&b.path));
    auxiliary
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_m4b_file() {
        assert!(is_m4b_file(Path::new("/path/to/book.m4b")));
        assert!(is_m4b_file(Path::new("/path/to/book.M4B")));
        assert!(!is_m4b_file(Path::new("/path/to/book.mp3")));
        assert!(!is_m4b_file(Path::new("/path/to/book")));
    }

    #[test]
    fn test_is_auxiliary_file() {
        assert!(is_auxiliary_file(Path::new("/path/to/book.cue")));
        assert!(is_auxiliary_file(Path::new("/path/to/notes.pdf")));
        assert!(is_auxiliary_file(Path::new("/path/to/NOTES.PDF")));
        assert!(!is_auxiliary_file(Path::new("/path/to/book.m4b")));
        assert!(!is_auxiliary_file(Path::new("/path/to/book.mp3")));
    }
}
