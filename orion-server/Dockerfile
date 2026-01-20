# ────── Stage 1: Build ──────
FROM rust:1.92-bookworm AS builder

# Configure Cargo with increased retry attempts and timeout
RUN mkdir -p /root/.cargo && \
    cat > /root/.cargo/config.toml << 'EOF'
[net]
retry = 10
git-fetch-with-cli = true
EOF

# The parent directory of build context will be mounted as /app
WORKDIR /app

# Copy the entire project structure (from repository root)
COPY . .

# Enter subproject directory
WORKDIR /app/orion-server

# Build release binary
RUN cargo build --release

# ────── Stage 2: Runtime ──────
FROM debian:bookworm-slim

# Install runtime dependencies (including curl for health checks)
RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates curl && \
    rm -rf /var/lib/apt/lists/*

ENV BUILD_LOG_DIR="/tmp/buck2ctl"
RUN mkdir -p ${BUILD_LOG_DIR}

ENV PORT=8004

# Copy built executable
COPY --from=builder /app/target/release/orion-server /usr/local/bin/app

# Reduce image size (optional)
RUN strip /usr/local/bin/app || true

EXPOSE 8004

CMD ["app"]