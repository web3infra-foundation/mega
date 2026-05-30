# ────── Stage 0: Chef ──────
# Downloads and installs cargo-chef (prebuilt binary, no Rust compilation needed).
# NOTE: This stage is intentionally identical to `mono/Dockerfile`'s chef stage
# (same base image, WORKDIR, cargo-chef version) so BuildKit can share the
# resulting layer between the two demo images.
FROM rust:1.96.0 AS chef

ARG CARGO_CHEF_VERSION=0.1.77

WORKDIR /opt/mega

# Install cargo-chef from the release installer script.
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && curl --proto '=https' --tlsv1.2 -LsSf \
         "https://github.com/LukeMathWalker/cargo-chef/releases/download/v${CARGO_CHEF_VERSION}/cargo-chef-installer.sh" \
         | sh

# ────── Stage 1: Planner ──────
# Copies all Cargo.toml files; cargo-chef emits recipe.json describing the full dependency graph.
# This stage is cached unless a Cargo.toml changes.
FROM chef AS planner

WORKDIR /opt/mega
ARG TARGETARCH

COPY Cargo.toml ./
COPY api-model/Cargo.toml api-model/
COPY ceres/Cargo.toml ceres/
COPY common/Cargo.toml common/
COPY context/Cargo.toml context/
COPY io-orbit/Cargo.toml io-orbit/
COPY jupiter/Cargo.toml jupiter/
COPY jupiter/callisto/Cargo.toml jupiter/callisto/
COPY mono/Cargo.toml mono/
COPY orion/Cargo.toml orion/
COPY orion/audit/Cargo.toml orion/audit/
COPY orion/td_util/Cargo.toml orion/td_util/
COPY orion/buck/Cargo.toml orion/buck/
COPY orion-scheduler/Cargo.toml orion-scheduler/
COPY orion-server/Cargo.toml orion-server/
COPY clients/orion-client/Cargo.toml clients/orion-client/
COPY saturn/Cargo.toml saturn/
COPY vault/Cargo.toml vault/

RUN --mount=type=cache,target=/usr/local/cargo/registry,id=mega-cargo-registry-${TARGETARCH},sharing=locked \
    --mount=type=cache,target=/usr/local/cargo/git,id=mega-cargo-git-${TARGETARCH},sharing=locked \
    cargo chef prepare --bin orion-server --recipe-path recipe.json

# ────── Stage 2: Builder ──────
# Installs system libs, builds all dependencies (cached), then builds the orion-server binary.
FROM chef AS builder

WORKDIR /opt/mega
ARG TARGETARCH

# Install system dependencies required for building.
# `libclang-dev` is required by some bindgen-using crates pulled in via orion-server.
RUN apt-get update && apt-get install -y --no-install-recommends \
    libssl-dev \
    ca-certificates \
    clang \
    libclang-dev \
    mold \
    protobuf-compiler \
    libprotobuf-dev \
    && rm -rf /var/lib/apt/lists/*

# Faster linking for release builds.
ENV RUSTFLAGS="-C link-arg=-fuse-ld=mold"

COPY --from=planner /opt/mega/recipe.json recipe.json

# Step 1: cook dependencies with shared cargo cache.
RUN --mount=type=cache,target=/usr/local/cargo/registry,id=mega-cargo-registry-${TARGETARCH},sharing=locked \
    --mount=type=cache,target=/usr/local/cargo/git,id=mega-cargo-git-${TARGETARCH},sharing=locked \
    cargo chef cook --profile demo --recipe-path recipe.json

COPY . .

# Step 2: build binary, then copy out of the cache mount so the runtime stage can COPY it.
# A buildkit cache mount is not committed into the image layer, so we build into a cached dir
# and persist the final binary into a regular filesystem path before the layer is sealed.
RUN --mount=type=cache,target=/usr/local/cargo/registry,id=mega-cargo-registry-${TARGETARCH},sharing=locked \
    --mount=type=cache,target=/usr/local/cargo/git,id=mega-cargo-git-${TARGETARCH},sharing=locked \
    --mount=type=cache,target=/opt/mega/target-cache \
    CARGO_TARGET_DIR=/opt/mega/target-cache cargo build --profile demo -p orion-server && \
    mkdir -p /opt/mega/target/demo && \
    cp /opt/mega/target-cache/demo/orion-server /opt/mega/target/demo/orion-server

# ────── Stage 3: Runtime ──────
FROM debian:trixie-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    libssl3t64 \
    && rm -rf /var/lib/apt/lists/*

# Copy built executable
COPY --from=builder /opt/mega/target/demo/orion-server /usr/local/bin/orion-server

EXPOSE 8004

CMD ["orion-server"]
