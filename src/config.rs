//! Configuration file handling.
//!
//! This module handles loading and merging configuration from
//! `.yoauditor.toml` files.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Root configuration structure.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    /// General settings.
    #[serde(default)]
    pub general: GeneralConfig,

    /// Model settings.
    #[serde(default)]
    pub model: ModelConfig,

    /// Scanner settings.
    #[serde(default)]
    pub scanner: ScannerConfig,

    /// Report settings.
    #[serde(default)]
    pub report: ReportConfig,
}

/// General application settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    /// Default output file path.
    #[serde(default = "default_output")]
    pub output: String,

    /// Enable verbose logging by default.
    #[serde(default)]
    pub verbose: bool,

    /// Number of concurrent file analyses.
    #[serde(default = "default_concurrency")]
    pub concurrency: usize,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            output: default_output(),
            verbose: false,
            concurrency: default_concurrency(),
        }
    }
}

fn default_output() -> String {
    "yoaudit_report.md".to_string()
}

fn default_concurrency() -> usize {
    4
}

/// LLM model settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    /// Default model name.
    #[serde(default = "default_model")]
    pub name: String,

    /// Ollama API URL.
    #[serde(default = "default_ollama_url")]
    pub ollama_url: String,

    /// Temperature for generation.
    #[serde(default = "default_temperature")]
    pub temperature: f32,

    /// Maximum tokens in response.
    #[serde(default)]
    pub max_tokens: Option<usize>,

    /// Request timeout in seconds.
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,

    /// Number of retries on failure.
    #[serde(default = "default_retries")]
    pub retries: usize,

    /// Use single-call mode (efficient for cloud models).
    /// If true: reads all files and sends in ONE API call.
    /// If false: uses tool-calling (many API calls).
    #[serde(default = "default_single_call")]
    pub single_call_mode: bool,
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            name: default_model(),
            ollama_url: default_ollama_url(),
            temperature: default_temperature(),
            max_tokens: None,
            timeout_seconds: default_timeout(),
            retries: default_retries(),
            single_call_mode: true, // Default to efficient mode
        }
    }
}

fn default_model() -> String {
    "llama3.2:latest".to_string()
}

fn default_ollama_url() -> String {
    "http://localhost:11434".to_string()
}

fn default_temperature() -> f32 {
    0.1
}

fn default_timeout() -> u64 {
    1800 // 30 min; single-call with large models can exceed 900s
}

fn default_retries() -> usize {
    3
}

fn default_single_call() -> bool {
    true
}

/// File scanner settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannerConfig {
    /// Maximum files to analyze.
    #[serde(default = "default_max_files")]
    pub max_files: usize,

    /// Maximum lines per file chunk.
    #[serde(default = "default_max_chunk_lines")]
    pub max_chunk_lines: usize,

    /// File extensions to include.
    #[serde(default = "default_extensions")]
    pub extensions: Vec<String>,

    /// Patterns to exclude.
    #[serde(default = "default_excludes")]
    pub excludes: Vec<String>,

    /// Maximum file size in bytes.
    #[serde(default = "default_max_file_size")]
    pub max_file_size: usize,
}

impl Default for ScannerConfig {
    fn default() -> Self {
        Self {
            max_files: default_max_files(),
            max_chunk_lines: default_max_chunk_lines(),
            extensions: default_extensions(),
            excludes: default_excludes(),
            max_file_size: default_max_file_size(),
        }
    }
}

fn default_max_files() -> usize {
    100
}

fn default_max_chunk_lines() -> usize {
    4000
}

fn default_extensions() -> Vec<String> {
    vec![
        "rs", "py", "js", "ts", "jsx", "tsx", "go", "java", "c", "cpp", "h", "hpp", "cs", "rb",
        "php", "swift", "kt", "scala", "vue", "svelte",
    ]
    .into_iter()
    .map(String::from)
    .collect()
}

fn default_excludes() -> Vec<String> {
    vec![
        ".git",
        "target",
        "node_modules",
        "vendor",
        "dist",
        "build",
        "__pycache__",
        ".venv",
        "venv",
        ".idea",
        ".vscode",
    ]
    .into_iter()
    .map(String::from)
    .collect()
}

fn default_max_file_size() -> usize {
    1024 * 1024 // 1MB
}

