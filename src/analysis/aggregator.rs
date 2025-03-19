//! Issue aggregation and statistics.
//!
//! This module provides utilities for aggregating issues from multiple files
//! and computing summary statistics.

use crate::models::{AnalyzedFile, Issue, IssueSummary, Severity};
use std::collections::HashMap;

/// Aggregate all issues from analyzed files.
#[allow(dead_code)] // Utility for batch processing
pub fn aggregate_issues(files: &[AnalyzedFile]) -> Vec<Issue> {
    files.iter().flat_map(|f| f.issues.clone()).collect()
}

/// Sort issues by severity (critical first).
#[allow(dead_code)] // Utility for sorting
pub fn sort_issues_by_severity(issues: &mut [Issue]) {
    issues.sort_by(|a, b| b.severity.cmp(&a.severity));
}

/// Group issues by file path.
pub fn group_by_file(issues: &[Issue]) -> HashMap<String, Vec<Issue>> {
    let mut grouped: HashMap<String, Vec<Issue>> = HashMap::new();

    for issue in issues {
        grouped
            .entry(issue.file_path.clone())
            .or_default()
            .push(issue.clone());
    }

    // Sort issues within each file by line number
    for issues in grouped.values_mut() {
        issues.sort_by_key(|i| i.start_line);
    }

    grouped
}

/// Group issues by severity.
#[allow(dead_code)] // Utility for grouping
pub fn group_by_severity(issues: &[Issue]) -> HashMap<Severity, Vec<Issue>> {
    let mut grouped: HashMap<Severity, Vec<Issue>> = HashMap::new();

    for issue in issues {
        grouped
            .entry(issue.severity)
            .or_default()
            .push(issue.clone());
    }

    grouped
}

/// Group issues by category.
#[allow(dead_code)] // Utility for grouping
pub fn group_by_category(issues: &[Issue]) -> HashMap<String, Vec<Issue>> {
    let mut grouped: HashMap<String, Vec<Issue>> = HashMap::new();

    for issue in issues {
        grouped
            .entry(issue.category.clone())
            .or_default()
            .push(issue.clone());
    }

    grouped
}

/// Get the top N issues by severity.
#[allow(dead_code)] // Utility for filtering
pub fn top_issues(issues: &[Issue], n: usize) -> Vec<Issue> {
    let mut sorted: Vec<Issue> = issues.to_vec();
    sort_issues_by_severity(&mut sorted);
    sorted.truncate(n);
    sorted
}

/// Compute language distribution from analyzed files.
pub fn language_distribution(files: &[AnalyzedFile]) -> HashMap<String, usize> {
    let mut dist: HashMap<String, usize> = HashMap::new();

    for file in files {
        *dist.entry(file.language.clone()).or_default() += 1;
    }

    dist
}

/// Compute issue density (issues per 1000 lines) by file.
#[allow(dead_code)] // Utility for statistics
pub fn issue_density(files: &[AnalyzedFile]) -> Vec<(String, f64)> {
    let mut densities: Vec<(String, f64)> = files
        .iter()
        .filter(|f| f.line_count > 0)
        .map(|f| {
            let density = (f.issues.len() as f64 / f.line_count as f64) * 1000.0;
            (f.path.clone(), density)
        })
        .collect();

    // Sort by density (highest first)
    densities.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    densities
}

/// Generate a text summary of issue statistics.
#[allow(dead_code)] // Utility for text generation
pub fn generate_summary_text(summary: &IssueSummary) -> String {
    let mut lines = Vec::new();

    lines.push(format!("Total Issues: {}", summary.total));
    lines.push(format!(
        "- {} Critical: {}",
        Severity::Critical.emoji(),
        summary.critical
    ));
    lines.push(format!(
        "- {} High: {}",
        Severity::High.emoji(),
        summary.high
    ));
    lines.push(format!(
        "- {} Medium: {}",
        Severity::Medium.emoji(),
        summary.medium
    ));
    lines.push(format!("- {} Low: {}", Severity::Low.emoji(), summary.low));

    if !summary.by_category.is_empty() {
        lines.push(String::new());
        lines.push("By Category:".to_string());

        let mut categories: Vec<_> = summary.by_category.iter().collect();
        categories.sort_by_key(|(_, count)| std::cmp::Reverse(*count));

        for (category, count) in categories {
            lines.push(format!("- {}: {}", category, count));
        }
    }

    lines.join("\n")
}

