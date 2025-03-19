//! Data models for the code auditor.
//!
//! This module contains all the core data structures used throughout
//! the application for representing issues, files, and reports.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Severity level of an issue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Low severity - style issues, minor suggestions
    Low,
    /// Medium severity - code quality issues, potential bugs
    Medium,
    /// High severity - bugs, security concerns
    High,
    /// Critical severity - security vulnerabilities, major bugs
    Critical,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Severity::Low => write!(f, "Low"),
            Severity::Medium => write!(f, "Medium"),
            Severity::High => write!(f, "High"),
            Severity::Critical => write!(f, "Critical"),
        }
    }
}

impl Severity {
    /// Returns an emoji representation of the severity.
    pub fn emoji(&self) -> &'static str {
        match self {
            Severity::Low => "🟢",
            Severity::Medium => "🟡",
            Severity::High => "🟠",
            Severity::Critical => "🔴",
        }
    }
}

/// Category of an issue (for future structured categorization).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[allow(dead_code)] // Keep for future use
pub enum Category {
    Bug,
    Security,
    Performance,
    Style,
    Documentation,
    BestPractice,
    Other(String),
}

impl fmt::Display for Category {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Category::Bug => write!(f, "Bug"),
            Category::Security => write!(f, "Security"),
            Category::Performance => write!(f, "Performance"),
            Category::Style => write!(f, "Style"),
            Category::Documentation => write!(f, "Documentation"),
            Category::BestPractice => write!(f, "Best Practice"),
            Category::Other(s) => write!(f, "{}", s),
        }
    }
}

impl From<&str> for Category {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "bug" => Category::Bug,
            "security" => Category::Security,
            "performance" => Category::Performance,
            "style" => Category::Style,
            "documentation" | "docs" => Category::Documentation,
            "best practice" | "bestpractice" | "best_practice" => Category::BestPractice,
            other => Category::Other(other.to_string()),
        }
    }
}

/// Represents a single issue found during code analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    /// Path to the file containing the issue (relative to repo root).
    pub file_path: String,
    /// Starting line number of the issue (1-indexed).
    pub start_line: usize,
    /// Ending line number of the issue (1-indexed, optional).
    pub end_line: Option<usize>,
    /// Severity of the issue.
    pub severity: Severity,
    /// Category of the issue.
    pub category: String,
    /// Short title describing the issue.
    pub title: String,
    /// Detailed description of the issue.
    pub description: String,
    /// Suggested fix or improvement.
    pub suggestion: String,
    /// Optional code snippet showing the issue.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code_snippet: Option<String>,
}

impl Issue {
    /// Returns the line range as a formatted string.
    pub fn line_range(&self) -> String {
        match self.end_line {
            Some(end) if end != self.start_line => format!("{}-{}", self.start_line, end),
            _ => self.start_line.to_string(),
        }
    }
}

/// Represents an analyzed source code file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyzedFile {
    /// Path to the file (relative to repo root).
    pub path: String,
    /// Detected programming language.
    pub language: String,
    /// Total number of lines in the file.
    pub line_count: usize,
    /// Issues found in this file.
    pub issues: Vec<Issue>,
    /// Whether the analysis was successful.
    pub analysis_successful: bool,
    /// Error message if analysis failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl AnalyzedFile {
    /// Creates a new analyzed file with no issues.
    #[allow(dead_code)] // Utility for future use
    pub fn new(path: String, language: String, line_count: usize) -> Self {
        Self {
            path,
            language,
            line_count,
            issues: Vec::new(),
            analysis_successful: true,
            error: None,
        }
    }

    /// Creates a failed analysis result.
    #[allow(dead_code)] // Utility for future use
    pub fn failed(path: String, error: String) -> Self {
        Self {
            path,
            language: String::new(),
            line_count: 0,
            issues: Vec::new(),
            analysis_successful: false,
            error: Some(error),
        }
    }

    /// Returns the number of issues by severity.
    #[allow(dead_code)] // Utility for filtering
    pub fn issue_count_by_severity(&self, severity: Severity) -> usize {
        self.issues
            .iter()
            .filter(|i| i.severity == severity)
            .count()
    }
}

/// Summary of issues found during analysis.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IssueSummary {
    /// Total number of issues.
    pub total: usize,
    /// Number of critical issues.
    pub critical: usize,
    /// Number of high severity issues.
    pub high: usize,
    /// Number of medium severity issues.
    pub medium: usize,
    /// Number of low severity issues.
    pub low: usize,
    /// Issues grouped by category.
    pub by_category: std::collections::HashMap<String, usize>,
}

impl IssueSummary {
    /// Creates a summary from a list of issues.
    pub fn from_issues(issues: &[Issue]) -> Self {
        let mut summary = Self::default();
        summary.total = issues.len();

        for issue in issues {
            match issue.severity {
                Severity::Critical => summary.critical += 1,
                Severity::High => summary.high += 1,
                Severity::Medium => summary.medium += 1,
                Severity::Low => summary.low += 1,
            }

            *summary
                .by_category
                .entry(issue.category.clone())
                .or_insert(0) += 1;
        }

        summary
    }
}

