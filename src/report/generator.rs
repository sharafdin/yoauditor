//! Markdown report generation.
//!
//! This module generates comprehensive Markdown audit reports from
//! the analysis results.

use crate::analysis::{language_distribution, most_problematic_files};
use crate::models::{AnalyzedFile, Issue, IssueSummary, Report, ReportMetadata, Severity};
use anyhow::Result;
use std::io::Write;
use std::path::Path;

/// Generate a complete Markdown report.
pub fn generate_markdown_report(report: &Report) -> String {
    let mut output = String::new();

    // Title
    output.push_str("# YoAuditor Report\n\n");

    // Metadata section
    output.push_str(&generate_metadata_section(&report.metadata));

    // Table of contents
    output.push_str(&generate_table_of_contents(report));

    // Project overview
    output.push_str(&generate_overview_section(&report.project_overview));

    // Summary section
    output.push_str(&generate_summary_section(&report.summary, &report.files));

    // Issues by file
    output.push_str(&generate_issues_section(&report.files));

    // Recommendations
    output.push_str(&generate_recommendations_section(&report.recommendations));

    // Footer
    output.push_str(&generate_footer());

    output
}

/// Generate the metadata section.
fn generate_metadata_section(metadata: &ReportMetadata) -> String {
    let mut section = String::new();

    section.push_str("## Metadata\n\n");
    section.push_str(&format!("- **Repository:** {}\n", metadata.repo_url));
    section.push_str(&format!(
        "- **Analysis Date:** {}\n",
        metadata.analysis_date.format("%Y-%m-%d %H:%M:%S UTC")
    ));
    section.push_str(&format!("- **Model Used:** `{}`\n", metadata.model_used));
    section.push_str(&format!(
        "- **Files Analyzed:** {}\n",
        metadata.files_analyzed
    ));
    if metadata.files_failed > 0 {
        section.push_str(&format!("- **Files Failed:** {}\n", metadata.files_failed));
    }
    section.push_str(&format!("- **Total Issues:** {}\n", metadata.total_issues));
    section.push_str(&format!(
        "- **Analysis Duration:** {:.1}s\n",
        metadata.duration_seconds
    ));
    section.push_str("\n");

    section
}

/// Generate the table of contents.
fn generate_table_of_contents(report: &Report) -> String {
    let mut toc = String::new();

    toc.push_str("## Table of Contents\n\n");
    toc.push_str("- [Metadata](#metadata)\n");

    if !report.project_overview.is_empty() {
        toc.push_str("- [Project Overview](#project-overview)\n");
    }

    toc.push_str("- [Summary](#summary)\n");
    toc.push_str("- [Issues by File](#issues-by-file)\n");

    // Add links to each file with issues
    for file in &report.files {
        if !file.issues.is_empty() {
            let anchor = file.path.replace(['/', '.', ' '], "-").to_lowercase();
            toc.push_str(&format!("  - [{}](#{})\n", file.path, anchor));
        }
    }

    if !report.recommendations.is_empty() {
        toc.push_str("- [Recommendations](#recommendations)\n");
    }

    toc.push_str("\n");

    toc
}

/// Generate the project overview section.
fn generate_overview_section(overview: &str) -> String {
    if overview.is_empty() {
        return String::new();
    }

    let mut section = String::new();

    section.push_str("## Project Overview\n\n");
    section.push_str(overview);
    section.push_str("\n\n");

    section
}

