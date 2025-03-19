# Development Guide

Quick reference for hacking on YoAuditor locally.

## Setup

```bash
git clone https://github.com/sharafdin/yoauditor.git
cd yoauditor
cargo build
cargo test
```

## Commands

| Command | Description |
|--------|-------------|
| `cargo build` | Debug build. |
| `cargo build --release` | Release build (optimized binary in `target/release/yoauditor`). |
| `cargo test` | Run all tests. |
| `cargo fmt` | Format code. |
| `cargo clippy` | Lints; fix any new warnings. |
| `cargo run -- --help` | Run the CLI and show help. |
| `cargo run -- --repo https://github.com/sharafdin/sharafdin --dry-run` | Dry-run without LLM. |

## Running an audit locally

You need Ollama running and a model pulled:

```bash
ollama serve
ollama pull llama3.2:latest
```

Then:

```bash
cargo run --release -- --repo https://github.com/owner/repo --model llama3.2:latest
```

Or use a local directory:

```bash
cargo run --release -- --repo local --local ./path/to/repo
```

## Where to change what

| Goal | Where to look |
|------|----------------|
| Add a CLI flag | `src/cli.rs` (Args), then `src/config.rs` (merge_with_args) if it’s a config override. |
| Change scan behavior | `src/scanner/mod.rs` (ScanConfig, FileScanner). |
| Change analysis (single-call vs tool-calling) | `src/agent/agent_loop.rs`. |
| Add or change a tool | `src/agent/tools.rs` (definitions + ToolExecutor). |
| Change report layout | `src/report/generator.rs`. |
| Change exit codes / fail-on | `src/main.rs` (run_audit, fail_on check). |

## Tests

- Unit tests are in the same file as the code, under `#[cfg(test)] mod tests`.
- Run: `cargo test`.
- Add tests for new behavior; keep existing tests green.

## Docs

- **Design & architecture:** [docs/DESIGN.md](DESIGN.md)
- **Config reference:** [docs/CONFIGURATION.md](CONFIGURATION.md)
- **CLI reference:** [docs/CLI.md](CLI.md)
- **Exit codes:** [docs/EXIT_CODES.md](EXIT_CODES.md)

## Contributing

See [CONTRIBUTING.md](../CONTRIBUTING.md) for branch workflow, style, and PR guidelines.
