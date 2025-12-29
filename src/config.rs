use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Application configuration loaded from ~/.config/audiobookctl/config.toml
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub organize: OrganizeConfig,
    #[serde(default)]
    pub backups: BackupsConfig,
}

/// Configuration for the organize and fix commands
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OrganizeConfig {
    /// Format string for organizing audiobooks
    /// Example: "{author}/{series}/{title}/{filename}"
    pub format: Option<String>,

    /// Default destination directory for organized audiobooks
    pub dest: Option<PathBuf>,
}

/// Configuration for backup management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupsConfig {
    /// Maximum storage allowed for backups in bytes (default: 2GB)
    #[serde(default = "default_max_storage")]
    pub max_storage_bytes: u64,
}

fn default_max_storage() -> u64 {
    2 * 1024 * 1024 * 1024 // 2GB
}

impl Default for BackupsConfig {
    fn default() -> Self {
        Self {
            max_storage_bytes: default_max_storage(),
        }
    }
}

impl Config {
    /// Load configuration from the default path (~/.config/audiobookctl/config.toml)
    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;
        Self::load_from(&path)
    }

    /// Load configuration from a specific path
    pub fn load_from(path: &PathBuf) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let content =
            std::fs::read_to_string(path).with_context(|| format!("Failed to read {:?}", path))?;

        toml::from_str(&content).with_context(|| format!("Failed to parse {:?}", path))
    }

    /// Get the default config file path
    pub fn config_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir().context("Could not determine config directory")?;
        Ok(config_dir.join("audiobookctl").join("config.toml"))
    }

    /// Get the format string, with CLI override taking precedence
    pub fn format(&self, cli_override: Option<&str>) -> Option<String> {
        cli_override
            .map(String::from)
            .or_else(|| self.organize.format.clone())
    }

    /// Get the destination path, with CLI override taking precedence
    pub fn dest(&self, cli_override: Option<&PathBuf>) -> Option<PathBuf> {
        cli_override.cloned().or_else(|| self.organize.dest.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_load_missing_config() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("config.toml");
        let config = Config::load_from(&path).unwrap();
        assert!(config.organize.format.is_none());
        assert!(config.organize.dest.is_none());
    }

    #[test]
    fn test_load_valid_config() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("config.toml");
        std::fs::write(
            &path,
            r#"
[organize]
format = "{author}/{title}/{filename}"
dest = "/home/user/audiobooks"
"#,
        )
        .unwrap();

        let config = Config::load_from(&path).unwrap();
        assert_eq!(
            config.organize.format,
            Some("{author}/{title}/{filename}".to_string())
        );
        assert_eq!(
            config.organize.dest,
            Some(PathBuf::from("/home/user/audiobooks"))
        );
    }

    #[test]
    fn test_cli_override() {
        let config = Config {
            organize: OrganizeConfig {
                format: Some("{author}/{title}".to_string()),
                dest: Some(PathBuf::from("/default/path")),
            },
            backups: BackupsConfig::default(),
        };

        // CLI override takes precedence
        assert_eq!(
            config.format(Some("{custom}/{format}")),
            Some("{custom}/{format}".to_string())
        );
        assert_eq!(
            config.dest(Some(&PathBuf::from("/cli/path"))),
            Some(PathBuf::from("/cli/path"))
        );

        // Falls back to config when no CLI override
        assert_eq!(config.format(None), Some("{author}/{title}".to_string()));
        assert_eq!(config.dest(None), Some(PathBuf::from("/default/path")));
    }

    #[test]
    fn test_backups_config_defaults() {
        let config = Config::default();
        assert_eq!(config.backups.max_storage_bytes, 2 * 1024 * 1024 * 1024);
    }

    #[test]
    fn test_load_with_backups_config() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("config.toml");
        std::fs::write(
            &path,
            r#"
[backups]
max_storage_bytes = 1073741824
"#,
        )
        .unwrap();

        let config = Config::load_from(&path).unwrap();
        assert_eq!(config.backups.max_storage_bytes, 1024 * 1024 * 1024); // 1GB
    }
}
