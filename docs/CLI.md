# CLI Reference

Exhaustive reference for the `yoauditor` command. For a shorter overview, see the [README](../README.md).

## Synopsis

```text
yoauditor [OPTIONS]
```

`--repo` is required unless `--init-config` is used.

## Options

### Repository & input

| Option | Short | Type | Default | Description |
|--------|-------|------|---------|-------------|
| `--repo` | `-r` | URL | *(required)* | GitHub repo URL (e.g. `https://github.com/owner/repo.git`). |
| `--local` | | DIR | — | Analyze this local directory instead of cloning. Use with `--repo local` or any placeholder. |
| `--branch` | `-b` | BRANCH | default branch | Branch to clone/analyze. |

### Model & API

| Option | Short | Type | Default | Env | Description |
|--------|-------|------|---------|-----|-------------|
| `--model` | `-m` | NAME | `deepseek-coder:33b` | `YOAUDITOR_MODEL` | Ollama model name. |
| `--ollama-url` | | URL | `http://localhost:11434` | `OLLAMA_URL` | Ollama API base URL. |
| `--temperature` | | FLOAT | `0.1` | — | LLM temperature (0.0–1.0). |
| `--timeout` | | SECS | (from config or 900) | — | Request timeout in seconds. |
| `--single-call` | | flag | — | — | Force single-call mode. |
| `--no-single-call` | | flag | — | — | Force tool-calling (agentic) mode. |

### Output

| Option | Short | Type | Default | Description |
|--------|-------|------|---------|-------------|
| `--output` | `-o` | FILE | `yoaudit_report.md` | Output report path. |
| `--format` | | FORMAT | `markdown` | Output format: `markdown` or `json`. |

### Scanner

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `--max-files` | COUNT | `100` | Max files to analyze. |
| `--max-chunk-lines` | LINES | `4000` | Max lines per chunk for large files. |
| `--extensions` | EXTS | (from config) | Comma-separated extensions (e.g. `rs,py,js`). |
| `--exclude` | PATTERNS | (from config) | Comma-separated exclude patterns. |

### Config & behavior

| Option | Short | Type | Default | Description |
|--------|-------|------|---------|-------------|
| `--config` | `-c` | FILE | `.yoauditor.toml` | Path to config file. |
| `--fail-on` | | LEVEL | — | Exit with code 2 if any issue has severity >= LEVEL (`critical`, `high`, `medium`, `low`). |
| `--min-severity` | | LEVEL | — | Only include issues with severity >= LEVEL in the report. |
| `--dry-run` | | flag | — | Scan files only; do not call the LLM. |
| `--init-config` | | flag | — | Write default `.yoauditor.toml` and exit. |
| `--concurrency` | | NUM | `4` | Concurrency (for future use). |
| `--skip-clone` | | flag | — | Skip cloning if directory exists (behavior may depend on clone logic). |

### Logging & misc

| Option | Short | Description |
|--------|-------|-------------|
| `--verbose` | `-v` | Verbose logging. |
| `--quiet` | `-q` | Minimal output. |
| `--help` | `-h` | Print help. |
| `--version` | `-V` | Print version. |

## Exit codes

| Code | Meaning |
|------|--------|
| 0 | Success (or no issues above `--fail-on` threshold). |
| 1 | Error (e.g. connection, config, clone failure). |
| 2 | Issues found at or above `--fail-on` severity. |

See [EXIT_CODES.md](EXIT_CODES.md) for CI usage.

## Examples

```bash
# Basic audit
yoauditor --repo https://github.com/owner/repo.git

# Specific model and output
yoauditor --repo https://github.com/owner/repo.git --model llama3.2:latest -o report.md

# Local directory
yoauditor --repo local --local ./my-project

# JSON report, only high+ severity
yoauditor --repo https://github.com/owner/repo.git --format json --min-severity high

# CI: fail if high or critical
yoauditor --repo https://github.com/owner/repo.git --fail-on high

# Dry run (no LLM)
yoauditor --repo https://github.com/owner/repo.git --dry-run

# Generate config
yoauditor --init-config

# Long timeout for large repo
yoauditor --repo https://github.com/owner/repo.git --timeout 1800
```
