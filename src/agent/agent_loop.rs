//! Agent loop for tool-based code analysis.
//!
//! This module implements both:
//! - Single-call mode: Send all files in one API call (efficient for cloud models)
//! - Tool-calling mode: LLM explores with tools (for capable local models)

use crate::agent::tools::{get_tool_definitions, ReportedIssue, ToolCall, ToolExecutor};
use crate::scanner::{FileScanner, ScanConfig};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use std::time::Duration;
use tracing::{debug, info, warn};

/// Configuration for the agent.
#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub ollama_url: String,
    pub model_name: String,
    pub temperature: f32,
    pub max_iterations: usize,
    pub timeout_seconds: u64,
    /// Use single-call mode instead of tool-calling
    pub single_call_mode: bool,
    /// Max tool results to keep in context (sliding window)
    pub max_context_messages: usize,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            ollama_url: "http://localhost:11434".to_string(),
            model_name: "llama3.2:latest".to_string(),
            temperature: 0.1,
            max_iterations: 200,
            timeout_seconds: 300,
            single_call_mode: false,
            max_context_messages: 50, // Keep last 50 messages for better file content retention
        }
    }
}

/// Message in the chat history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCallMessage>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallMessage {
    pub function: ToolCallFunction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallFunction {
    pub name: String,
    pub arguments: Value,
}

/// Ollama chat API request.
#[derive(Debug, Serialize)]
struct OllamaChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<Value>,
    stream: bool,
    options: OllamaOptions,
}

#[derive(Debug, Serialize)]
struct OllamaOptions {
    temperature: f32,
}

/// Ollama chat API response.
#[derive(Debug, Deserialize)]
struct OllamaChatResponse {
    message: ResponseMessage,
    #[allow(dead_code)] // Response field, used for future stream handling
    done: bool,
}

#[derive(Debug, Deserialize)]
struct ResponseMessage {
    #[allow(dead_code)] // Response field
    role: String,
    content: String,
    #[serde(default)]
    tool_calls: Option<Vec<ToolCallMessage>>,
}

/// The code analysis agent.
pub struct CodeAnalysisAgent {
    config: AgentConfig,
    http_client: reqwest::Client,
    tool_executor: ToolExecutor,
    messages: Vec<ChatMessage>,
    repo_root: PathBuf,
    scan_config: ScanConfig,
    /// Tracks which files the agent has read (for agentic mode).
    files_read: Vec<String>,
    /// Tracks which files the agent has reported issues for.
    files_reported: std::collections::HashSet<String>,
}

/// Result of running analysis: issues found and (when known) total files analyzed.
pub struct AnalysisResult {
    pub issues: Vec<ReportedIssue>,
    /// Total files sent to the LLM (single-call mode). None in tool-calling mode.
    pub total_files_analyzed: Option<usize>,
}

