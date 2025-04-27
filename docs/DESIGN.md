# YoAuditor — Design & Architecture

High-level design, patterns, and data flow. For contributors and the curious.

---

## 1. Big Picture

YoAuditor is a **pipeline**: clone → scan → analyze → report. The “analyze” step is pluggable: either **single-call** (one big LLM request) or **tool-calling** (agent loop). Config and CLI are merged once; the rest of the app is stateless per run.

```
  ┌─────────┐     ┌─────────┐     ┌──────────────┐     ┌───────────┐
  │  Clone  │ ──▶│  Scan   │ ──▶│   Analyze    │ ──▶│  Report   │
  │  Repo   │     │  Files  │     │ (LLM mode)   │     │ (MD/JSON) │
  └─────────┘     └─────────┘     └──────────────┘     └───────────┘
       │                │                    │                 │
       ▼                ▼                    ▼                 ▼
   repo::cloner    scanner::*           agent::*          report::*
   (git2)          (one source         (strategy)         (generator)
                    of truth)
```

---

## 2. Design Patterns

| Pattern | Where | What it does |
|--------|--------|----------------|
| **Pipeline / Orchestrator** | `main::run_audit` | Runs the four stages in order, passes config and paths, handles `--dry-run` and `--init-config` early exits. |
| **Strategy** | `agent::CodeAnalysisAgent::run_analysis` | Picks implementation at runtime: `run_single_call_analysis()` vs `run_tool_calling_analysis()`. Same interface (`Vec<ReportedIssue>`), different algorithms. |
| **Tool Executor** | `agent::tools::ToolExecutor` | Executes tool calls from the LLM (e.g. `list_files`, `read_file`, `report_issue`). Single place that talks to the filesystem and collects issues; agent loop only does HTTP + message handling. |
| **Config Merge** | `config::Config::merge_with_args` | CLI overrides config. Optional flags (e.g. `--timeout`, `--single-call`) only override when set; required-ish ones (e.g. model, ollama_url) always come from CLI (which has defaults). |
| **Adapter** | `scanner::ScanConfig::from(&ScannerConfig)` | Turns `config::ScannerConfig` into `scanner::ScanConfig` so the scanner stays independent of the TOML shape. |
| **Single source of truth (scanner)** | `scanner::FileScanner` | All file discovery and filtering (extensions, excludes, max size, max files) lives here. Both single-call and tool-calling use it; no duplicated walk logic. |

---

## 3. Data Flow

### 3.1 Config and CLI

```
  .yoauditor.toml (optional)     CLI (Args)
           │                            │
           └──────────┬─────────────────┘
                      ▼
              Config::merge_with_args
                      │
                      ▼
         Config (model, scanner, …)
         ScanConfig = ScannerConfig.into()
```

- One `Config` is built per run (file → merge args).
- Scanner gets a `ScanConfig` derived from `Config.scanner` (and thus from CLI when those args are set).

### 3.2 Repository and Scan

```
  --repo URL | --local DIR
           │
           ▼
  get_repository(args)  →  PathBuf (repo root)
           │
           ▼
  FileScanner::new(repo_root, scan_config)
           │
  ┌────────┴────────┐
  │                 │
  ▼                 ▼
  .scan()           .collect_files()
  Vec<ScannedFile>  HashMap<path, content>
  (dry-run, tools)  (single-call only)
```

- Clone/local is handled once; the same `PathBuf` is used for scanner and agent.
- Dry-run stops after scan and never calls the agent.

### 3.3 Analysis (Strategy)

```
  CodeAnalysisAgent::run_analysis()
           │
           ├── single_call_mode ──▶ run_single_call_analysis()
           │                              │
           │                              ├── scanner.collect_files()
           │                              ├── build prompt (all files)
           │                              ├── send_simple_prompt()
           │                              └── parse_issues_from_response()
           │
           └── !single_call_mode ──▶ run_tool_calling_analysis()
                                        │
                                        ├── push system + user message
                                        └── loop:
                                              chat_with_tools()
                                              → tool_calls? → ToolExecutor::execute()
                                              → push tool results, prune context
                                              → finish_analysis? → break
                                              → else push assistant message, continue
```