/// Generate the summary section.
fn generate_summary_section(summary: &IssueSummary, files: &[AnalyzedFile]) -> String {
    let mut section = String::new();

    section.push_str("## Summary\n\n");

    // Severity breakdown
    section.push_str("### Issue Severity Breakdown\n\n");
    section.push_str(&format!(
        "| {} Critical | {} High | {} Medium | {} Low | **Total** |\n",
        Severity::Critical.emoji(),
        Severity::High.emoji(),
        Severity::Medium.emoji(),
        Severity::Low.emoji(),
    ));
    section.push_str("|:---:|:---:|:---:|:---:|:---:|\n");
    section.push_str(&format!(
        "| {} | {} | {} | {} | **{}** |\n\n",
        summary.critical, summary.high, summary.medium, summary.low, summary.total
    ));

    // Category breakdown
    if !summary.by_category.is_empty() {
        section.push_str("### Issues by Category\n\n");
        section.push_str("| Category | Count |\n");
        section.push_str("|:---|:---:|\n");

        let mut categories: Vec<_> = summary.by_category.iter().collect();
        categories.sort_by_key(|(_, count)| std::cmp::Reverse(*count));

        for (category, count) in categories {
            section.push_str(&format!("| {} | {} |\n", category, count));
        }
        section.push_str("\n");
    }

    // Language distribution
    let lang_dist = language_distribution(files);
    if !lang_dist.is_empty() {
        section.push_str("### Files by Language\n\n");
        section.push_str("| Language | Files |\n");
        section.push_str("|:---|:---:|\n");

        let mut langs: Vec<_> = lang_dist.iter().collect();
        langs.sort_by_key(|(_, count)| std::cmp::Reverse(*count));

        for (lang, count) in langs {
            section.push_str(&format!("| {} | {} |\n", lang, count));
        }
        section.push_str("\n");
    }

    // Most problematic files
    let problematic = most_problematic_files(files, 5);
    if !problematic.is_empty() {
        section.push_str("### Most Problematic Files\n\n");
        section.push_str("| File | Issues |\n");
        section.push_str("|:---|:---:|\n");

        for (file, count) in problematic {
            section.push_str(&format!("| `{}` | {} |\n", file.path, count));
        }
        section.push_str("\n");
    }

    section
}

/// Generate the issues section.
fn generate_issues_section(files: &[AnalyzedFile]) -> String {
    let mut section = String::new();

    section.push_str("## Issues by File\n\n");

    // Filter files with issues
    let files_with_issues: Vec<_> = files.iter().filter(|f| !f.issues.is_empty()).collect();

    if files_with_issues.is_empty() {
        section.push_str("No issues were found in the analyzed files. Great job! 🎉\n\n");
        return section;
    }

    for file in files_with_issues {
        section.push_str(&generate_file_issues_section(file));
    }

    section
}

/// Generate the issues section for a single file.
fn generate_file_issues_section(file: &AnalyzedFile) -> String {
    let mut section = String::new();

    let anchor = file.path.replace(['/', '.', ' '], "-").to_lowercase();
    section.push_str(&format!("### {} {{#{}}}\n\n", file.path, anchor));

    // File info
    section.push_str(&format!(
        "*Language: {} | Lines: {} | Issues: {}*\n\n",
        file.language,
        file.line_count,
        file.issues.len()
    ));

    // Sort issues by severity then by line number
    let mut issues = file.issues.clone();
    issues.sort_by(|a, b| {
        b.severity
            .cmp(&a.severity)
            .then_with(|| a.start_line.cmp(&b.start_line))
    });

    for issue in &issues {
        section.push_str(&generate_issue_block(issue));
    }

    section
}

/// Generate a single issue block.
fn generate_issue_block(issue: &Issue) -> String {
    let mut block = String::new();

    // Issue header with severity badge
    let severity_badge = match issue.severity {
        Severity::Critical => "🔴 **CRITICAL**",
        Severity::High => "🟠 **HIGH**",
        Severity::Medium => "🟡 **MEDIUM**",
        Severity::Low => "🟢 **LOW**",
    };

    block.push_str(&format!(
        "#### {} {} - {}\n\n",
        severity_badge, issue.category, issue.title
    ));

    // Line reference
    block.push_str(&format!("**Lines:** {}\n\n", issue.line_range()));

    // Description
    if !issue.description.is_empty() {
        block.push_str(&format!("**Description:** {}\n\n", issue.description));
    }

    // Code snippet
    if let Some(ref snippet) = issue.code_snippet {
        block.push_str("<details>\n<summary>View Code</summary>\n\n```\n");
        block.push_str(snippet);
        block.push_str("\n```\n</details>\n\n");
    }

    // Suggestion
    if !issue.suggestion.is_empty() {
        block.push_str(&format!("> 💡 **Suggestion:** {}\n\n", issue.suggestion));
    }

    block.push_str("---\n\n");

    block
}

/// Generate the recommendations section.
fn generate_recommendations_section(recommendations: &[String]) -> String {
    if recommendations.is_empty() {
        return String::new();
    }

    let mut section = String::new();

    section.push_str("## Recommendations\n\n");
    section.push_str(
        "Based on the analysis, here are the top recommendations for improving this codebase:\n\n",
    );

    for (i, rec) in recommendations.iter().enumerate() {
        section.push_str(&format!("{}. {}\n", i + 1, rec));
    }
    section.push_str("\n");

    section
}