impl CodeAnalysisAgent {
    /// Create a new agent for analyzing a repository.
    pub fn new(config: AgentConfig, repo_root: PathBuf, scan_config: ScanConfig) -> Self {
        info!(
            "Initializing agent with model {} for repo: {}",
            config.model_name,
            repo_root.display()
        );

        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            config,
            http_client,
            tool_executor: ToolExecutor::new(repo_root.clone(), scan_config.clone()),
            messages: Vec::new(),
            repo_root: repo_root.clone(),
            scan_config,
            files_read: Vec::new(),
            files_reported: std::collections::HashSet::new(),
        }
    }

    /// Run the analysis and return found issues plus total files analyzed when known.
    pub async fn run_analysis(&mut self) -> Result<AnalysisResult> {
        if self.config.single_call_mode {
            self.run_single_call_analysis().await
        } else {
            self.run_tool_calling_analysis().await
        }
    }

    /// Single-call mode: Read all files, send in ONE API call
    async fn run_single_call_analysis(&mut self) -> Result<AnalysisResult> {
        info!("Starting single-call analysis (efficient mode)");

        // Use the unified scanner
        let scanner = FileScanner::new(self.repo_root.clone(), self.scan_config.clone());
        let files = scanner.collect_files()?;
        let total_files = files.len();
        info!("Collected {} source files", total_files);

        if files.is_empty() {
            warn!("No source files found to analyze");
            return Ok(AnalysisResult {
                issues: vec![],
                total_files_analyzed: Some(0),
            });
        }

        // Build the prompt with all file contents
        let mut prompt = String::new();
        prompt.push_str(&format!(
            "You are auditing a codebase with {} source files. Analyze EVERY file below for security vulnerabilities, bugs, performance issues, and code quality problems.\n\n",
            files.len()
        ));
        prompt.push_str("For each issue found, output one JSON object per line in this exact format:\n");
        prompt.push_str(r#"{"file_path": "path/to/file.rs", "line_number": 42, "severity": "high", "category": "security", "title": "SQL Injection in query builder", "description": "User input is concatenated directly into SQL query without parameterization, allowing an attacker to execute arbitrary SQL.", "suggestion": "Use parameterized queries or a query builder with bound parameters."}"#);
        prompt.push_str("\n\n");
        prompt.push_str("Requirements:\n");
        prompt.push_str("- Analyze ALL files, not just a few\n");
        prompt.push_str("- Use exact line numbers from the source code\n");
        prompt.push_str("- severity must be one of: critical, high, medium, low\n");
        prompt.push_str("- category must be one of: security, bug, performance, code-quality\n");
        prompt.push_str("- title should be concise (under 10 words)\n");
        prompt.push_str("- description should explain WHAT and WHY\n");
        prompt.push_str("- suggestion should explain HOW to fix\n");
        prompt.push_str("- Only report real issues you are confident about\n");
        prompt.push_str("- Output ONLY JSON lines, no other text\n\n");
        prompt.push_str(&format!("=== {} FILES TO ANALYZE ===\n\n", files.len()));

        for (i, (path, content)) in files.iter().enumerate() {
            prompt.push_str(&format!(
                "--- FILE {}/{}: {} ---\n```\n{}\n```\n\n",
                i + 1,
                files.len(),
                path,
                content
            ));
        }

        prompt.push_str("=== END OF FILES ===\n\n");
        prompt.push_str("Now analyze every file above and output issues as JSON (one per line). Remember: analyze ALL files, use accurate line numbers, and only report real issues:");

        // Send single API call
        info!("Sending single API request with all files...");
        let response = self.send_simple_prompt(&prompt).await?;

        // Parse issues from response
        let issues = self.parse_issues_from_response(&response);
        info!("Parsed {} issues from response", issues.len());

        Ok(AnalysisResult {
            issues,
            total_files_analyzed: Some(total_files),
        })
    }

    /// Send a simple prompt (no tools) and get response
    async fn send_simple_prompt(&self, prompt: &str) -> Result<String> {
        let url = format!("{}/api/chat", self.config.ollama_url);

        let request = OllamaChatRequest {
            model: self.config.model_name.clone(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: SINGLE_CALL_SYSTEM_PROMPT.to_string(),
                    tool_calls: None,
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: prompt.to_string(),
                    tool_calls: None,
                },
            ],
            tools: vec![],
            stream: false,
            options: OllamaOptions {
                temperature: self.config.temperature,
            },
        };

        let response = self
            .http_client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    anyhow::anyhow!("Request timed out after {}s", self.config.timeout_seconds)
                } else if e.is_connect() {
                    anyhow::anyhow!("Cannot connect to Ollama at {}", self.config.ollama_url)
                } else {
                    anyhow::anyhow!("Failed to send request: {}", e)
                }
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Ollama API error {}: {}", status, body));
        }

        let chat_response: OllamaChatResponse = response
            .json()
            .await
            .context("Failed to parse Ollama response")?;

        Ok(chat_response.message.content)
    }

    /// Parse issues from LLM response (JSON lines format)
    fn parse_issues_from_response(&self, response: &str) -> Vec<ReportedIssue> {
        let mut issues = Vec::new();

        for line in response.lines() {
            let line = line.trim();
            if line.is_empty() || !line.starts_with('{') {
                continue;
            }

            // Try to parse as JSON
            if let Ok(json) = serde_json::from_str::<Value>(line) {
                if let Some(issue) = self.json_to_issue(&json) {
                    issues.push(issue);
                }
            }
        }

        issues
    }

    fn json_to_issue(&self, json: &Value) -> Option<ReportedIssue> {
        Some(ReportedIssue {
            file_path: json["file_path"].as_str()?.to_string(),
            line_number: json["line_number"].as_u64().unwrap_or(0) as usize,
            severity: json["severity"].as_str().unwrap_or("medium").to_string(),
            category: json["category"].as_str().unwrap_or("general").to_string(),
            title: json["title"].as_str().unwrap_or("Issue").to_string(),
            description: json["description"].as_str().unwrap_or("").to_string(),
            suggestion: json["suggestion"].as_str().unwrap_or("").to_string(),
        })
    }

    /// Tool-calling mode: LLM uses tools to explore repository
    async fn run_tool_calling_analysis(&mut self) -> Result<AnalysisResult> {
        info!("Starting agentic code analysis (tool-calling mode)");

        // Initialize with system prompt
        self.messages.push(ChatMessage {
            role: "system".to_string(),
            content: AGENT_SYSTEM_PROMPT.to_string(),
            tool_calls: None,
        });

        // Initial user message — must be directive and clear
        self.messages.push(ChatMessage {
            role: "user".to_string(),
            content: r#"Analyze this repository for code issues. Follow these steps exactly:

1. Call list_files(".") to discover the project structure
2. Call read_file for EACH source code file (skip docs, configs, tests, fixtures)
3. After reading each file, IMMEDIATELY call report_issue for every bug, security vulnerability, performance problem, or code quality issue you find in that file. Do NOT wait until the end.
4. After you have read and analyzed ALL source files and reported ALL issues, call finish_analysis

IMPORTANT:
- You MUST call report_issue for each issue. Do NOT just describe issues in text.
- Call report_issue IMMEDIATELY after reading each file, before moving to the next file.
- If you find no issues in a file, move to the next file.
- Do NOT call finish_analysis until you have read ALL source files."#.to_string(),
            tool_calls: None,
        });

        // Agent loop
        let mut consecutive_no_tool_calls = 0;
        for iteration in 0..self.config.max_iterations {
            debug!("Agent iteration {}", iteration + 1);

            // Get LLM response
            let response = self.chat_with_tools().await?;

            // Check if there are tool calls
            if let Some(tool_calls) = response.tool_calls {
                consecutive_no_tool_calls = 0;
                let mut should_finish = false;

                // Execute ALL tool calls in this batch, collect results
                let mut tool_results: Vec<(String, String)> = Vec::new();
                let mut just_read_file: Option<String> = None;

                for tool_call in &tool_calls {
                    let tool_name = &tool_call.function.name;

                    if tool_name == "finish_analysis" {
                        // Before finishing, check if there are unanalyzed files
                        let unanalyzed: Vec<_> = self.files_read.iter()
                            .filter(|f| !self.files_reported.contains(*f))
                            .cloned()
                            .collect();

                        if !unanalyzed.is_empty() && self.tool_executor.get_issues().is_empty() {
                            info!("Agent tried to finish but {} files unanalyzed — nudging", unanalyzed.len());
                            tool_results.push((tool_name.clone(),
                                format!("WAIT — you read {} files but reported 0 issues. Please go back and call report_issue for issues in: {}",
                                    unanalyzed.len(), unanalyzed.join(", "))));
                            continue;
                        }

                        let issues_count = self.tool_executor.get_issues().len();
                        info!("Agent finished analysis with {} issues reported", issues_count);
                        should_finish = true;
                        break;
                    }

                    // Track read_file calls
                    if tool_name == "read_file" {
                        if let Some(path) = tool_call.function.arguments.get("path").and_then(|v| v.as_str()) {
                            if !self.files_read.contains(&path.to_string()) {
                                self.files_read.push(path.to_string());
                            }
                            just_read_file = Some(path.to_string());
                        }
                    }

                    // Track report_issue calls
                    if tool_name == "report_issue" {
                        if let Some(fp) = tool_call.function.arguments.get("file_path").and_then(|v| v.as_str()) {
                            self.files_reported.insert(fp.to_string());
                        }
                    }

                    // Execute tool
                    let call = ToolCall {
                        function: crate::agent::tools::FunctionCall {
                            name: tool_call.function.name.clone(),
                            arguments: tool_call.function.arguments.clone(),
                        },
                    };

                    let result = self.tool_executor.execute(&call);
                    let output = if result.success {
                        result.output.clone()
                    } else {
                        format!("Error: {}", result.error.unwrap_or_default())
                    };

                    info!("Tool {} executed", tool_name);
                    tool_results.push((tool_name.clone(), output));
                }

                // Add all tool results to messages AFTER executing the batch
                let reported_in_batch = tool_results.iter().any(|(name, _)| name == "report_issue");

                for (tool_name, output) in &tool_results {
                    // Truncate very large tool outputs to save context
                    let truncated = if output.len() > 8000 {
                        format!("{}... [truncated, {} bytes total]", &output[..8000], output.len())
                    } else {
                        output.clone()
                    };

                    // If this is a read_file result and the model didn't report issues in this batch,
                    // append a nudge directly to the tool result (saves a message slot)
                    let content = if tool_name == "read_file" && !reported_in_batch && !should_finish {
                        if let Some(ref fp) = just_read_file {
                            format!("[{}] {}\n\n⚠️ Now call report_issue for EACH issue in \"{}\" before reading the next file.", tool_name, truncated, fp)
                        } else {
                            format!("[{}] {}", tool_name, truncated)
                        }
                    } else {
                        format!("[{}] {}", tool_name, truncated)
                    };

                    self.messages.push(ChatMessage {
                        role: "tool".to_string(),
                        content,
                        tool_calls: None,
                    });
                }

                // Prune ONCE after all results are added
                self.prune_old_messages();

                if should_finish {
                    break;
                }
            } else {
                consecutive_no_tool_calls += 1;

                // No tool calls - LLM sent a text response
                let content = response.content.to_lowercase();
                if content.contains("complete")
                    || content.contains("finished")
                    || content.contains("done")
                {
                    let issues_count = self.tool_executor.get_issues().len();
                    if issues_count > 0 {
                        info!("Agent indicated completion via text with {} issues", issues_count);
                        break;
                    }
                    // If no issues reported yet, nudge the model
                    info!("Agent said done but reported 0 issues — nudging to use report_issue");
                }

                // After 3 consecutive text-only responses, strongly redirect
                let nudge = if consecutive_no_tool_calls >= 3 {
                    "You are not using tools. You MUST call report_issue for each issue you find. Please read the next source file and call report_issue for any issues, or call finish_analysis if truly done."
                } else if self.tool_executor.get_issues().is_empty() {
                    "You have not reported any issues yet. Please call report_issue for each issue you found while reading files. Do NOT describe issues in text — use the report_issue tool. If you need to read more files, call read_file first."
                } else {
                    "Please continue analyzing files or call finish_analysis if you're done."
                };

                self.messages.push(ChatMessage {
                    role: "user".to_string(),
                    content: nudge.to_string(),
                    tool_calls: None,
                });

                // Give up after too many text-only responses
                if consecutive_no_tool_calls >= 5 {
                    warn!("Agent sent 5 consecutive text responses without tool calls — stopping");
                    break;
                }
            }
        }

        let issues = self.tool_executor.get_issues().to_vec();
        let files_read_count = self.files_read.len();
        info!(
            "Analysis complete. Read {} files, reported issues in {} files, {} total issues.",
            files_read_count,
            self.files_reported.len(),
            issues.len()
        );

        Ok(AnalysisResult {
            issues,
            total_files_analyzed: if files_read_count > 0 {
                Some(files_read_count)
            } else {
                None
            },
        })
    }

    /// Prune old tool messages to keep context small (sliding window).
    fn prune_old_messages(&mut self) {
        // Keep: system prompt (first) + last N messages
        let max_keep = self.config.max_context_messages + 2; // +2 for system + initial user

        if self.messages.len() > max_keep {
            // Find where tool messages start (after system + initial user)
            let keep_start = 2; // Keep first 2 messages (system + initial user)
            let remove_count = self.messages.len() - max_keep;

            // Remove oldest tool-related messages (from index 2 onwards)
            if remove_count > 0 && self.messages.len() > keep_start + remove_count {
                self.messages.drain(keep_start..keep_start + remove_count);
                debug!("Pruned {} old messages to save context", remove_count);
            }
        }
    }

    /// Send a chat request with tools to Ollama.
    async fn chat_with_tools(&mut self) -> Result<ResponseMessage> {
        let url = format!("{}/api/chat", self.config.ollama_url);

        let tools = get_tool_definitions();
        let tools_json: Vec<Value> = tools
            .iter()
            .map(|t| serde_json::to_value(t).unwrap())
            .collect();

        let request = OllamaChatRequest {
            model: self.config.model_name.clone(),
            messages: self.messages.clone(),
            tools: tools_json,
            stream: false,
            options: OllamaOptions {
                temperature: self.config.temperature,
            },
        };

        debug!("Sending chat request with {} messages", self.messages.len());

        let response = self
            .http_client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    anyhow::anyhow!(
                        "Request timed out after {}s. Try a different model.",
                        self.config.timeout_seconds
                    )
                } else if e.is_connect() {
                    anyhow::anyhow!(
                        "Cannot connect to Ollama at {}. Is Ollama running?",
                        self.config.ollama_url
                    )
                } else {
                    anyhow::anyhow!("Failed to send request: {}", e)
                }
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Ollama API error {}: {}", status, body));
        }

        let chat_response: OllamaChatResponse = response
            .json()
            .await
            .context("Failed to parse Ollama response")?;

        self.messages.push(ChatMessage {
            role: "assistant".to_string(),
            content: chat_response.message.content.clone(),
            tool_calls: chat_response.message.tool_calls.clone(),
        });

        Ok(chat_response.message)
    }
}

