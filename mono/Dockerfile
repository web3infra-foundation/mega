# ────── Stage 0: Chef ──────
# Downloads and installs cargo-chef (prebuilt binary, no Rust compilation needed).
FROM rust:1.95.0 AS chef

ARG TARGETARCH=amd64
ARG CARGO_CHEF_VERSION=0.1.73

WORKDIR /opt/mega

# Prebuilt cargo-chef (avoids compiling it on every cold planner layer).
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && case "$TARGETARCH" in \
         amd64) chef_arch=x86_64-unknown-linux-musl ;; \
         arm64) chef_arch=aarch64-unknown-linux-gnu ;; \
         *) echo "unsupported TARGETARCH=$TARGETARCH" >&2; exit 1 ;; \
       esac \
    && curl -fsSL "https://github.com/LukeMathWalker/cargo-chef/releases/download/v${CARGO_CHEF_VERSION}/cargo-chef-${chef_arch}.tar.gz" \
         | tar xz -C /usr/local/cargo/bin

# ────── Stage 1: Planner ──────
# Copies all Cargo.toml files; cargo-chef emits recipe.json describing the full dependency graph.
# This stage is cached unless a Cargo.toml changes.
FROM chef AS planner

WORKDIR /opt/mega

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
COPY orion-server/bellatrix/Cargo.toml orion-server/bellatrix/
COPY saturn/Cargo.toml saturn/
COPY vault/Cargo.toml vault/

# Named cache mounts let the builder stage inherit this cache via `from=planner`.
RUN --mount=type=cache,target=/usr/local/cargo/registry,id=mono-registry \
    --mount=type=cache,target=/usr/local/cargo/git,id=mono-git \
    cargo chef prepare --bin mono --recipe-path recipe.json

# ────── Stage 2: Builder ──────
# Installs system libs, builds all dependencies (cached), then builds the mono binary.
FROM chef AS builder

WORKDIR /opt/mega

# Install system dependencies required for building
RUN apt-get update && apt-get install -y --no-install-recommends \
    libssl-dev \
    ca-certificates \
    clang \
    mold \
    protobuf-compiler \
    libprotobuf-dev \
    && rm -rf /var/lib/apt/lists/*

# Faster linking for release builds.
ENV RUSTFLAGS="-C link-arg=-fuse-ld=mold"

# Inherit registry/git caches from the planner stage.
COPY --from=planner /opt/mega/recipe.json recipe.json

RUN --mount=type=cache,target=/usr/local/cargo/registry,id=mono-registry,from=planner \
    --mount=type=cache,target=/usr/local/cargo/git,id=mono-git,from=planner \
    cargo chef cook --release --recipe-path recipe.json

COPY . .

# Persist built binary into the image layer (buildkit cache mounts evaporate after build).
RUN --mount=type=cache,target=/usr/local/cargo/registry,id=mono-registry,from=planner \
    --mount=type=cache,target=/usr/local/cargo/git,id=mono-git,from=planner \
    --mount=type=cache,target=/opt/mega/target-cache \
    CARGO_TARGET_DIR=/opt/mega/target-cache cargo build --release -p mono && \
    mkdir -p /opt/mega/target/release && \
    cp /opt/mega/target-cache/release/mono /opt/mega/target/release/mono

# ────── Stage 3: Runtime ──────
FROM debian:trixie-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    less \
    libssl3t64 \
    && rm -rf /var/lib/apt/lists/*

# Copy the compiled binary and startup script
COPY --from=builder /opt/mega/target/release/mono /usr/local/bin/mono
COPY --from=builder /opt/mega/mono/start-mono.sh /usr/local/bin/start-mono.sh

# Ensure binaries are executable
RUN chmod +x /usr/local/bin/mono /usr/local/bin/start-mono.sh

# Optional volume for runtime data
VOLUME /opt/mega

# Default startup command
CMD ["bash", "-c", "/usr/local/bin/start-mono.sh"]
