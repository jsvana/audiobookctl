//! SHA256 file hashing utilities

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{BufReader, Read, Write};
use std::path::{Path, PathBuf};

/// Compute SHA256 hash of a file, streaming to avoid loading into memory
pub fn sha256_file(path: &Path) -> Result<String> {
    let file = File::open(path).with_context(|| format!("Failed to open {:?}", path))?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];

    loop {
        let bytes_read = reader
            .read(&mut buffer)
            .with_context(|| format!("Failed to read {:?}", path))?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(hex::encode(hasher.finalize()))
}

/// Get the path to the hash file for an m4b file
pub fn hash_file_path(m4b_path: &Path) -> PathBuf {
    let mut hash_path = m4b_path.as_os_str().to_owned();
    hash_path.push(".sha256");
    PathBuf::from(hash_path)
}

/// Read a cached hash from a .sha256 file
pub fn read_hash_file(m4b_path: &Path) -> Result<Option<String>> {
    let hash_path = hash_file_path(m4b_path);
    if !hash_path.exists() {
        return Ok(None);
    }

    let contents = std::fs::read_to_string(&hash_path)
        .with_context(|| format!("Failed to read hash file {:?}", hash_path))?;

    let hash = contents.trim().to_string();

    // Validate it looks like a SHA256 hash (64 hex chars)
    if hash.len() == 64 && hash.chars().all(|c| c.is_ascii_hexdigit()) {
        Ok(Some(hash))
    } else {
        Ok(None) // Invalid format, treat as missing
    }
}

/// Write a hash to a .sha256 file
pub fn write_hash_file(m4b_path: &Path, hash: &str) -> Result<()> {
    let hash_path = hash_file_path(m4b_path);
    let mut file =
        File::create(&hash_path).with_context(|| format!("Failed to create {:?}", hash_path))?;
    writeln!(file, "{}", hash).with_context(|| format!("Failed to write {:?}", hash_path))?;
    Ok(())
}

/// Get hash for a file, using cache if available, computing otherwise
///
/// If `write_cache` is true and hash is computed, writes it to the cache file.
pub fn get_hash(m4b_path: &Path, write_cache: bool) -> Result<String> {
    // Try to read from cache first
    if let Some(cached) = read_hash_file(m4b_path)? {
        return Ok(cached);
    }

    // Compute hash
    let hash = sha256_file(m4b_path)?;

    // Write to cache if requested
    if write_cache {
        write_hash_file(m4b_path, &hash)?;
    }

    Ok(hash)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_sha256_known_content() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"hello world").unwrap();
        file.flush().unwrap();

        let hash = sha256_file(file.path()).unwrap();
        // SHA256 of "hello world"
        assert_eq!(
            hash,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn test_sha256_empty_file() {
        let file = NamedTempFile::new().unwrap();
        let hash = sha256_file(file.path()).unwrap();
        // SHA256 of empty string
        assert_eq!(
            hash,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn test_hash_file_path() {
        let path = Path::new("/foo/bar/book.m4b");
        assert_eq!(
            hash_file_path(path),
            PathBuf::from("/foo/bar/book.m4b.sha256")
        );
    }

    #[test]
    fn test_read_hash_file_missing() {
        let file = NamedTempFile::new().unwrap();
        let result = read_hash_file(file.path()).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_write_and_read_hash_file() {
        let dir = tempfile::tempdir().unwrap();
        let m4b_path = dir.path().join("book.m4b");
        std::fs::write(&m4b_path, b"test").unwrap();

        let hash = "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9";
        write_hash_file(&m4b_path, hash).unwrap();

        let read = read_hash_file(&m4b_path).unwrap();
        assert_eq!(read, Some(hash.to_string()));
    }

    #[test]
    fn test_read_hash_file_invalid_format() {
        let dir = tempfile::tempdir().unwrap();
        let m4b_path = dir.path().join("book.m4b");
        std::fs::write(&m4b_path, b"test").unwrap();

        let hash_path = hash_file_path(&m4b_path);
        std::fs::write(&hash_path, "not a valid hash").unwrap();

        let result = read_hash_file(&m4b_path).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_get_hash_uses_cache() {
        let dir = tempfile::tempdir().unwrap();
        let m4b_path = dir.path().join("book.m4b");
        std::fs::write(&m4b_path, b"hello world").unwrap();

        // Write a fake cached hash (different from actual)
        let fake_hash = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        write_hash_file(&m4b_path, fake_hash).unwrap();

        // Should return cached hash, not compute
        let result = get_hash(&m4b_path, false).unwrap();
        assert_eq!(result, fake_hash);
    }

    #[test]
    fn test_get_hash_computes_when_no_cache() {
        let dir = tempfile::tempdir().unwrap();
        let m4b_path = dir.path().join("book.m4b");
        std::fs::write(&m4b_path, b"hello world").unwrap();

        let result = get_hash(&m4b_path, false).unwrap();
        assert_eq!(
            result,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn test_get_hash_writes_cache() {
        let dir = tempfile::tempdir().unwrap();
        let m4b_path = dir.path().join("book.m4b");
        std::fs::write(&m4b_path, b"hello world").unwrap();

        // Compute and write cache
        let result = get_hash(&m4b_path, true).unwrap();
        assert_eq!(
            result,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );

        // Verify cache file was created
        let hash_path = hash_file_path(&m4b_path);
        assert!(hash_path.exists());
    }
}