/// System prompt for single-call mode
const SINGLE_CALL_SYSTEM_PROMPT: &str = r#"You are an expert code auditor specializing in security, bugs, and performance analysis.

## Your Task
Analyze EVERY provided source file. For each real issue found, output one JSON object per line. Output ONLY valid JSON lines — no markdown, no explanations, no commentary.

## Severity Levels (use exactly these strings)
- "critical": Exploitable vulnerabilities (SQLi, RCE, auth bypass, hardcoded secrets/keys, path traversal)
- "high": Likely bugs or security risks that could cause data loss, crashes, or privilege issues (unwrap on user input, division by zero, SSRF, XSS, insecure deserialization)
- "medium": Code that is fragile, error-prone, or has performance impact (N+1 queries, blocking I/O in loops, missing error handling, race conditions, unbounded growth)
- "low": Code quality and maintainability concerns (dead code, poor naming, missing validation on non-critical paths, inefficient string operations)

## Categories (use exactly these strings)
- "security": Vulnerabilities, secrets, auth issues, injection, XSS, CSRF
- "bug": Logic errors, crashes, panics, null/undefined risks, resource leaks
- "performance": Inefficient algorithms, N+1, blocking I/O, memory waste
- "code-quality": Maintainability, duplication, complexity, error handling

## Rules
1. Analyze EVERY file. Do not skip files even if they look simple.
2. Use accurate line numbers. If unsure, use the best approximation.
3. Do NOT report issues in test files, fixtures, or intentionally vulnerable demo code unless asked.
4. Avoid false positives: only report issues you are confident about.
5. Be specific: describe WHAT the issue is, WHY it matters, and HOW to fix it.
6. One JSON object per line. No arrays, no wrapping.
7. If a file has no issues, do not output anything for it."#;

