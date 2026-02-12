# YoAuditor 🔍

LLM-powered code auditor for GitHub repos. Audit any repo for bugs, security issues, and performance problems using local AI. Built in Rust.

## ✨ Two Analysis Modes

Instead of a one-size-fits-all approach, YoAuditor gives you two ways to audit code:

**Single-call mode** sends all files in one LLM request — fast and efficient.

**Tool-calling (agentic) mode** lets the LLM explore the codebase autonomously:

1. 🔍 **Explores** — Uses `list_files` to discover the project structure
2. 📖 **Reads** — Calls `read_file` to examine specific files
3. 🔎 **Searches** — Uses `search_code` to find patterns
4. 🐛 **Reports** — Calls `report_issue` for each problem found
5. ✅ **Finishes** — Calls `finish_analysis` when done

This is similar to how Claude or GPT-4 use tools to interact with codebases!

## Features

- 🤖 **Agentic Analysis**: LLM autonomously explores and analyzes the codebase
- 🛠️ **Tool Calling**: Uses Ollama's tool-calling API for structured interactions
- 🔒 **Security Scanning**: Identifies potential security vulnerabilities
- 🐛 **Bug Detection**: Finds logic errors, null pointer risks, race conditions
- ⚡ **Performance Issues**: Detects inefficient algorithms and blocking I/O
- 📊 **Comprehensive Reports**: Generates detailed Markdown/JSON reports with line numbers
- 🚦 **CI-Friendly**: `--fail-on` flag with exit codes for gating builds
- 🔧 **Fully Configurable**: `.yoauditor.toml` for model, extensions, excludes, timeouts

## Prerequisites

