use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Create a backup of a file before modifying it
pub fn create_backup(file_path: &Path) -> Result<PathBuf> {
    let backup_path = backup_path_for(file_path);

    fs::copy(file_path, &backup_path).with_context(|| {
        format!(
            "Failed to create backup: {} -> {}",
            file_path.display(),
            backup_path.display()
        )
    })?;

    Ok(backup_path)
}

/// Get the backup path for a file
pub fn backup_path_for(file_path: &Path) -> PathBuf {
    let mut backup = file_path.to_path_buf();
    let mut name = backup.file_name().unwrap_or_default().to_os_string();
    name.push(".bak");
    backup.set_file_name(name);
    backup
}

/// Check if a backup exists for a file
pub fn has_backup(file_path: &Path) -> bool {
    backup_path_for(file_path).exists()
}

/// Delete the backup for a specific file
pub fn delete_backup(file_path: &Path) -> Result<bool> {
    let backup_path = backup_path_for(file_path);

    if backup_path.exists() {
        fs::remove_file(&backup_path)
            .with_context(|| format!("Failed to delete backup: {}", backup_path.display()))?;
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Information about a backup file
#[derive(Debug)]
pub struct BackupInfo {
    pub backup_path: PathBuf,
    pub original_path: PathBuf,
    pub size_bytes: u64,
}

/// Find all backup files in a directory recursively
pub fn find_all_backups(dir: &Path) -> Result<Vec<BackupInfo>> {
    let mut backups = Vec::new();

    for entry in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();

        if path.extension().is_some_and(|e| e == "bak") {
            // Check if it's an m4b backup
            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            if stem.ends_with(".m4b") {
                let original = path.with_file_name(stem);
                let size = fs::metadata(path).map(|m| m.len()).unwrap_or(0);

                backups.push(BackupInfo {
                    backup_path: path.to_path_buf(),
                    original_path: original,
                    size_bytes: size,
                });
            }
        }
    }

    Ok(backups)
}

/// Format bytes as human-readable size
pub fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.0} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.0} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} bytes", bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backup_path_for() {
        let path = Path::new("/home/user/book.m4b");
        let backup = backup_path_for(path);
        assert_eq!(backup, PathBuf::from("/home/user/book.m4b.bak"));
    }

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(500), "500 bytes");
        assert_eq!(format_size(1024), "1 KB");
        assert_eq!(format_size(1024 * 1024), "1 MB");
        assert_eq!(format_size(1024 * 1024 * 1024), "1.0 GB");
        assert_eq!(format_size(523 * 1024 * 1024), "523 MB");
    }
}
