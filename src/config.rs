// src/config.rs
use serde::Deserialize;
use std::path::Path;
use tokio::fs;

/// Represents the persistent configuration for git2prompt.
/// This file is typically named `.git2promptconfig` (TOML format).
#[derive(Debug, Deserialize, Default, Clone)]
pub struct Config {
    /// List of patterns to ignore (supplementary to .git2promptignore)
    pub ignore_patterns: Option<Vec<String>>,
    /// List of folders to split into separate output files
    pub split_folders: Option<Vec<String>>,
    /// Whether to remove headers (default: false)
    pub no_headers: Option<bool>,
    /// Path to a custom ignore file
    pub ignore_file: Option<String>,
}

impl Config {
    /// Attempt to load configuration from a file.
    /// Returns default config if file doesn't exist or errors.
    pub async fn load_from_file(path: &Path) -> Self {
        if !path.exists() {
            return Config::default();
        }

        match fs::read_to_string(path).await {
            Ok(content) => match toml::from_str(&content) {
                Ok(config) => config,
                Err(e) => {
                    eprintln!("Warning: Failed to parse config file {:?}: {}", path, e);
                    Config::default()
                }
            },
            Err(e) => {
                eprintln!("Warning: Failed to read config file {:?}: {}", path, e);
                Config::default()
            }
        }
    }
}