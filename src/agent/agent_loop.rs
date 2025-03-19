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
            max_iterations: 50,
            timeout_seconds: 300,
            single_call_mode: false,
            max_context_messages: 10, // Keep last 10 tool results
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
        }
    }

    /// Run the analysis and return found issues.
    pub async fn run_analysis(&mut self) -> Result<Vec<ReportedIssue>> {
        if self.config.single_call_mode {
            self.run_single_call_analysis().await
        } else {
            self.run_tool_calling_analysis().await
        }
    }

    /// Single-call mode: Read all files, send in ONE API call
    async fn run_single_call_analysis(&mut self) -> Result<Vec<ReportedIssue>> {
        info!("Starting single-call analysis (efficient mode)");

        // Use the unified scanner
        let scanner = FileScanner::new(self.repo_root.clone(), self.scan_config.clone());
        let files = scanner.collect_files()?;
        info!("Collected {} source files", files.len());

        if files.is_empty() {
            warn!("No source files found to analyze");
            return Ok(vec![]);
        }

        // Build the prompt with all file contents
        let mut prompt = String::new();
        prompt.push_str("Analyze the following code files and report any issues.\n\n");
        prompt.push_str("For each issue found, output it in this exact JSON format:\n");
        prompt.push_str(r#"{"file_path": "path/to/file.rs", "line_number": 42, "severity": "high", "category": "security", "title": "Issue Title", "description": "Description", "suggestion": "How to fix"}"#);
        prompt.push_str("\n\nOutput one JSON object per line for each issue. Only output JSON, no other text.\n\n");
        prompt.push_str("=== FILES TO ANALYZE ===\n\n");

        for (path, content) in &files {
            prompt.push_str(&format!("### FILE: {}\n```\n{}\n```\n\n", path, content));
        }

        prompt.push_str("=== END OF FILES ===\n\n");
        prompt.push_str("Now analyze and output issues as JSON (one per line):");

        // Send single API call
        info!("Sending single API request with all files...");
        let response = self.send_simple_prompt(&prompt).await?;

        // Parse issues from response
        let issues = self.parse_issues_from_response(&response);
        info!("Parsed {} issues from response", issues.len());

        Ok(issues)
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
    async fn run_tool_calling_analysis(&mut self) -> Result<Vec<ReportedIssue>> {
        info!("Starting agentic code analysis (tool-calling mode)");

        // Initialize with system prompt
        self.messages.push(ChatMessage {
            role: "system".to_string(),
            content: AGENT_SYSTEM_PROMPT.to_string(),
            tool_calls: None,
        });

        // Initial user message
        self.messages.push(ChatMessage {
            role: "user".to_string(),
            content: "Please analyze this repository for code issues. Start by exploring the file structure, then read and analyze the source code files. Report any bugs, security issues, performance problems, or code quality concerns you find. When you've finished analyzing all relevant files, call finish_analysis.".to_string(),
            tool_calls: None,
        });

        // Agent loop
        for iteration in 0..self.config.max_iterations {
            debug!("Agent iteration {}", iteration + 1);

            // Get LLM response
            let response = self.chat_with_tools().await?;

            // Check if there are tool calls
            if let Some(tool_calls) = response.tool_calls {
                let mut should_finish = false;

                for tool_call in tool_calls {
                    let tool_name = &tool_call.function.name;

                    if tool_name == "finish_analysis" {
                        info!("Agent finished analysis");
                        should_finish = true;
                        break;
                    }

                    // Execute tool
                    let call = ToolCall {
                        function: crate::agent::tools::FunctionCall {
                            name: tool_call.function.name.clone(),
                            arguments: tool_call.function.arguments.clone(),
                        },
                    };

                    let result = self.tool_executor.execute(&call);

                    // Add tool result to messages
                    self.messages.push(ChatMessage {
                        role: "tool".to_string(),
                        content: if result.success {
                            result.output
                        } else {
                            format!("Error: {}", result.error.unwrap_or_default())
                        },
                        tool_calls: None,
                    });

                    // Sliding window: prune old tool messages to save context
                    self.prune_old_messages();

                    info!("Tool {} executed", tool_name);
                }

                if should_finish {
                    break;
                }
            } else {
                // No tool calls - LLM sent a text response
                let content = response.content.to_lowercase();
                if content.contains("complete")
                    || content.contains("finished")
                    || content.contains("done")
                {
                    info!("Agent indicated completion via text");
                    break;
                }

                self.messages.push(ChatMessage {
                    role: "assistant".to_string(),
                    content: response.content,
                    tool_calls: None,
                });

                self.messages.push(ChatMessage {
                    role: "user".to_string(),
                    content: "Please continue analyzing or call finish_analysis if you're done."
                        .to_string(),
                    tool_calls: None,
                });
            }
        }

        let issues = self.tool_executor.get_issues().to_vec();
        info!("Analysis complete. Found {} issues.", issues.len());

        Ok(issues)
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
const SINGLE_CALL_SYSTEM_PROMPT: &str = r#"You are an expert code reviewer and security auditor. 
Analyze the provided code files and identify issues.
Output each issue as a JSON object on its own line.
Only output valid JSON, no explanations or markdown."#;

/// System prompt for tool-calling mode
const AGENT_SYSTEM_PROMPT: &str = r#"You are an expert code reviewer and security auditor. Your task is to analyze a code repository for issues.

## Available Tools

You have access to tools to explore and analyze the codebase:
- `list_files(directory)` - List files in a directory
- `read_file(path)` - Read a source file's contents
- `search_code(pattern)` - Search for code patterns
- `get_file_info(path)` - Get file metadata
- `report_issue(...)` - Report a found issue
- `finish_analysis()` - Call when done

## Your Process

1. Start by listing files in the root directory
2. Identify source code files to analyze
3. Read and analyze each relevant source file
4. For each issue found, call report_issue
5. When finished, call finish_analysis

## Issues to Look For

- Bugs: Logic errors, null pointer risks, race conditions
- Security: SQL injection, XSS, hardcoded secrets
- Performance: Inefficient algorithms, memory issues
- Code Quality: Duplicated code, complex functions

Be thorough but focused. Report real issues with specific line numbers.
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
