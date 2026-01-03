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
    scan_directory_with_progress(dir, |_| {})
}

/// Recursively scan a directory for .m4b files and read their metadata,
/// calling progress callback with each file path as it's scanned
pub fn scan_directory_with_progress<F>(dir: &Path, mut on_file: F) -> Result<Vec<ScannedFile>>
where
    F: FnMut(&Path),
{
    let mut files = Vec::new();

    for entry in WalkDir::new(dir)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();

        // Only process .m4b files
        if path.is_file() && is_m4b_file(path) {
            on_file(path);

            let metadata = read_metadata(path)
                .with_context(|| format!("Failed to read metadata from {:?}", path))?;

            let filename = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();

            // Scan for auxiliary files that match this m4b's base name
            let auxiliary_files = scan_auxiliary_files_for(path);

            files.push(ScannedFile {
                path: path.to_path_buf(),
                filename,
                metadata,
                auxiliary_files,
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

/// Scan for auxiliary files that match an m4b file's base name
///
/// For example, if the m4b is "book.m4b", this finds "book.cue", "book.pdf", etc.
/// in the same directory.
fn scan_auxiliary_files_for(m4b_path: &Path) -> Vec<AuxiliaryFile> {
    let mut auxiliary = Vec::new();

    let Some(parent) = m4b_path.parent() else {
        return auxiliary;
    };

    let Some(m4b_stem) = m4b_path.file_stem().map(|s| s.to_string_lossy().to_string()) else {
        return auxiliary;
    };

    // Look for files with the same base name but auxiliary extensions
    for ext in AUXILIARY_EXTENSIONS {
        let aux_filename = format!("{}.{}", m4b_stem, ext);
        let aux_path = parent.join(&aux_filename);

        if aux_path.exists() && aux_path.is_file() {
            auxiliary.push(AuxiliaryFile {
                path: aux_path,
                relative_path: PathBuf::from(&aux_filename),
            });
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
