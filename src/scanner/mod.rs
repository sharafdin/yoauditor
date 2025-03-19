//! File scanner for discovering and filtering source files.
//!
//! This module provides a unified file scanner that respects
//! configuration for extensions, excludes, and file size limits.

use anyhow::Result;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, warn};

/// Configuration for file scanning.
#[derive(Debug, Clone)]
pub struct ScanConfig {
    /// File extensions to include (e.g., ["rs", "py", "js"])
    pub extensions: Vec<String>,
    /// Patterns to exclude (e.g., ["node_modules", "target", ".git"])
    pub excludes: Vec<String>,
    /// Maximum file size in bytes
    pub max_file_size: usize,
    /// Maximum number of files to scan
    pub max_files: Option<usize>,
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            extensions: vec![
                "rs", "py", "js", "ts", "jsx", "tsx", "go", "java", "c", "cpp", "h", "hpp",
                "cs", "rb", "php", "swift", "kt", "scala", "vue", "svelte",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
            excludes: vec![
                ".git",
                "target",
                "node_modules",
                "vendor",
                "dist",
                "build",
                "__pycache__",
                ".venv",
                "venv",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
            max_file_size: 100 * 1024, // 100KB
            max_files: None,
        }
    }
}

impl From<&crate::config::ScannerConfig> for ScanConfig {
    fn from(config: &crate::config::ScannerConfig) -> Self {
        Self {
            extensions: config.extensions.clone(),
            excludes: config.excludes.clone(),
            max_file_size: config.max_file_size,
            max_files: Some(config.max_files),
        }
    }
}

/// Scanned file information.
#[derive(Debug, Clone)]
pub struct ScannedFile {
    /// Relative path from repo root
    pub path: String,
    /// File size in bytes
    #[allow(dead_code)] // Metadata for future file filtering/sorting
    pub size: u64,
    /// File extension (without dot)
    #[allow(dead_code)] // Metadata for future file filtering/grouping
    pub extension: String,
}

/// File scanner for discovering source files.
pub struct FileScanner {
    config: ScanConfig,
    repo_root: PathBuf,
}

impl FileScanner {
    /// Create a new file scanner.
    pub fn new(repo_root: PathBuf, config: ScanConfig) -> Self {
        Self { config, repo_root }
    }

    /// Scan for all matching files.
    pub fn scan(&self) -> Result<Vec<ScannedFile>> {
        let mut files = Vec::new();
        self.walk_dir(&self.repo_root, &mut files)?;

        // Apply max_files limit if set
        if let Some(max) = self.config.max_files {
            files.truncate(max);
        }

        Ok(files)
    }

    /// Collect all source files with their contents.
    pub fn collect_files(&self) -> Result<HashMap<String, String>> {
        let scanned = self.scan()?;
        let mut files = HashMap::new();

        for file in scanned {
            let full_path = self.repo_root.join(&file.path);
            match fs::read_to_string(&full_path) {
                Ok(content) => {
                    files.insert(file.path, content);
                }
                Err(e) => {
                    warn!("Failed to read {}: {}", file.path, e);
                }
            }
        }

        Ok(files)
    }

    /// List files in a directory (for tool use).
    pub fn list_directory(&self, relative_dir: &str) -> Result<Vec<String>> {
        let dir_path = self.repo_root.join(relative_dir);

        if !dir_path.exists() {
            return Err(anyhow::anyhow!("Directory not found: {}", relative_dir));
        }

        if !dir_path.is_dir() {
            return Err(anyhow::anyhow!("Not a directory: {}", relative_dir));
        }

        // Security check
        if !self.is_within_repo(&dir_path)? {
            return Err(anyhow::anyhow!("Access denied: path outside repository"));
        }

        let mut entries = Vec::new();
        let dir_entries = fs::read_dir(&dir_path)?;

        for entry in dir_entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();

            // Skip excluded patterns
            if self.is_excluded(&name) {
                continue;
            }

            let suffix = if path.is_dir() { "/" } else { "" };
            entries.push(format!("{}{}", name, suffix));
        }

        entries.sort();
        Ok(entries)
    }

    /// Check if a file matches scan criteria.
    pub fn matches(&self, path: &Path) -> bool {
        // Check if excluded
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if self.is_excluded(name) {
                return false;
            }
        }

        // Check extension
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if !self.config.extensions.contains(&ext.to_string()) {
            return false;
        }

        // Check file size
        if let Ok(metadata) = fs::metadata(path) {
            if metadata.len() > self.config.max_file_size as u64 {
                return false;
            }
        } else {
            return false;
        }

        true
    }

    /// Check if a name matches exclusion patterns.
    fn is_excluded(&self, name: &str) -> bool {
        // Hidden files
        if name.starts_with('.') {
            return true;
        }

        // Explicit excludes
        self.config.excludes.iter().any(|pattern| name == pattern)
    }

    /// Check if a path is within the repository root.
    fn is_within_repo(&self, path: &Path) -> Result<bool> {
        // Canonicalize paths to handle symlinks and ..
        let canonical_repo = fs::canonicalize(&self.repo_root)
            .unwrap_or_else(|_| self.repo_root.clone());
        
        let canonical_path = fs::canonicalize(path)
            .unwrap_or_else(|_| path.to_path_buf());

        Ok(canonical_path.starts_with(canonical_repo))
    }

    /// Walk directory recursively.
    fn walk_dir(&self, dir: &Path, files: &mut Vec<ScannedFile>) -> Result<()> {
        // Check max_files limit
        if let Some(max) = self.config.max_files {
            if files.len() >= max {
                return Ok(());
            }
        }

        if !dir.is_dir() {
            return Ok(());
        }

        let entries = match fs::read_dir(dir) {
            Ok(e) => e,
            Err(e) => {
                debug!("Cannot read directory {}: {}", dir.display(), e);
                return Ok(());
            }
        };

        for entry in entries.flatten() {
            // Check max_files limit again
            if let Some(max) = self.config.max_files {
                if files.len() >= max {
                    break;
                }
            }

            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();

            // Skip excluded
            if self.is_excluded(&name) {
                continue;
            }

            if path.is_dir() {
                self.walk_dir(&path, files)?;
            } else if path.is_file() && self.matches(&path) {
                if let Ok(metadata) = fs::metadata(&path) {
                    let rel_path = path.strip_prefix(&self.repo_root).unwrap_or(&path);
                    let ext = path
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("")
                        .to_string();

                    files.push(ScannedFile {
                        path: rel_path.to_string_lossy().to_string(),
                        size: metadata.len(),
                        extension: ext,
                    });
                }
            }
        }

        Ok(())
    }
}
