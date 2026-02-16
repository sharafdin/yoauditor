# Configuration Reference

YoAuditor is configured via **`.yoauditor.toml`** (project or repo root). CLI flags override config. Generate a default file with:

```bash
yoauditor --init-config
```

---

## File location

- **Current directory:** `./.yoauditor.toml` (used if no `-c`/`--config` is passed).
- **Repo being audited:** If the cloned repo contains `.yoauditor.toml`, it is loaded after the initial config and merged; then CLI is applied again.

---

## `[general]`

| Option   | Type   | Default              | Description |
| -------- | ------ | -------------------- | ----------- |
| `output` | string | `"yoaudit_report.md"` | Default output file path for the report. |
| `verbose` | bool | `false` | Enable verbose logging. |
| `concurrency` | number | `4` | Number of concurrent file analyses (for future use). |

**Example:**

```toml
[general]
output = "yoaudit_report.md"
verbose = false
concurrency = 4
```

---

## `[model]`

| Option             | Type   | Default                | Description |
| ------------------ | ------ | ---------------------- | ----------- |
| `name`             | string | `"llama3.2:latest"`    | Ollama model name. |
| `ollama_url`       | string | `"http://localhost:11434"` | Ollama API base URL. |
| `temperature`      | number | `0.1`                  | LLM temperature (0.0–1.0). Lower = more deterministic. |
| `timeout_seconds`  | number | `900`                  | Request timeout in seconds. Single-call mode can take 10+ minutes. |
| `retries`         | number | `3`                    | Retries on transient failure (future use). |
| `single_call_mode`| bool   | `true`                 | `true` = single-call (one request); `false` = tool-calling (agentic). |

**Example:**

```toml
[model]
name = "llama3.2:latest"
ollama_url = "http://localhost:11434"
temperature = 0.1
timeout_seconds = 900
retries = 3
single_call_mode = true
```

---

## `[scanner]`

| Option           | Type     | Default | Description |
| ---------------- | -------- | ------- | ----------- |
| `max_files`      | number   | `100`   | Maximum number of files to scan/analyze. |
| `max_chunk_lines`| number   | `4000`  | Max lines per file chunk (for large-file handling). |
| `extensions`     | list     | (see below) | File extensions to include (without dot). |
| `excludes`       | list     | (see below) | Directory/file names to exclude (e.g. `node_modules`). |
| `max_file_size`  | number   | `1048576` (1 MiB) | Max file size in bytes; larger files are skipped. |

**Default extensions:** `rs`, `py`, `js`, `ts`, `jsx`, `tsx`, `go`, `java`, `c`, `cpp`, `h`, `hpp`, `cs`, `rb`, `php`, `swift`, `kt`, `scala`, `vue`, `svelte`, `sh`, `sql`, `lua`, `dart`, `ex`, `exs`, `tf`, `hcl`, `zig`.

**Default excludes:** `.git`, `target`, `node_modules`, `vendor`, `dist`, `build`, `__pycache__`, `.venv`, `venv`, `.idea`, `.vscode`.

**Example:**

```toml
[scanner]
max_files = 100
max_chunk_lines = 4000
extensions = ["rs", "py", "js", "ts", "go", "java", "c", "cpp"]
excludes = [".git", "target", "node_modules", "vendor"]
max_file_size = 1048576
```

---

## `[report]`

| Option              | Type   | Default | Description |
| ------------------- | ------ | ------- | ----------- |
| `include_snippets`  | bool   | `true`  | Include code snippets in the report. |
| `include_summaries` | bool   | `true`  | Include file summary sections. |
| `max_snippet_lines` | number | `10`    | Max lines per code snippet. |
| `group_by_file`     | bool   | `true`  | Group issues by file; `false` = group by severity. |

**Example:**

```toml
[report]
include_snippets = true
include_summaries = true
max_snippet_lines = 10
group_by_file = true
```

---

## Override with CLI

These flags override the config file when provided:

- `-m` / `--model` → `[model] name`
- `--ollama-url` → `[model] ollama_url`
- `-o` / `--output` → output path (config has no direct CLI name; general output is used as default)
- `--timeout` → `[model] timeout_seconds`
- `--single-call` → `[model] single_call_mode = true`
- `--no-single-call` → `[model] single_call_mode = false`
- `--max-files` → `[scanner] max_files`
- `--extensions` → `[scanner] extensions`
- `--exclude` → `[scanner] excludes`
- `-c` / `--config` → path to a different config file
