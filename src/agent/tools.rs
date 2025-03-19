//! Tool definitions for the code analysis agent.
//!
//! This module defines the tools that the LLM can use to interact
//! with the repository being analyzed.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use tracing::debug;

/// Tool definition for Ollama's tool-calling API.
#[derive(Debug, Clone, Serialize)]
pub struct ToolDefinition {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: FunctionDefinition,
}

#[derive(Debug, Clone, Serialize)]
pub struct FunctionDefinition {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

/// A tool call made by the LLM.
#[derive(Debug, Clone, Deserialize)]
pub struct ToolCall {
    pub function: FunctionCall,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: Value,
}

/// Result of executing a tool.
#[derive(Debug, Clone, Serialize)]
pub struct ToolResult {
    pub success: bool,
    pub output: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl ToolResult {
    pub fn success(output: String) -> Self {
        Self {
            success: true,
            output,
            error: None,
        }
    }

    pub fn error(message: String) -> Self {
        Self {
            success: false,
            output: String::new(),
            error: Some(message),
        }
    }
}

/// The tools executor that handles tool calls.
pub struct ToolExecutor {
    /// Root directory of the repository being analyzed.
    repo_root: PathBuf,
    /// Collected issues during analysis.
    issues: Vec<ReportedIssue>,
    /// File scanner for respecting config.
    scanner: crate::scanner::FileScanner,
}

/// An issue reported by the LLM via the report_issue tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportedIssue {
    pub file_path: String,
    pub line_number: usize,
    pub severity: String,
    pub category: String,
    pub title: String,
    pub description: String,
    pub suggestion: String,
}

impl ToolExecutor {
    /// Create a new tool executor for the given repository.
    pub fn new(repo_root: PathBuf, scan_config: crate::scanner::ScanConfig) -> Self {
        let scanner = crate::scanner::FileScanner::new(repo_root.clone(), scan_config);
        Self {
            repo_root,
            issues: Vec::new(),
            scanner,
        }
    }

    /// Get all reported issues.
    pub fn get_issues(&self) -> &[ReportedIssue] {
        &self.issues
    }

    /// Execute a tool call and return the result.
    pub fn execute(&mut self, tool_call: &ToolCall) -> ToolResult {
        let name = &tool_call.function.name;
        let args = &tool_call.function.arguments;

        debug!("Executing tool: {} with args: {:?}", name, args);

        match name.as_str() {
            "list_files" => self.list_files(args),
            "read_file" => self.read_file(args),
            "search_code" => self.search_code(args),
            "get_file_info" => self.get_file_info(args),
            "report_issue" => self.report_issue(args),
            "finish_analysis" => ToolResult::success("done".to_string()),
            _ => ToolResult::error(format!("Unknown tool: {}", name)),
        }
    }

    /// List files in a directory.
    fn list_files(&self, args: &Value) -> ToolResult {
        let dir = args
            .get("directory")
            .and_then(|v| v.as_str())
            .unwrap_or(".");

        match self.scanner.list_directory(dir) {
            Ok(entries) => ToolResult::success(entries.join("\n")),
            Err(e) => ToolResult::error(e.to_string()),
        }
    }

    /// Read the contents of a file.
    fn read_file(&self, args: &Value) -> ToolResult {
        let path = match args.get("path").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return ToolResult::error("Missing required parameter: path".to_string()),
        };

        let full_path = self.repo_root.join(path);

        // Security check with canonicalization
        match std::fs::canonicalize(&self.repo_root) {
            Ok(canonical_repo) => {
                match std::fs::canonicalize(&full_path) {
                    Ok(canonical_path) => {
                        if !canonical_path.starts_with(&canonical_repo) {
                            return ToolResult::error("Access denied: path outside repository".to_string());
                        }
                    }
                    Err(_) => {
                        // Path doesn't exist or can't be canonicalized
                        if !full_path.starts_with(&self.repo_root) {
                            return ToolResult::error("Access denied: path outside repository".to_string());
                        }
                    }
                }
            }
            Err(_) => {
                // Fallback to basic check
                if !full_path.starts_with(&self.repo_root) {
                    return ToolResult::error("Access denied: path outside repository".to_string());
                }
            }
        }

        if !full_path.exists() {
            return ToolResult::error(format!("File not found: {}", path));
        }

