//! LLM agent modules for code analysis.
//!
//! This module provides the tool-calling agent for analyzing code repositories.

pub mod agent_loop;
pub mod tools;

pub use agent_loop::{AgentConfig, CodeAnalysisAgent};