/// Generate the report footer.
fn generate_footer() -> String {
    let mut footer = String::new();

    footer.push_str("---\n\n");
    footer.push_str("*Report generated by [YoAuditor](https://github.com/sharafdin/yoauditor)*\n");

    footer
}

/// Write the report to a file.
#[allow(dead_code)] // Alternative to using save_report
pub fn write_report(report: &Report, path: &Path) -> Result<()> {
    let content = generate_markdown_report(report);

    let mut file = std::fs::File::create(path)?;
    file.write_all(content.as_bytes())?;

    Ok(())
}

/// Generate a JSON report.
pub fn generate_json_report(report: &Report) -> Result<String> {
    serde_json::to_string_pretty(report).map_err(Into::into)
}

/// Write a JSON report to a file.
#[allow(dead_code)] // Convenience wrapper
pub fn write_json_report(report: &Report, path: &Path) -> Result<()> {
    let content = generate_json_report(report)?;

    let mut file = std::fs::File::create(path)?;
    file.write_all(content.as_bytes())?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_test_report() -> Report {
        let metadata = ReportMetadata {
            repo_url: "https://github.com/test/repo".to_string(),
            analysis_date: Utc::now(),
            model_used: "test-model".to_string(),
            files_analyzed: 10,
            files_failed: 0,
            total_issues: 5,
            duration_seconds: 30.0,
        };

        Report {
            metadata,
            project_overview: "Test project overview".to_string(),
            files: vec![AnalyzedFile {
                path: "src/main.rs".to_string(),
                language: "Rust".to_string(),
                line_count: 100,
                issues: vec![Issue {
                    file_path: "src/main.rs".to_string(),
                    start_line: 10,
                    end_line: Some(15),
                    severity: Severity::High,
                    category: "Bug".to_string(),
                    title: "Unchecked error".to_string(),
                    description: "Error is not handled".to_string(),
                    suggestion: "Use ? operator".to_string(),
                    code_snippet: Some("let x = foo.unwrap();".to_string()),
                }],
                analysis_successful: true,
                error: None,
            }],
            summary: IssueSummary {
                total: 5,
                critical: 0,
                high: 2,
                medium: 2,
                low: 1,
                by_category: [("Bug".to_string(), 3), ("Style".to_string(), 2)]
                    .into_iter()
                    .collect(),
            },
            recommendations: vec![
                "Add proper error handling".to_string(),
                "Improve test coverage".to_string(),
            ],
        }
    }

    #[test]
    fn test_generate_markdown_report() {
        let report = create_test_report();
        let markdown = generate_markdown_report(&report);

        assert!(markdown.contains("# YoAuditor Report"));
        assert!(markdown.contains("## Metadata"));
        assert!(markdown.contains("## Summary"));
        assert!(markdown.contains("## Issues by File"));
        assert!(markdown.contains("src/main.rs"));
        assert!(markdown.contains("Unchecked error"));
    }

    #[test]
    fn test_generate_metadata_section() {
        let metadata = ReportMetadata {
            repo_url: "https://github.com/test/repo".to_string(),
            analysis_date: Utc::now(),
            model_used: "test-model".to_string(),
            files_analyzed: 10,
            files_failed: 2,
            total_issues: 5,
            duration_seconds: 30.0,
        };

        let section = generate_metadata_section(&metadata);

        assert!(section.contains("https://github.com/test/repo"));
        assert!(section.contains("test-model"));
        assert!(section.contains("10"));
        assert!(section.contains("Files Failed:"));
    }

    #[test]
    fn test_generate_issue_block() {
        let issue = Issue {
            file_path: "test.rs".to_string(),
            start_line: 10,
            end_line: Some(15),
            severity: Severity::Critical,
            category: "Security".to_string(),
            title: "SQL Injection".to_string(),
            description: "User input not sanitized".to_string(),
            suggestion: "Use parameterized queries".to_string(),
            code_snippet: Some("query(user_input)".to_string()),
        };

        let block = generate_issue_block(&issue);

        assert!(block.contains("CRITICAL"));
        assert!(block.contains("Security"));
        assert!(block.contains("SQL Injection"));
        assert!(block.contains("10-15"));
        assert!(block.contains("Use parameterized queries"));
    }

    #[test]
    fn test_generate_json_report() {
        let report = create_test_report();
        let json = generate_json_report(&report).unwrap();

        assert!(json.contains("\"repo_url\""));
        assert!(json.contains("\"files\""));
        assert!(json.contains("\"issues\""));
    }
}