1. **Rust** (latest stable): [Install Rust](https://rustup.rs/)
2. **LLM Backend** (choose one):
   - **Ollama** (recommended): [Install Ollama](https://ollama.ai/download)
   - **llama.cpp server**: [Build from source](https://github.com/ggerganov/llama.cpp)

3. **A model that supports tool calling**:
   ```bash
   # For Ollama:
   ollama pull llama3.2:latest     # Fast, good tool support
   ollama pull qwen3-coder:480b-cloud  # Cloud model, excellent

   # For llama.cpp:
   # Download a GGUF model and run:
   ./llama-server -m model.gguf --port 8080
   ```

> ⚠️ **Important**: For agentic mode, the model MUST support tool/function calling. Use `--single-call` if your model doesn't.

## Installation

```bash
# Clone and build
git clone https://github.com/sharafdin/yoauditor.git
cd yoauditor
cargo build --release

# Install system-wide (optional)
cargo install --path .
```

## Docker

```bash
# Build the image
docker build -t yoauditor .

# Run (Ollama must be reachable; use host network or pass Ollama URL)
docker run --rm yoauditor --repo https://github.com/owner/repo.git --ollama-url http://host.docker.internal:11434

# With output mounted so you can read the report on the host
docker run --rm -v "$(pwd)/out:/app/out" yoauditor \
  --repo https://github.com/owner/repo.git \
  --ollama-url http://host.docker.internal:11434 \
  --output /app/out/yoaudit_report.md
```

On Linux, use `--network host` and `--ollama-url http://localhost:11434` if Ollama runs on the host.

See **[docs/DOCKER.md](docs/DOCKER.md)** for more options and Docker Compose.

## Usage

### Basic Usage

```bash
# Analyze a GitHub repository
yoauditor --repo https://github.com/owner/repo.git

# Use a specific model (must support tool calling!)
yoauditor --repo https://github.com/owner/repo.git --model llama3.2:latest

# Analyze a local directory
yoauditor --repo local --local ./my-project

# Preview which files would be analyzed (no LLM call)
yoauditor --repo https://github.com/owner/repo.git --dry-run

# Generate a default config file
yoauditor --init-config
```

### All Options

```
yoauditor [OPTIONS]

Options:
  -r, --repo <URL>             GitHub repository URL to analyze
  -m, --model <MODEL>          Ollama model name [default: deepseek-coder:33b]
  -o, --output <FILE>          Output file path [default: yoaudit_report.md]
      --max-files <COUNT>      Maximum files to analyze [default: 100]
      --ollama-url <URL>       Ollama API endpoint [default: http://localhost:11434]
  -c, --config <FILE>          Path to configuration file
  -v, --verbose                Enable verbose logging
  -q, --quiet                  Quiet mode (minimal output)
  -b, --branch <BRANCH>        Specific branch to analyze
      --extensions <EXTS>      File extensions to include (comma-separated)
      --exclude <PATTERNS>     Patterns to exclude (comma-separated)
      --local <DIR>            Analyze a local directory instead of cloning
      --format <FORMAT>        Output format: markdown, json [default: markdown]
      --timeout <SECS>         LLM request timeout [default: from config or 900s]
      --single-call            Force single-call mode
      --no-single-call         Force tool-calling (agentic) mode
      --fail-on <LEVEL>        Fail (exit 2) if issues at or above level
      --min-severity <LEVEL>   Only include issues at or above level in report
      --dry-run                Scan files without calling the LLM
      --init-config            Generate default .yoauditor.toml
  -h, --help                   Print help
  -V, --version                Print version
```

## How It Works (Architecture)

```
┌───────────────────────────────────────────────────────────┐
│                       YoAuditor                           │
├───────────────────────────────────────────────────────────┤
│  1. Clone Repository                                      │
│     └──> /tmp/repo_clone                                  │
│                                                           │
│  2. Scan Source Files (respects config)                    │
│     ├── extensions, excludes, max_file_size               │
│     └── unified scanner used by both modes                │
│                                                           │
│  3. Analyze (choose one mode):                            │
│                                                           │
│     Single-call             Tool-calling (agentic)        │
│     ┌──────────────┐       ┌──────────────────────┐      │
│     │ Read all files│       │ LLM calls tools:     │      │
│     │ Send in ONE   │       │  list_files()        │      │
│     │ API request   │       │  read_file()         │      │
│     │ Parse issues  │       │  search_code()       │      │
│     └──────────────┘       │  get_file_info()     │      │
│                             │  report_issue()      │      │
│                             │  finish_analysis()   │      │
│                             └──────────────────────┘      │
│                                                           │
│  4. Generate Report from collected issues                 │
└───────────────────────────────────────────────────────────┘
```

## Configuration

Create `.yoauditor.toml` in your project (or run `yoauditor --init-config`):

```toml
[general]
output = "yoaudit_report.md"
verbose = false

[model]
name = "llama3.2:latest"
ollama_url = "http://localhost:11434"
temperature = 0.1
timeout_seconds = 900
# true = single-call (efficient), false = tool-calling (agentic)
single_call_mode = true

[scanner]
max_files = 100
extensions = ["rs", "py", "js", "ts", "go", "java", "c", "cpp"]
excludes = [".git", "target", "node_modules", "vendor"]
max_file_size = 1048576
```

## CI Usage

```bash
# Fail the build if any high or critical issues are found
yoauditor --repo https://github.com/owner/repo.git --fail-on high

# Only report critical issues, output as JSON
yoauditor --repo https://github.com/owner/repo.git --min-severity critical --format json
```

**Exit codes:**

| Code | Meaning |
| :--: | :------ |
| 0    | Success (no issues above threshold) |
| 1    | Runtime error (connection, config, etc.) |
| 2    | Issues found above `--fail-on` threshold |

## Project Structure

```
yoauditor/
├── src/
│   ├── main.rs              # CLI entry and workflow
│   ├── cli.rs               # Argument parsing
│   ├── config.rs            # Configuration handling
│   ├── models.rs            # Data structures
│   ├── scanner/
│   │   └── mod.rs           # Unified file scanner
│   ├── repo/
│   │   ├── mod.rs           # Module exports
│   │   └── cloner.rs        # Git repository cloning
│   ├── agent/
│   │   ├── mod.rs           # Module exports
│   │   ├── tools.rs         # Tool definitions
│   │   └── agent_loop.rs    # Agentic loop
│   ├── analysis/
│   │   ├── mod.rs           # Module exports
│   │   └── aggregator.rs    # Issue aggregation
│   └── report/
│       ├── mod.rs           # Module exports
│       └── generator.rs     # Report generation
├── Cargo.toml
└── .yoauditor.toml
```

## Troubleshooting

### "Cannot connect to Ollama"
- Ensure Ollama is running: `ollama serve`
- Check the URL: default is `http://localhost:11434`

### "Request timed out"
- Increase timeout: `--timeout 1800` or edit `timeout_seconds` in `.yoauditor.toml`
- Single-call mode with large repos can take 10+ minutes

### "Model not found"
- Pull it first: `ollama pull llama3.2:latest`

### "Tool calling not working"
- Make sure you're using a model that supports tool calling
- Try: `llama3.2:latest`, `llama3.1:latest`, or `mistral:latest`
- Or use `--single-call` mode which works with any model
- Models like `deepseek-coder` may NOT support tools

### Analysis takes too long
- Use `--single-call` mode for faster analysis
- Use a faster/smaller model: `llama3.2:latest`
- Reduce scope: `--max-files 20`
- Check Ollama logs for issues

## Docs

| Doc | Description |
|-----|-------------|
| [docs/DESIGN.md](docs/DESIGN.md) | Design patterns, architecture, data flow |
| [docs/CONFIGURATION.md](docs/CONFIGURATION.md) | Full `.yoauditor.toml` reference |
| [docs/CLI.md](docs/CLI.md) | Exhaustive CLI options and examples |
| [docs/EXIT_CODES.md](docs/EXIT_CODES.md) | Exit codes 0 / 1 / 2 and CI usage |
| [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md) | Local dev, tests, where to change what |
| [docs/DOCKER.md](docs/DOCKER.md) | Docker build, run, and compose |

## License

MIT License

## Acknowledgments

- [Ollama](https://ollama.ai) — Local LLM runtime with tool calling support
- [git2](https://github.com/rust-lang/git2-rs) — Git bindings for Rust