/// System prompt for tool-calling mode
const AGENT_SYSTEM_PROMPT: &str = r#"You are an expert code auditor specializing in security, bugs, and performance analysis. Your task is to thoroughly analyze a code repository.

## Available Tools

- `list_files(directory)` — List files in a directory (start with root ".")
- `read_file(path)` — Read a source file's full contents
- `search_code(pattern)` — Search for patterns across the codebase (regex supported)
- `get_file_info(path)` — Get file metadata (size, language, line count)
- `report_issue(file_path, line_number, severity, category, title, description, suggestion)` — Report a found issue
- `finish_analysis()` — Call when you have analyzed all files and reported all issues

## Your Process

1. Call `list_files(".")` to discover the project structure
2. Identify ALL source code files (skip tests, fixtures, vendored/generated code)
3. Read and analyze EACH source file thoroughly using `read_file`
4. Use `search_code` to trace cross-file patterns (e.g. how user input flows, shared state)
5. For each real issue, call `report_issue` with accurate details
6. After analyzing ALL files, call `finish_analysis`

## Severity Levels (use exactly these strings)

- "critical": Exploitable vulnerabilities — SQLi, RCE, auth bypass, hardcoded secrets/keys, path traversal
- "high": Likely bugs or security risks — unwrap on user input, division by zero, SSRF, XSS, insecure deserialization
- "medium": Fragile or error-prone code — N+1 queries, blocking I/O in loops, missing error handling, race conditions, unbounded growth
- "low": Maintainability concerns — dead code, poor naming, missing validation on non-critical paths

## Categories (use exactly these strings)

- "security": Vulnerabilities, secrets, auth issues, injection, XSS, CSRF
- "bug": Logic errors, crashes, panics, null/undefined risks, resource leaks
- "performance": Inefficient algorithms, N+1, blocking I/O, memory waste
- "code-quality": Maintainability, duplication, complexity, error handling

## Rules

1. Be thorough: read and analyze EVERY source file, not just a few.
2. Use accurate line numbers from the file content you read.
3. Avoid false positives: only report issues you are confident about.
4. Be specific: describe WHAT the issue is, WHY it matters, and HOW to fix it.
5. Look for cross-file issues: how data flows between modules, shared mutable state, missing validation at boundaries.
6. Do NOT report issues in test files or intentionally vulnerable fixtures.
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_config_default() {
        let config = AgentConfig::default();
        assert_eq!(config.model_name, "llama3.2:latest");
        assert!(!config.single_call_mode);
    }
}
