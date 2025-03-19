//! YoAuditor - AI-powered GitHub Repository Analyzer
//!
//! A CLI tool that uses Ollama with tool-calling to analyze
//! source code repositories and generate detailed audit reports.
//!
//! Exit codes:
//!   0 - Success (no issues above threshold, or no --fail-on set)
//!   1 - Runtime error (connection, config, clone failure, etc.)
//!   2 - Issues found above --fail-on threshold

mod agent;
mod analysis;
mod cli;
mod config;
mod models;
mod repo;
mod report;
mod scanner;

use anyhow::{Context, Result};
use chrono::Utc;
use cli::{Args, FailOnLevel, OutputFormat};
use config::Config;
use models::{AnalyzedFile, Issue, IssueSummary, Report, ReportMetadata, Severity};
use std::path::PathBuf;
use std::time::Instant;
use tracing::{debug, error, info, warn};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command-line arguments
    let args = Args::parse_args();

    // Validate arguments
    if let Err(e) = args.validate() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }

    // Handle --init-config early (no logging needed)
    if args.init_config {
        return handle_init_config();
    }

    // Initialize logging
    init_logging(&args);

    info!("YoAuditor v{}", env!("CARGO_PKG_VERSION"));
    debug!("Arguments: {:?}", args);

    // Run the audit
    match run_audit(args).await {
        Ok(exit_code) => {
            std::process::exit(exit_code);
        }
        Err(e) => {
            error!("Audit failed: {}", e);
            eprintln!("\n❌ Error: {}", e);
            std::process::exit(1);
        }
    }
}

/// Handle --init-config: generate a default .yoauditor.toml.
fn handle_init_config() -> Result<()> {
    let path = std::path::Path::new(".yoauditor.toml");

    if path.exists() {
        eprintln!("⚠️  .yoauditor.toml already exists. Remove it first or edit it manually.");
        std::process::exit(1);
    }

    let content = Config::default_toml();
    std::fs::write(path, &content).context("Failed to write .yoauditor.toml")?;

    println!("✅ Created .yoauditor.toml with default settings.");
    println!("   Edit it to customize model, extensions, excludes, and more.");
    Ok(())
}

/// Initialize logging based on verbosity settings.
fn init_logging(args: &Args) {
    let level = args.log_level();

    let subscriber = FmtSubscriber::builder()
        .with_max_level(level)
        .with_target(false)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .compact()
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("Failed to set tracing subscriber");
}

/// Run the complete audit workflow. Returns exit code (0 or 2).
async fn run_audit(args: Args) -> Result<i32> {
    let start_time = Instant::now();

    // Load configuration
    let mut config = load_config(&args)?;
    config.merge_with_args(&args);

    let repo_url = args.repo_url().to_string();

    // Step 1: Get the repository
    println!("📥 Cloning repository: {}", repo_url);
    let repo_path = get_repository(&args).await?;
    info!("Repository at: {}", repo_path.display());

    // Try to load config from repository
    if let Ok(Some(repo_config)) = Config::load_from_repo(&repo_path) {
        info!("Found .yoauditor.toml in repository");
        config = repo_config;
        config.merge_with_args(&args);
    }

    // Create scan config from scanner settings
    let scan_config = scanner::ScanConfig::from(&config.scanner);

    // Handle --dry-run: scan files and exit
    if args.dry_run {
        return handle_dry_run(&repo_path, &scan_config);
    }

    // Step 2: Initialize the agent
    let mode_str = if config.model.single_call_mode {
        "Single-call (efficient)"
    } else {
        "Tool-calling (agentic)"
    };

    println!("🤖 Initializing AI agent...");
    println!("   Model: {}", config.model.name);
    println!("   Ollama: {}", config.model.ollama_url);
    println!("   Mode: {}", mode_str);
    println!("   Timeout: {}s", config.model.timeout_seconds);

    let agent_config = agent::AgentConfig {
        ollama_url: config.model.ollama_url.clone(),
        model_name: config.model.name.clone(),
        temperature: config.model.temperature,
        max_iterations: 50,
        timeout_seconds: config.model.timeout_seconds,
        single_call_mode: config.model.single_call_mode,
        max_context_messages: 10,
    };

    let mut agent = agent::CodeAnalysisAgent::new(agent_config, repo_path.clone(), scan_config);

    // Step 3: Run the agentic analysis
    println!("\n🔬 Running code analysis...");
    if config.model.single_call_mode {
        println!("   Reading all files and sending in ONE API call...");
        println!("   ⏳ This may take several minutes. Timeout: {}s\n", config.model.timeout_seconds);
    } else {
        println!("   The AI agent will explore the repository using tools...\n");
    }

    let reported_issues = agent.run_analysis().await?;

    // Step 4: Convert reported issues to our Issue format
    let mut issues: Vec<Issue> = reported_issues
        .into_iter()
        .map(|ri| Issue {
            file_path: ri.file_path,
            start_line: ri.line_number,
            end_line: None,
            severity: match ri.severity.to_lowercase().as_str() {
                "critical" => Severity::Critical,
                "high" => Severity::High,
                "medium" => Severity::Medium,
                _ => Severity::Low,
            },
            category: ri.category,
            title: ri.title,
            description: ri.description,
            suggestion: ri.suggestion,
            code_snippet: None,
        })
        .collect();

    // Apply --min-severity filter
    if let Some(min_level) = args.min_severity {
        let min_severity = fail_on_to_severity(min_level);
        issues.retain(|issue| issue.severity >= min_severity);
    }

    // Step 5: Build the report
    println!("\n📝 Generating report...");

    let duration = start_time.elapsed().as_secs_f64();
    let summary = IssueSummary::from_issues(&issues);

    // Group issues by file using the aggregator
    let files_map = analysis::group_by_file(&issues);

    let analyzed_files: Vec<AnalyzedFile> = files_map
        .into_iter()
        .map(|(path, file_issues)| AnalyzedFile {
            path,
            language: "Unknown".to_string(),
            line_count: 0,
            issues: file_issues,
            analysis_successful: true,
            error: None,
        })
        .collect();

    let metadata = ReportMetadata {
        repo_url: repo_url.clone(),
        analysis_date: Utc::now(),
        model_used: config.model.name.clone(),
        files_analyzed: analyzed_files.len(),
        files_failed: 0,
        total_issues: summary.total,
        duration_seconds: duration,
    };

    let report = Report {
        metadata,
        project_overview: "Analysis performed by AI agent with tool-calling capabilities."
            .to_string(),
        files: analyzed_files,
        summary: summary.clone(),
        recommendations: vec![
            "Review all reported issues and prioritize by severity.".to_string(),
            "Address critical and high severity issues first.".to_string(),
        ],
    };

    // Step 6: Generate and save the report
    let output = match args.format {
        OutputFormat::Json => report::generate_json_report(&report)?,
        OutputFormat::Markdown => report::generate_markdown_report(&report),
    };

    std::fs::write(&args.output, &output)
        .with_context(|| format!("Failed to write report to {}", args.output.display()))?;

    // Print summary
    println!("\n📊 Analysis Summary:");
    println!("   Files with issues: {}", report.files.len());
    println!("   Total issues: {}", summary.total);
    println!(
        "   - 🔴 Critical: {} | 🟠 High: {} | 🟡 Medium: {} | 🟢 Low: {}",
        summary.critical, summary.high, summary.medium, summary.low
    );
    println!("   Duration: {:.1}s", duration);
    println!(
        "\n✅ Audit complete! Report saved to: {}",
        args.output.display()
    );

    // Check --fail-on threshold
    if let Some(fail_level) = args.fail_on {
        let threshold_severity = fail_on_to_severity(fail_level);
        let has_issues_above = issues.iter().any(|i| i.severity >= threshold_severity);

        if has_issues_above {
            eprintln!(
                "\n⛔ Issues found at or above {:?} severity. Failing (exit code 2).",
                fail_level
            );
            return Ok(2);
        }
    }

    Ok(0)
}

