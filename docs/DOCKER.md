# Docker

Build and run YoAuditor with Docker. Ollama can run on the host or in another container; point YoAuditor at it with `--ollama-url`.

## Build

```bash
docker build -t yoauditor .
```

## Run

**Help / version:**

```bash
docker run --rm yoauditor --help
docker run --rm yoauditor --version
```

**Dry-run (no LLM):**

```bash
docker run --rm yoauditor --repo https://github.com/owner/repo.git --dry-run
```

**Full audit (Ollama on host):**

- **macOS / Windows:** `host.docker.internal` points to the host.

  ```bash
  docker run --rm yoauditor \
    --repo https://github.com/owner/repo.git \
    --ollama-url http://host.docker.internal:11434 \
    --model llama3.2:latest
  ```

- **Linux:** Use host network so the container can reach `localhost:11434`:

  ```bash
  docker run --rm --network host yoauditor \
    --repo https://github.com/owner/repo.git \
    --ollama-url http://localhost:11434 \
    --model llama3.2:latest
  ```

**Save report on host:**

```bash
mkdir -p out
docker run --rm -v "$(pwd)/out:/app/out" yoauditor \
  --repo https://github.com/owner/repo.git \
  --ollama-url http://host.docker.internal:11434 \
  --output /app/out/yoaudit_report.md
# Report is in ./out/yoaudit_report.md
```

## Docker Compose

```bash
# Build
docker compose build

# Run one-off audit (override default command)
docker compose run --rm yoauditor \
  --repo https://github.com/owner/repo.git \
  --ollama-url http://host.docker.internal:11434 \
  --output /app/out/yoaudit_report.md
```

Reports written to `/app/out` in the container appear in `./out` on the host (see `volumes` in `docker-compose.yml`).

## Image details

- **Base:** Multi-stage build; final image is `debian:bookworm-slim` with `yoauditor` binary and runtime libs (`libgit2-1.5`, `libssl3`, `ca-certificates`).
- **Entrypoint:** `yoauditor`. Pass any CLI args after the image name.
- **Workdir:** `/app`. Use `--output /app/out/report.md` and mount `./out:/app/out` to get the file on the host.
