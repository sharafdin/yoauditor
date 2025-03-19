//! Git repository cloning functionality.
//!
//! This module handles cloning GitHub repositories to local temporary directories
//! using the git2 library.

use anyhow::{Context, Result};
use git2::{FetchOptions, Progress, RemoteCallbacks, Repository};
use indicatif::{ProgressBar, ProgressStyle};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tempfile::TempDir;
use tracing::{debug, info, warn};

/// Result of a successful clone operation.
pub struct CloneResult {
    /// The cloned repository.
    #[allow(dead_code)] // Keep for future direct repo operations
    pub repo: Repository,
    /// Path to the cloned repository.
    pub path: PathBuf,
    /// Temporary directory handle (keeps the directory alive).
    /// If None, the repository was cloned to a persistent location.
    pub temp_dir: Option<TempDir>,
}

impl CloneResult {
    /// Get the path to the repository root.
    #[allow(dead_code)] // Utility accessor (path field is used directly)
    pub fn repo_path(&self) -> &Path {
        &self.path
    }

    /// Consume the CloneResult and return the path, optionally keeping the temp directory.
    pub fn into_path(self) -> PathBuf {
        // If we have a temp dir, we need to persist it by forgetting the TempDir
        if let Some(temp_dir) = self.temp_dir {
            let path = temp_dir.path().to_path_buf();
            let _ = temp_dir.keep();
            path
        } else {
            self.path
        }
    }
}

/// Options for cloning a repository.
#[derive(Debug, Clone)]
pub struct CloneOptions {
    /// Branch to checkout (None for default branch).
    pub branch: Option<String>,
    /// Depth for shallow clone (None for full clone).
    pub depth: Option<i32>,
    /// Whether to show progress.
    pub show_progress: bool,
    /// Target directory (None for temp directory).
    pub target_dir: Option<PathBuf>,
}

impl Default for CloneOptions {
    fn default() -> Self {
        Self {
            branch: None,
            depth: Some(1), // Shallow clone by default for speed
            show_progress: true,
            target_dir: None,
        }
    }
}

/// Clone a repository from a URL.
pub fn clone_repository(url: &str, options: CloneOptions) -> Result<CloneResult> {
    info!("Cloning repository: {}", url);

    // Determine the target path
    let (path, temp_dir) = if let Some(target) = options.target_dir {
        if target.exists() {
            debug!("Target directory already exists: {}", target.display());
            // Try to open existing repository
            if let Ok(repo) = Repository::open(&target) {
                info!("Using existing repository at: {}", target.display());
                return Ok(CloneResult {
                    repo,
                    path: target,
                    temp_dir: None,
                });
            }
        }
        (target, None)
    } else {
        let temp = TempDir::new().context("Failed to create temporary directory")?;
        let path = temp.path().to_path_buf();
        (path, Some(temp))
    };

    debug!("Clone target: {}", path.display());

    // Set up progress callback
    let progress_bar = if options.show_progress {
        let pb = ProgressBar::new(0);
        pb.set_style(
            ProgressStyle::default_bar()
                .template(
                    "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})",
                )
                .unwrap()
                .progress_chars("#>-"),
        );
        Some(Arc::new(pb))
    } else {
        None
    };

    let pb_clone = progress_bar.clone();
    let mut callbacks = RemoteCallbacks::new();

    callbacks.transfer_progress(move |progress: Progress<'_>| {
        if let Some(ref pb) = pb_clone {
            pb.set_length(progress.total_objects() as u64);
            pb.set_position(progress.received_objects() as u64);
        }
        true
    });

    // Set up fetch options
    let mut fetch_opts = FetchOptions::new();
    fetch_opts.remote_callbacks(callbacks);

    if let Some(depth) = options.depth {
        fetch_opts.depth(depth);
    }

    // Build the repository
    let mut builder = git2::build::RepoBuilder::new();
    builder.fetch_options(fetch_opts);

    if let Some(ref branch) = options.branch {
        builder.branch(branch);
    }

    // Perform the clone
    let repo = builder
        .clone(url, &path)
        .with_context(|| format!("Failed to clone repository: {}", url))?;

    if let Some(pb) = progress_bar {
        pb.finish_with_message("Clone complete");
    }

    info!("Successfully cloned repository to: {}", path.display());

    Ok(CloneResult {
        repo,
        path,
        temp_dir,
    })
}