        if !full_path.is_file() {
            return ToolResult::error(format!("Not a file: {}", path));
        }

        // Use scanner's configured max file size
        if let Ok(_metadata) = std::fs::metadata(&full_path) {
            if !self.scanner.matches(&full_path) {
                return ToolResult::error(
                    "File too large or doesn't match scan criteria.".to_string(),
                );
            }
        }

        match std::fs::read_to_string(&full_path) {
            Ok(content) => ToolResult::success(content),
            Err(e) => ToolResult::error(format!("Failed to read file: {}", e)),
        }
    }

    /// Search for a pattern in the codebase.
    fn search_code(&self, args: &Value) -> ToolResult {
        let pattern = match args.get("pattern").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return ToolResult::error("Missing required parameter: pattern".to_string()),
        };

        let max_results = args
            .get("max_results")
            .and_then(|v| v.as_u64())
            .unwrap_or(10) as usize;

        let mut results = Vec::new();
        self.search_in_dir(&self.repo_root, pattern, &mut results, max_results);

        if results.is_empty() {
            ToolResult::success(String::new())
        } else {
            ToolResult::success(results.join("\n"))
        }
    }

    fn search_in_dir(&self, dir: &Path, pattern: &str, results: &mut Vec<String>, max: usize) {
        if results.len() >= max {
            return;
        }

        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };

        for entry in entries.flatten() {
            if results.len() >= max {
                break;
            }

            let path = entry.path();

            if path.is_dir() {
                self.search_in_dir(&path, pattern, results, max);
            } else if path.is_file() && self.scanner.matches(&path) {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    for (line_num, line) in content.lines().enumerate() {
                        if line.contains(pattern) {
                            let rel_path = path.strip_prefix(&self.repo_root).unwrap_or(&path);
                            results.push(format!("{}:{}", rel_path.display(), line_num + 1));
                            if results.len() >= max {
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    /// Get information about a file.
    fn get_file_info(&self, args: &Value) -> ToolResult {
        let path = match args.get("path").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return ToolResult::error("Missing required parameter: path".to_string()),
        };

        let full_path = self.repo_root.join(path);

        // Security check with canonicalization
        match std::fs::canonicalize(&self.repo_root) {
            Ok(canonical_repo) => {
                match std::fs::canonicalize(&full_path) {
                    Ok(canonical_path) => {
                        if !canonical_path.starts_with(&canonical_repo) {
                            return ToolResult::error("Access denied: path outside repository".to_string());
                        }
                    }
                    Err(_) => {
                        if !full_path.starts_with(&self.repo_root) {
                            return ToolResult::error("Access denied: path outside repository".to_string());
                        }
                    }
                }
            }
            Err(_) => {
                if !full_path.starts_with(&self.repo_root) {
                    return ToolResult::error("Access denied: path outside repository".to_string());
                }
            }
        }

        if !full_path.exists() {
            return ToolResult::error(format!("File not found: {}", path));
        }

        let metadata = match std::fs::metadata(&full_path) {
            Ok(m) => m,
            Err(e) => return ToolResult::error(format!("Failed to get metadata: {}", e)),
        };

        let language = full_path
            .extension()
            .and_then(|e| e.to_str())
            .map(|ext| match ext {
                "rs" => "Rust",
                "py" => "Python",
                "js" => "JavaScript",
                "ts" => "TypeScript",
                "go" => "Go",
                "java" => "Java",
                "c" | "h" => "C",
                "cpp" | "hpp" => "C++",
                _ => ext,
            })
            .unwrap_or("Unknown");

        let line_count = if full_path.is_file() {
            std::fs::read_to_string(&full_path)
                .map(|c| c.lines().count())
                .unwrap_or(0)
        } else {
            0
        };

        // Minimal format: lang,lines,bytes
        ToolResult::success(format!("{},{},{}", language, line_count, metadata.len()))
    }

    /// Report an issue found in the code.
    fn report_issue(&mut self, args: &Value) -> ToolResult {
        let issue = ReportedIssue {
            file_path: args
                .get("file_path")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string(),
            line_number: args
                .get("line_number")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as usize,
            severity: args
                .get("severity")
                .and_then(|v| v.as_str())
                .unwrap_or("medium")
                .to_string(),
            category: args
                .get("category")
                .and_then(|v| v.as_str())
                .unwrap_or("general")
                .to_string(),
            title: args
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("Issue")
                .to_string(),
            description: args
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            suggestion: args
                .get("suggestion")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
        };

        debug!("Reported issue: {:?}", issue);
        self.issues.push(issue);

        ToolResult::success("ok".to_string())
    }
}

/// Get the tool definitions for the Ollama API.
pub fn get_tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "list_files".to_string(),
                description: "List files and directories in the repository. Use this to explore the codebase structure.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "directory": {
                            "type": "string",
                            "description": "Directory path relative to repository root. Use '.' for root."
                        }
                    },
                    "required": []
                }),
            },
        },
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "read_file".to_string(),
                description: "Read the contents of a source code file. Returns content with line numbers.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Path to the file relative to repository root"
                        }
                    },
                    "required": ["path"]
                }),
            },
        },
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "search_code".to_string(),
                description: "Search for a pattern in the codebase. Returns matching lines with file and line numbers.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "pattern": {
                            "type": "string",
                            "description": "Text pattern to search for"
                        },
                        "max_results": {
                            "type": "integer",
                            "description": "Maximum number of results (default: 10)"
                        }
                    },
                    "required": ["pattern"]
                }),
            },
        },
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "get_file_info".to_string(),
                description: "Get metadata about a file (size, language, line count).".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Path to the file"
                        }
                    },
                    "required": ["path"]
                }),
            },
        },
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "report_issue".to_string(),
                description: "Report a code issue you found. Call this for each bug, security issue, or code quality problem.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "file_path": {
                            "type": "string",
                            "description": "Path to the file with the issue"
                        },
                        "line_number": {
                            "type": "integer",
                            "description": "Line number where the issue is"
                        },
                        "severity": {
                            "type": "string",
                            "enum": ["critical", "high", "medium", "low"],
                            "description": "Severity of the issue"
                        },
                        "category": {
                            "type": "string",
                            "enum": ["bug", "security", "performance", "style", "best_practice"],
                            "description": "Category of the issue"
                        },
                        "title": {
                            "type": "string",
                            "description": "Short title describing the issue"
                        },
                        "description": {
                            "type": "string",
                            "description": "Detailed description of the issue"
                        },
                        "suggestion": {
                            "type": "string",
                            "description": "How to fix the issue"
                        }
                    },
                    "required": ["file_path", "line_number", "severity", "category", "title", "description"]
                }),
            },
        },
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "finish_analysis".to_string(),
                description: "Call this when you have finished analyzing the repository and reported all issues.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
            },
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scanner::ScanConfig;
    use tempfile::TempDir;

    #[test]
    fn test_tool_executor_list_files() {
        let temp_dir = TempDir::new().unwrap();
        std::fs::write(temp_dir.path().join("test.rs"), "fn main() {}").unwrap();
        std::fs::create_dir(temp_dir.path().join("src")).unwrap();

        let executor = ToolExecutor::new(temp_dir.path().to_path_buf(), ScanConfig::default());
        let result = executor.list_files(&json!({"directory": "."}));

        assert!(result.success);
        assert!(result.output.contains("test.rs"));
        assert!(result.output.contains("src"));
    }

    #[test]
    fn test_tool_executor_read_file() {
        let temp_dir = TempDir::new().unwrap();
        std::fs::write(
            temp_dir.path().join("test.rs"),
            "fn main() {\n    println!(\"Hello\");\n}",
        )
        .unwrap();

        let executor = ToolExecutor::new(temp_dir.path().to_path_buf(), ScanConfig::default());
        let result = executor.read_file(&json!({"path": "test.rs"}));

        assert!(result.success);
        assert!(result.output.contains("fn main()"));
        // Raw content, no line numbers - minimal tokens
    }

    #[test]
    fn test_tool_definitions() {
        let tools = get_tool_definitions();
        assert_eq!(tools.len(), 6);

        let names: Vec<_> = tools.iter().map(|t| t.function.name.as_str()).collect();
        assert!(names.contains(&"list_files"));
        assert!(names.contains(&"read_file"));
        assert!(names.contains(&"report_issue"));
        assert!(names.contains(&"finish_analysis"));
    }
}