- Single-call: one request, no tools; response is parsed as JSON lines into `ReportedIssue`s.
- Tool-calling: classic agent loop (message → LLM → tool_calls → execute → append results → repeat until `finish_analysis` or max iterations).

### 3.4 Report

```
  Vec<ReportedIssue>  (from agent)
           │
           ▼
  map into models::Issue (severity, category, …)
           │
  optional: filter by --min-severity
           │
           ▼
  analysis::group_by_file(issues)
           │
           ▼
  Vec<AnalyzedFile> + ReportMetadata + IssueSummary
           │
           ▼
  Report  →  generate_markdown_report() | generate_json_report()
           │
           ▼
  write to args.output  ;  optional: --fail-on → exit 2
```

- Domain model lives in `models`; report layer only formats and writes.
- Exit code: 0 = success, 1 = error, 2 = issues above `--fail-on` threshold.

---

## 4. Module Map

| Module | Role |
|--------|------|
| **main** | Entrypoint, logging, `run_audit` orchestration, exit codes, `--init-config` / dry-run. |
| **cli** | Args (clap), validation, `FailOnLevel` / `OutputFormat`. No I/O beyond parsing. |
| **config** | Load TOML, default config, merge CLI into config, `default_toml()` for `--init-config`. |
| **scanner** | `ScanConfig`, `FileScanner`, `ScannedFile`. Walk tree, filter by config, `list_directory` for tools. Single place that knows “what is a source file”. |
| **repo** | Clone (git2), `CloneOptions`, `CloneResult`. No knowledge of scanner or agent. |
| **agent** | `AgentConfig`, `CodeAnalysisAgent`. Strategy: single-call vs tool-calling. HTTP to Ollama, message list, context pruning. |
| **agent::tools** | Tool definitions for Ollama, `ToolExecutor`, `ReportedIssue`. Executes tools and collects issues; uses scanner for list/read and for path safety. |
| **models** | `Issue`, `Severity`, `AnalyzedFile`, `Report`, `ReportMetadata`, `IssueSummary`. Shared domain types. |
| **analysis** | `group_by_file`, and helpers (e.g. aggregate, sort by severity). Used when building the report. |
| **report** | Markdown/JSON generation from `Report`. No knowledge of agent or scanner. |

---

## 5. Boundaries and Safety

- **Path safety**: Tool executor and scanner resolve paths under `repo_root` and use canonicalization where needed to avoid escape (e.g. `..` or symlinks).
- **Config vs CLI**: All “effective” settings go through `Config` after merge; the rest of the app only sees `Config` and `ScanConfig`, not raw `Args`.
- **LLM boundary**: Only the agent talks to Ollama. Parsing (e.g. JSON lines for single-call, tool_calls for agentic) is inside the agent. Rest of the app only sees `Vec<ReportedIssue>`.

---

## 6. File Layout (conceptual)

```
  docs/
    DESIGN.md          ← this file

  src/
    main.rs            # Orchestrator, exit codes
    cli.rs             # CLI surface
    config.rs          # Config load + merge
    models.rs          # Domain types
    scanner/           # File discovery (single source of truth)
    repo/              # Clone
    agent/             # Analysis strategy + HTTP
      tools.rs         # Tool definitions + executor
      agent_loop.rs    # Single-call + tool-calling impl
    analysis/          # Grouping and aggregates
    report/            # Markdown/JSON output
```

---

## 7. Extension Points

- **New analysis mode**: Add another branch in `CodeAnalysisAgent::run_analysis()` and implement a `run_*_analysis()` that returns `Vec<ReportedIssue>`.
- **New tool**: Add definition in `get_tool_definitions()`, handle in `ToolExecutor::execute()`, keep path safety and scanner use in mind.
- **New output format**: Add a variant to `OutputFormat` and a `generate_*_report()` that consumes `&Report`.
- **New config section**: Add to `Config` in `config.rs`, extend `merge_with_args` if CLI should override it.

---

*Last updated to match the post-rename, post–unified-scanner design.*