/// Handle --dry-run: scan files, print what would be analyzed, exit.
fn handle_dry_run(repo_path: &PathBuf, scan_config: &scanner::ScanConfig) -> Result<i32> {
    println!("\n🔍 Dry run: scanning files (no LLM call)...\n");

    let file_scanner = scanner::FileScanner::new(repo_path.clone(), scan_config.clone());
    let files = file_scanner.scan()?;

    if files.is_empty() {
        println!("   No matching source files found.");
    } else {
        println!("   Found {} files that would be analyzed:\n", files.len());
        for file in &files {
            println!("     📄 {} ({} bytes)", file.path, file.size);
        }
        println!("\n   Total: {} files", files.len());
    }

    println!("\n✅ Dry run complete. No LLM calls were made.");
    Ok(0)
}

/// Convert FailOnLevel to Severity for comparison.
fn fail_on_to_severity(level: FailOnLevel) -> Severity {
    match level {
        FailOnLevel::Low => Severity::Low,
        FailOnLevel::Medium => Severity::Medium,
        FailOnLevel::High => Severity::High,
        FailOnLevel::Critical => Severity::Critical,
    }
}

/// Load configuration from file or use defaults.
fn load_config(args: &Args) -> Result<Config> {
    // Try explicit config path
    if let Some(ref config_path) = args.config {
        info!("Loading config from: {}", config_path.display());
        return Config::load(config_path);
    }

    // Try default location
    match Config::load_default() {
        Ok(Some(config)) => {
            info!("Loaded default config from .yoauditor.toml");
            Ok(config)
        }
        Ok(None) => {
            debug!("No config file found, using defaults");
            Ok(Config::default())
        }
        Err(e) => {
            warn!("Failed to load config: {}", e);
            Ok(Config::default())
        }
    }
}

/// Get the repository path (clone if needed).
async fn get_repository(args: &Args) -> Result<PathBuf> {
    // Use local directory if specified
    if let Some(ref local) = args.local {
        info!("Using local directory: {}", local.display());
        return Ok(local.clone());
    }

    // Clone the repository
    let repo_url = args.repo_url();
    info!("Cloning repository: {}", repo_url);

    let clone_options = repo::CloneOptions {
        branch: args.branch.clone(),
        depth: Some(1), // Shallow clone
        show_progress: !args.quiet,
        target_dir: None,
    };

    let clone_result = repo::clone_repository(repo_url, clone_options)?;
    Ok(clone_result.into_path())
}
