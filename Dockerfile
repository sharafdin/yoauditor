# Build stage
FROM rust:1-bookworm AS builder

WORKDIR /app

# Install build dependencies for git2 (libgit2, ssl, pkg-config)
RUN apt-get update && apt-get install -y \
    libgit2-dev \
    libssl-dev \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests and source
COPY Cargo.toml Cargo.lock ./
COPY src ./src

# Build release binary
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libgit2-1.5 \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/yoauditor /usr/local/bin/yoauditor

ENTRYPOINT ["yoauditor"]
CMD ["--help"]