/// Identify the most problematic files (by issue count).
pub fn most_problematic_files(files: &[AnalyzedFile], n: usize) -> Vec<(&AnalyzedFile, usize)> {
    let mut file_issues: Vec<_> = files
        .iter()
        .map(|f| (f, f.issues.len()))
        .filter(|(_, count)| *count > 0)
        .collect();

    file_issues.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
    file_issues.truncate(n);

    file_issues
}

/// Identify patterns in issues (common titles/categories).
#[allow(dead_code)] // Utility for pattern analysis
pub fn identify_patterns(issues: &[Issue]) -> Vec<(String, usize)> {
    let mut title_counts: HashMap<String, usize> = HashMap::new();

    for issue in issues {
        // Normalize the title for grouping
        let normalized = issue.title.to_lowercase();
        *title_counts.entry(normalized).or_default() += 1;
    }

    let mut patterns: Vec<_> = title_counts
        .into_iter()
        .filter(|(_, count)| *count > 1) // Only show repeated patterns
        .collect();

    patterns.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
    patterns
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_issue(severity: Severity, category: &str) -> Issue {
        Issue {
            file_path: "test.rs".to_string(),
            start_line: 1,
            end_line: None,
            severity,
            category: category.to_string(),
            title: "Test issue".to_string(),
            description: "Test description".to_string(),
            suggestion: "Fix it".to_string(),
            code_snippet: None,
        }
    }

    #[test]
    fn test_aggregate_issues() {
        let files = vec![
            AnalyzedFile {
                path: "file1.rs".to_string(),
                language: "Rust".to_string(),
                line_count: 100,
                issues: vec![create_test_issue(Severity::High, "Bug")],
                analysis_successful: true,
                error: None,
            },
            AnalyzedFile {
                path: "file2.rs".to_string(),
                language: "Rust".to_string(),
                line_count: 50,
                issues: vec![
                    create_test_issue(Severity::Critical, "Security"),
                    create_test_issue(Severity::Low, "Style"),
                ],
                analysis_successful: true,
                error: None,
            },
        ];

        let issues = aggregate_issues(&files);
        assert_eq!(issues.len(), 3);
    }

    #[test]
    fn test_sort_issues_by_severity() {
        let mut issues = vec![
            create_test_issue(Severity::Low, "Style"),
            create_test_issue(Severity::Critical, "Security"),
            create_test_issue(Severity::Medium, "Bug"),
        ];

        sort_issues_by_severity(&mut issues);

        assert_eq!(issues[0].severity, Severity::Critical);
        assert_eq!(issues[1].severity, Severity::Medium);
        assert_eq!(issues[2].severity, Severity::Low);
    }

    #[test]
    fn test_group_by_category() {
        let issues = vec![
            create_test_issue(Severity::High, "Bug"),
            create_test_issue(Severity::High, "Security"),
            create_test_issue(Severity::Low, "Bug"),
        ];

        let grouped = group_by_category(&issues);

        assert_eq!(grouped.get("Bug").map(|v| v.len()), Some(2));
        assert_eq!(grouped.get("Security").map(|v| v.len()), Some(1));
    }

    #[test]
    fn test_top_issues() {
        let issues = vec![
            create_test_issue(Severity::Low, "Style"),
            create_test_issue(Severity::Critical, "Security"),
            create_test_issue(Severity::High, "Bug"),
        ];

        let top = top_issues(&issues, 2);

        assert_eq!(top.len(), 2);
        assert_eq!(top[0].severity, Severity::Critical);
        assert_eq!(top[1].severity, Severity::High);
    }

    #[test]
    fn test_issue_density() {
        let files = vec![
            AnalyzedFile {
                path: "dense.rs".to_string(),
                language: "Rust".to_string(),
                line_count: 100,
                issues: vec![
                    create_test_issue(Severity::High, "Bug"),
                    create_test_issue(Severity::High, "Bug"),
                ],
                analysis_successful: true,
                error: None,
            },
            AnalyzedFile {
                path: "sparse.rs".to_string(),
                language: "Rust".to_string(),
                line_count: 1000,
                issues: vec![create_test_issue(Severity::Low, "Style")],
                analysis_successful: true,
                error: None,
            },
        ];

        let densities = issue_density(&files);

        // dense.rs should have higher density (20 per 1000 lines vs 1 per 1000 lines)
        assert_eq!(densities[0].0, "dense.rs");
        assert!(densities[0].1 > densities[1].1);
    }
}
