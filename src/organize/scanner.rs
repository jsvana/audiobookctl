use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::metadata::{read_metadata, AudiobookMetadata};

/// Information about a scanned audiobook file
#[derive(Debug, Clone)]
pub struct ScannedFile {
    pub path: PathBuf,
    pub filename: String,
    pub metadata: AudiobookMetadata,
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
}