/// Open an existing local repository.
#[allow(dead_code)] // Utility for opening existing repos
pub fn open_local_repository(path: &Path) -> Result<CloneResult> {
    info!("Opening local repository: {}", path.display());

    if !path.exists() {
        anyhow::bail!("Repository path does not exist: {}", path.display());
    }

    let repo = Repository::open(path)
        .with_context(|| format!("Failed to open repository: {}", path.display()))?;

    Ok(CloneResult {
        repo,
        path: path.to_path_buf(),
        temp_dir: None,
    })
}

/// Parse a GitHub URL to extract owner and repo name.
#[allow(dead_code)] // Utility for URL parsing
pub fn parse_github_url(url: &str) -> Option<(String, String)> {
    // Handle various GitHub URL formats
    let url = url.trim_end_matches(".git");

    // https://github.com/owner/repo
    if let Some(rest) = url.strip_prefix("https://github.com/") {
        let parts: Vec<&str> = rest.split('/').collect();
        if parts.len() >= 2 {
            return Some((parts[0].to_string(), parts[1].to_string()));
        }
    }

    // git@github.com:owner/repo
    if let Some(rest) = url.strip_prefix("git@github.com:") {
        let parts: Vec<&str> = rest.split('/').collect();
        if parts.len() >= 2 {
            return Some((parts[0].to_string(), parts[1].to_string()));
        }
    }

    None
}

/// Get the current branch name of a repository.
#[allow(dead_code)] // Utility for repo inspection
pub fn get_current_branch(repo: &Repository) -> Option<String> {
    repo.head()
        .ok()
        .and_then(|head| head.shorthand().map(String::from))
}

/// Get the current commit hash (short form).
#[allow(dead_code)] // Utility for repo inspection
pub fn get_current_commit(repo: &Repository) -> Option<String> {
    repo.head()
        .ok()
        .and_then(|head| head.peel_to_commit().ok())
        .map(|commit| commit.id().to_string()[..8].to_string())
}

/// Clean up a cloned repository.
#[allow(dead_code)] // Utility for cleanup
pub fn cleanup_repository(clone_result: CloneResult) -> Result<()> {
    if let Some(temp_dir) = clone_result.temp_dir {
        debug!(
            "Cleaning up temporary directory: {}",
            temp_dir.path().display()
        );
        // temp_dir is dropped automatically
    } else {
        warn!(
            "Not cleaning up repository at {} (not a temp directory)",
            clone_result.path.display()
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_github_url_https() {
        let result = parse_github_url("https://github.com/rust-lang/rust");
        assert_eq!(result, Some(("rust-lang".to_string(), "rust".to_string())));
    }

    #[test]
    fn test_parse_github_url_https_with_git() {
        let result = parse_github_url("https://github.com/rust-lang/rust.git");
        assert_eq!(result, Some(("rust-lang".to_string(), "rust".to_string())));
    }

    #[test]
    fn test_parse_github_url_ssh() {
        let result = parse_github_url("git@github.com:rust-lang/rust");
        assert_eq!(result, Some(("rust-lang".to_string(), "rust".to_string())));
    }

    #[test]
    fn test_parse_github_url_invalid() {
        let result = parse_github_url("https://gitlab.com/user/repo");
        assert_eq!(result, None);
    }

    #[test]
    fn test_clone_options_default() {
        let opts = CloneOptions::default();
        assert!(opts.branch.is_none());
        assert_eq!(opts.depth, Some(1));
        assert!(opts.show_progress);
    }
}
