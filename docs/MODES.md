# Analysis Modes: Single-Call vs Agentic

YoAuditor supports two analysis modes. This document explains how each works, when to use it, and how to switch.

---

## Overview

| Aspect | Single-call | Agentic (tool-calling) |
|--------|-------------|------------------------|
| **How it works** | Send all source files in one LLM request; model returns issues as JSON lines | LLM explores the repo with tools, reads files one-by-one, calls `report_issue` per finding |
| **API calls** | 1 (one big request) | Many (one per tool-calling round) |
| **Model requirement** | Any model (no tool-calling needed) | Model must support **tool/function calling** (e.g. Ollama tool calls) |
| **Speed** | Usually faster (one round-trip) | Slower (multiple round-trips) |
| **Context** | All files in one prompt (limited by model context window) | Sliding window; only recent tool results kept |
| **Best for** | Large/cloud models, quick audits, many small files | Local models with tool support, deeper per-file analysis |

---

## Single-call mode

### What it does

1. **Scan** — The same file scanner used in both modes discovers all source files (by extension, excludes, max size).
2. **Pack** — All file paths and contents are packed into one user prompt: “Analyze these N files, output one JSON object per line for each issue.”
3. **One request** — A single request is sent to the LLM (system prompt + user prompt with all files).
4. **Parse** — The response is parsed line-by-line for JSON issue objects and turned into the report.

### Flow

```
[Scanner] → collect files
     ↓
[Build prompt] → "FILE 1: path\n```content```" … "FILE N: …"
     ↓
[One API call] → model returns JSON lines
     ↓
[Parse issues] → report
```

### When to use

- You want **one shot**: send everything, get back issues.
- Your model **does not support tool-calling** (e.g. many cloud or older models).
- You prefer **speed** and **predictable cost** (one long request).
- The repo has **many small files** that fit in the model’s context.

### CLI / config

- **Force single-call:** `--single-call`
- **Config:** In `.yoauditor.toml`, `[model]` → `single_call_mode = true` (default in many setups)

### Limitations

- **Context limit** — Total size of all files must fit in the model’s context; very large repos may hit limits.
- **No exploration** — The model cannot “decide” what to read next; it only sees what you pack.
- **One shot** — If the model forgets a file or under-reports, there is no second pass.

---

## Agentic mode (tool-calling)

### What it does

1. **Start** — The LLM gets a system prompt describing the tools and a user prompt: “Analyze this repo; use tools to explore, then report issues; call `finish_analysis` when done.”
2. **Loop** — The model repeatedly:
   - Calls tools: `list_files`, `read_file`, `search_code`, `get_file_info`, `report_issue`, or `finish_analysis`.
   - Receives tool results (and optional nudges, e.g. “call report_issue for this file”).
   - Decides the next step.
3. **Finish** — When the model calls `finish_analysis`, the loop stops and collected `report_issue` results become the report.

### Tools available

| Tool | Purpose |
|------|--------|
| `list_files(directory)` | List files and dirs (e.g. `"."` for repo root). |
| `read_file(path)` | Read full contents of a file (path relative to repo root). |
| `search_code(pattern)` | Search for a text pattern; returns file:line matches. |
| `get_file_info(path)` | Get metadata (language, line count, size). |
| `report_issue(file_path, line_number, severity, category, title, description, suggestion)` | Report one finding. |
| `finish_analysis()` | Signal end of analysis. |

### Flow

```
[User prompt] → "Analyze repo, use tools, then finish_analysis"
     ↓
┌───[Agent loop]─────────────────────────────────────────────┐
│  [LLM response] → tool_calls: [read_file("api.js"), …]    │
│       ↓                                                    │
│  [Execute tools] → results + optional nudge               │
│       ↓                                                    │
│  [Append to context] → prune old messages (keep last N)   │
│       ↓                                                    │
│  [LLM again] → more tool_calls or finish_analysis          │
└─── until finish_analysis or max_iterations ────────────────┘
     ↓
[Report] from all report_issue() calls
```

### When to use

- Your model **supports tool-calling** (e.g. Ollama with a capable model).
- You want the model to **explore** the repo (list, read, search) and **report as it goes**.
- You are okay with **more API calls** and **longer runs** for potentially more thorough, per-file analysis.

### CLI / config

- **Force agentic:** `--no-single-call`
- **Config:** `[model]` → `single_call_mode = false`
- **Tuning (in code/defaults):** `max_iterations` (e.g. 200), `max_context_messages` (e.g. 50) so the agent has enough steps and context to read and report on many files.

### Limitations

- **Model support** — Must use a model that implements tool/function calling; otherwise use `--single-call`.
- **Slower** — Many round-trips; good for “deeper” runs, not for fastest possible audit.
- **Context window** — Only the last N messages are kept; very long runs can “forget” earlier file contents unless the agent reports soon after reading.

---

## Switching modes

### Command line

```bash
# Single-call (one big request)
yoauditor --repo local --local ./my-app --single-call -o report.md

# Agentic (tool-calling)
yoauditor --repo local --local ./my-app --no-single-call -o report.md
```

### Config file

In `.yoauditor.toml`:

```toml
[model]
name = "qwen3-coder:480b-cloud"
# Use single-call (one request, any model)
single_call_mode = true

# Or use agentic (tool-calling; model must support it)
# single_call_mode = false
```

If you pass `--single-call` or `--no-single-call`, it overrides the config.

---

## Comparison summary

| Goal | Prefer |
|------|--------|
| Fastest run, one request | Single-call |
| Model without tool-calling | Single-call |
| Explore + report step-by-step | Agentic |
| Maximum compatibility (any model) | Single-call |
| Deeper, tool-driven analysis | Agentic |

Both modes use the **same scanner** (extensions, excludes, max files) and produce the **same report format** (Markdown/JSON, severity, categories). The only difference is *how* the LLM sees the code and returns issues: all at once (single-call) or via tools (agentic).

For more on options and exit codes, see [CLI.md](CLI.md) and [EXIT_CODES.md](EXIT_CODES.md).
