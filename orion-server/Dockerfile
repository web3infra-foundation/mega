# ────── Stage 0: Chef ──────
FROM rust:1.93-bookworm AS chef
RUN cargo install cargo-chef
WORKDIR /app

# ────── Stage 1: Planner ──────
FROM chef AS planner
COPY . .

WORKDIR /app
RUN cargo chef prepare --bin orion-server --recipe-path recipe.json

# ────── Stage 2: Builder ──────
FROM chef AS builder

RUN apt-get update && \
    apt-get install -y --no-install-recommends libclang-dev && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=planner /app/recipe.json recipe.json

RUN cargo chef cook --bin orion-server --release --recipe-path recipe.json

COPY . .

RUN cargo build -p orion-server --release

# ────── Stage 3: Runtime ──────
FROM debian:bookworm-slim

RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates curl && \
    rm -rf /var/lib/apt/lists/*

# Copy built executable
COPY --from=builder /app/target/release/orion-server /usr/local/bin/orion-server

# Reduce image size (optional)
RUN strip /usr/local/bin/orion-server || true

EXPOSE 8004

CMD ["orion-server"]