/// Metadata about the audit report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportMetadata {
    /// URL of the analyzed repository.
    pub repo_url: String,
    /// Date and time of the analysis.
    pub analysis_date: DateTime<Utc>,
    /// Name of the LLM model used.
    pub model_used: String,
    /// Number of files analyzed.
    pub files_analyzed: usize,
    /// Number of files that failed analysis.
    pub files_failed: usize,
    /// Total number of issues found.
    pub total_issues: usize,
    /// Duration of the analysis in seconds.
    pub duration_seconds: f64,
}

/// The complete code audit report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    /// Metadata about the report.
    pub metadata: ReportMetadata,
    /// AI-generated overview of the project.
    pub project_overview: String,
    /// List of analyzed files with their issues.
    pub files: Vec<AnalyzedFile>,
    /// Summary statistics of the issues.
    pub summary: IssueSummary,
    /// High-level improvement recommendations.
    pub recommendations: Vec<String>,
}

impl Report {
    /// Creates a new report with the given metadata.
    #[allow(dead_code)] // Builder utility
    pub fn new(metadata: ReportMetadata) -> Self {
        Self {
            metadata,
            project_overview: String::new(),
            files: Vec::new(),
            summary: IssueSummary::default(),
            recommendations: Vec::new(),
        }
    }

    /// Calculates and updates the summary from all analyzed files.
    #[allow(dead_code)] // Utility for incremental report building
    pub fn calculate_summary(&mut self) {
        let all_issues: Vec<&Issue> = self.files.iter().flat_map(|f| &f.issues).collect();
        self.summary =
            IssueSummary::from_issues(&all_issues.into_iter().cloned().collect::<Vec<_>>());
        self.metadata.total_issues = self.summary.total;
    }
}

/// Represents a file to be analyzed (for future batch processing).
#[derive(Debug, Clone)]
#[allow(dead_code)] // Kept for future batch analysis mode
pub struct FileToAnalyze {
    /// Absolute path to the file.
    pub absolute_path: std::path::PathBuf,
    /// Path relative to the repository root.
    pub relative_path: String,
    /// Detected programming language.
    pub language: String,
    /// File content.
    pub content: String,
    /// Number of lines in the file.
    pub line_count: usize,
}

/// Response from the LLM analysis (for future structured parsing).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)] // Kept for future structured response parsing
pub struct AnalysisResponse {
    /// List of issues found.
    pub issues: Vec<Issue>,
    /// Brief summary of the file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_summary: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_severity_ordering() {
        assert!(Severity::Low < Severity::Medium);
        assert!(Severity::Medium < Severity::High);
        assert!(Severity::High < Severity::Critical);
    }

    #[test]
    fn test_severity_emoji() {
        assert_eq!(Severity::Critical.emoji(), "🔴");
        assert_eq!(Severity::High.emoji(), "🟠");
        assert_eq!(Severity::Medium.emoji(), "🟡");
        assert_eq!(Severity::Low.emoji(), "🟢");
    }

    #[test]
    fn test_category_from_str() {
        assert_eq!(Category::from("bug"), Category::Bug);
        assert_eq!(Category::from("Security"), Category::Security);
        assert_eq!(Category::from("PERFORMANCE"), Category::Performance);
        assert_eq!(
            Category::from("custom"),
            Category::Other("custom".to_string())
        );
    }

    #[test]
    fn test_issue_line_range() {
        let issue = Issue {
            file_path: "test.rs".to_string(),
            start_line: 10,
            end_line: Some(15),
            severity: Severity::Medium,
            category: "Bug".to_string(),
            title: "Test".to_string(),
            description: "Test description".to_string(),
            suggestion: "Test suggestion".to_string(),
            code_snippet: None,
        };
        assert_eq!(issue.line_range(), "10-15");

        let single_line_issue = Issue {
            start_line: 10,
            end_line: None,
            ..issue.clone()
        };
        assert_eq!(single_line_issue.line_range(), "10");
    }

    #[test]
    fn test_issue_summary() {
        let issues = vec![
            Issue {
                file_path: "test.rs".to_string(),
                start_line: 1,
                end_line: None,
                severity: Severity::Critical,
                category: "Security".to_string(),
                title: "Test".to_string(),
                description: "".to_string(),
                suggestion: "".to_string(),
                code_snippet: None,
            },
            Issue {
                file_path: "test.rs".to_string(),
                start_line: 2,
                end_line: None,
                severity: Severity::High,
                category: "Bug".to_string(),
                title: "Test".to_string(),
                description: "".to_string(),
                suggestion: "".to_string(),
                code_snippet: None,
            },
            Issue {
                file_path: "test.rs".to_string(),
                start_line: 3,
                end_line: None,
                severity: Severity::Low,
                category: "Security".to_string(),
                title: "Test".to_string(),
                description: "".to_string(),
                suggestion: "".to_string(),
                code_snippet: None,
            },
        ];

        let summary = IssueSummary::from_issues(&issues);
        assert_eq!(summary.total, 3);
        assert_eq!(summary.critical, 1);
        assert_eq!(summary.high, 1);
        assert_eq!(summary.low, 1);
        assert_eq!(summary.by_category.get("Security"), Some(&2));
        assert_eq!(summary.by_category.get("Bug"), Some(&1));
    }
}
