use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

/// Represents a pending edit waiting to be applied
#[derive(Debug)]
pub struct PendingEdit {
    pub original_path: PathBuf,
    pub toml_content: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Manages the pending edits cache directory
pub struct PendingEditsCache {
    cache_dir: PathBuf,
}

impl PendingEditsCache {
    /// Create a new cache, initializing the directory if needed
    pub fn new() -> Result<Self> {
        let cache_dir = dirs::cache_dir()
            .context("Could not determine cache directory")?
            .join("audiobookctl")
            .join("pending");

        fs::create_dir_all(&cache_dir)
            .with_context(|| format!("Failed to create cache directory: {}", cache_dir.display()))?;

        Ok(Self { cache_dir })
    }

    /// Get the cache file path for a given m4b file
    pub fn cache_path_for(&self, file_path: &Path) -> Result<PathBuf> {
        let abs_path = file_path.canonicalize()
            .with_context(|| format!("Failed to get absolute path for: {}", file_path.display()))?;

        let hash = Self::hash_path(&abs_path);
        Ok(self.cache_dir.join(format!("{}.toml", hash)))
    }

    /// Hash a path to a 16-char hex string
    fn hash_path(path: &Path) -> String {
        let mut hasher = Sha256::new();
        hasher.update(path.to_string_lossy().as_bytes());
        let result = hasher.finalize();
        hex::encode(&result[..8]) // First 8 bytes = 16 hex chars
    }

    /// Check if a pending edit exists for a file
    pub fn has_pending(&self, file_path: &Path) -> Result<bool> {
        let cache_path = self.cache_path_for(file_path)?;
        Ok(cache_path.exists())
    }

    /// Load a pending edit for a file
    pub fn load(&self, file_path: &Path) -> Result<Option<PendingEdit>> {
        let cache_path = self.cache_path_for(file_path)?;

        if !cache_path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&cache_path)
            .with_context(|| format!("Failed to read pending edit: {}", cache_path.display()))?;

        // Parse header comments for metadata
        let mut original_path = file_path.to_path_buf();
        let mut created_at = chrono::Utc::now();
        let mut toml_start = 0;

        for (i, line) in content.lines().enumerate() {
            if line.starts_with("# Pending edit for: ") {
                let path_str = line.trim_start_matches("# Pending edit for: ");
                original_path = PathBuf::from(path_str);
            } else if line.starts_with("# Created: ") {
                let ts_str = line.trim_start_matches("# Created: ");
                if let Ok(ts) = chrono::DateTime::parse_from_rfc3339(ts_str) {
                    created_at = ts.with_timezone(&chrono::Utc);
                }
            } else if !line.starts_with('#') && !line.is_empty() {
                toml_start = content.lines().take(i).map(|l| l.len() + 1).sum();
                break;
            }
        }

        let toml_content = content[toml_start..].to_string();

        Ok(Some(PendingEdit {
            original_path,
            toml_content,
            created_at,
        }))
    }

    /// Save a pending edit for a file
    pub fn save(&self, file_path: &Path, toml_content: &str) -> Result<PathBuf> {
        let cache_path = self.cache_path_for(file_path)?;
        let abs_path = file_path.canonicalize()?;
        let now = chrono::Utc::now();

        let header = format!(
            "# Pending edit for: {}\n# Created: {}\n# Run: audiobookctl edit \"{}\" --no-dry-run\n\n",
            abs_path.display(),
            now.to_rfc3339(),
            abs_path.display()
        );

        let full_content = format!("{}{}", header, toml_content);

        fs::write(&cache_path, full_content)
            .with_context(|| format!("Failed to write pending edit: {}", cache_path.display()))?;

        Ok(cache_path)
    }

    /// Clear pending edit for a specific file
    pub fn clear(&self, file_path: &Path) -> Result<bool> {
        let cache_path = self.cache_path_for(file_path)?;

        if cache_path.exists() {
            fs::remove_file(&cache_path)
                .with_context(|| format!("Failed to remove pending edit: {}", cache_path.display()))?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Clear all pending edits
    pub fn clear_all(&self) -> Result<usize> {
        let mut count = 0;

        if self.cache_dir.exists() {
            for entry in fs::read_dir(&self.cache_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().map_or(false, |e| e == "toml") {
                    fs::remove_file(&path)?;
                    count += 1;
                }
            }
        }

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_path_consistent() {
        let hash1 = PendingEditsCache::hash_path(Path::new("/home/user/book.m4b"));
        let hash2 = PendingEditsCache::hash_path(Path::new("/home/user/book.m4b"));
        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 16);
    }

    #[test]
    fn test_hash_path_different() {
        let hash1 = PendingEditsCache::hash_path(Path::new("/home/user/book1.m4b"));
        let hash2 = PendingEditsCache::hash_path(Path::new("/home/user/book2.m4b"));
        assert_ne!(hash1, hash2);
    }
}
