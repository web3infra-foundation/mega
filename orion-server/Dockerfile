# ────── Stage 0: Chef ──────
FROM rust:1.93.0-bookworm AS chef
RUN cargo install cargo-chef --version 0.1.73
WORKDIR /app

# ────── Stage 1: Planner ──────
FROM chef AS planner

COPY Cargo.toml Cargo.lock ./
COPY api-model/Cargo.toml api-model/
COPY ceres/Cargo.toml ceres/
COPY common/Cargo.toml common/
COPY context/Cargo.toml context/
COPY extensions/rag/chat/Cargo.toml extensions/rag/chat/
COPY extensions/rag/index/Cargo.toml extensions/rag/index/
COPY extensions/observatory/Cargo.toml extensions/observatory/
COPY io-orbit/Cargo.toml io-orbit/
COPY jupiter/Cargo.toml jupiter/
COPY jupiter/callisto/Cargo.toml jupiter/callisto/
COPY mono/Cargo.toml mono/
COPY orion/Cargo.toml orion/
COPY orion/audit/Cargo.toml orion/audit/
COPY orion/td_util/Cargo.toml orion/td_util/
COPY orion/buck/Cargo.toml orion/buck/
COPY orion-server/Cargo.toml orion-server/
COPY orion-server/bellatrix/Cargo.toml orion-server/bellatrix/
COPY saturn/Cargo.toml saturn/
COPY scorpio/Cargo.toml scorpio/
COPY vault/Cargo.toml vault/

RUN cargo chef prepare --bin orion-server --recipe-path recipe.json

# ────── Stage 2: Builder ──────
FROM chef AS builder

RUN apt-get update && \
    apt-get install -y --no-install-recommends libclang-dev && \
    rm -rf /var/lib/apt/lists/*


COPY --from=planner /app/recipe.json recipe.json

# -- Step 1: cook dependencies with cache
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    cargo chef cook --release --recipe-path recipe.json

COPY . .

# -- Step 2: build binary into normal target directory
ENV CARGO_TARGET_DIR=/app/target
RUN cargo build -p orion-server --release

# ────── Stage 3: Runtime ──────
FROM debian:bookworm-slim

RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy built executable
COPY --from=builder /app/target/release/orion-server /usr/local/bin/orion-server

# Reduce image size (optional)
RUN strip /usr/local/bin/orion-server || true

EXPOSE 8004

CMD ["orion-server"]