/// Report generation settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportConfig {
    /// Include code snippets in the report.
    #[serde(default = "default_true")]
    pub include_snippets: bool,

    /// Include file summary sections.
    #[serde(default = "default_true")]
    pub include_summaries: bool,

    /// Maximum code snippet lines.
    #[serde(default = "default_snippet_lines")]
    pub max_snippet_lines: usize,

    /// Group issues by file (true) or by severity (false).
    #[serde(default = "default_true")]
    pub group_by_file: bool,
}

impl Default for ReportConfig {
    fn default() -> Self {
        Self {
            include_snippets: true,
            include_summaries: true,
            max_snippet_lines: default_snippet_lines(),
            group_by_file: true,
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_snippet_lines() -> usize {
    10
}

impl Config {
    /// Load configuration from a file path.
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;

        let config: Config = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", path.display()))?;

        Ok(config)
    }

    /// Try to load configuration from the default location.
    ///
    /// Returns `Ok(None)` if the file doesn't exist, `Err` if it exists but can't be parsed.
    pub fn load_default() -> Result<Option<Self>> {
        let default_path = Path::new(".yoauditor.toml");

        if default_path.exists() {
            Ok(Some(Self::load(default_path)?))
        } else {
            Ok(None)
        }
    }

    /// Try to load configuration from a repo directory.
    pub fn load_from_repo(repo_path: &Path) -> Result<Option<Self>> {
        let config_path = repo_path.join(".yoauditor.toml");

        if config_path.exists() {
            Ok(Some(Self::load(&config_path)?))
        } else {
            Ok(None)
        }
    }

    /// Merge this configuration with CLI arguments.
    ///
    /// CLI arguments take precedence over config file settings.
    /// This method only overrides config when CLI provides explicit values.
    pub fn merge_with_args(&mut self, args: &crate::cli::Args) {
        // Model settings - always override since they have defaults in CLI
        self.model.name = args.model.clone();
        self.model.ollama_url = args.ollama_url.clone();
        self.model.temperature = args.temperature;

        // Timeout - only override if explicitly provided via CLI
        if let Some(timeout) = args.timeout {
            self.model.timeout_seconds = timeout;
        }

        // Single-call mode - only override if explicitly provided via CLI flags
        if args.single_call {
            self.model.single_call_mode = true;
        } else if args.no_single_call {
            self.model.single_call_mode = false;
        }

        // Scanner settings - always override
        self.scanner.max_files = args.max_files;
        self.scanner.max_chunk_lines = args.max_chunk_lines;

        // Optional settings - only override if provided
        if let Some(ref extensions) = args.extensions {
            self.scanner.extensions = extensions.clone();
        }
        if let Some(ref excludes) = args.exclude {
            self.scanner.excludes = excludes.clone();
        }

        // General settings
        self.general.concurrency = args.concurrency;

        // Flags always override
        if args.verbose {
            self.general.verbose = true;
        }
    }

    /// Generate a default configuration file content.
    #[allow(dead_code)] // Utility for generating example config
    pub fn default_toml() -> String {
        let config = Config::default();
        toml::to_string_pretty(&config).unwrap_or_else(|_| String::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.model.name, "llama3.2:latest");
        assert_eq!(config.scanner.max_files, 100);
        assert!(config.scanner.extensions.contains(&"rs".to_string()));
    }

    #[test]
    fn test_parse_config() {
        let toml_content = r#"
[general]
output = "custom_report.md"
verbose = true

[model]
name = "codellama:34b"
temperature = 0.2

[scanner]
max_files = 50
extensions = ["rs", "py"]
"#;

        let config: Config = toml::from_str(toml_content).unwrap();
        assert_eq!(config.general.output, "custom_report.md");
        assert!(config.general.verbose);
        assert_eq!(config.model.name, "codellama:34b");
        assert_eq!(config.model.temperature, 0.2);
        assert_eq!(config.scanner.max_files, 50);
        assert_eq!(config.scanner.extensions, vec!["rs", "py"]);
    }

    #[test]
    fn test_default_toml_generation() {
        let toml_str = Config::default_toml();
        assert!(!toml_str.is_empty());
        assert!(toml_str.contains("[general]"));
        assert!(toml_str.contains("[model]"));
        assert!(toml_str.contains("[scanner]"));
    }
}
