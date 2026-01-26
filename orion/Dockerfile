# =============================================================================
# Mega Dev Image - Multi-stage Dockerfile
# =============================================================================
# This Dockerfile builds a unified development image containing:
# - Orion Worker (build execution client)
# - Scorpio (FUSE filesystem service)
# - Buck2 (build tool)
#
# Supports: linux/amd64, linux/arm64
# Build targets: runtime (slim), dev (with Rust toolchain)
# Optional build targets: runtime-nocache, dev-nocache (disable BuildKit cargo cache mounts)
# =============================================================================

# -----------------------------------------------------------------------------
# Build Arguments
# -----------------------------------------------------------------------------
# Use "bookworm" for latest stable, or specify version like "1.83"
ARG RUST_VERSION=1.92-bookworm
ARG RUST_TOOLCHAIN=1.92.0
ARG BUCK2_VERSION=2025-06-01

# =============================================================================
# Stage 1: Builder Base - Prepare workspace
# =============================================================================
FROM rust:${RUST_VERSION} AS builder-base

# Build arguments for version tracking
ARG GIT_COMMIT=unknown
ARG BUILD_DATE=unknown

# Platform args (provided by buildx/BuildKit). Used to scope cache keys.
ARG TARGETARCH
ARG TARGETOS
ARG TARGETPLATFORM

WORKDIR /build

RUN mkdir -p /build/bin

# Install build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    pkg-config \
    cmake \
    clang \
    llvm-dev \
    libclang-dev \
    libssl-dev \
    libfuse3-dev \
    protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

# Copy entire project
COPY . .

# =============================================================================
# Stage 1a: Builder (BuildKit cache enabled) - Compile Rust binaries
# =============================================================================
FROM builder-base AS builder

RUN --mount=type=cache,id=mega-cargo-registry-${TARGETARCH},target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=mega-cargo-git-${TARGETARCH},target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,id=mega-cargo-target-${TARGETARCH},target=/build/target,sharing=locked \
    CARGO_TARGET_DIR=/build/target cargo build --release --package orion --package scorpio \
    && install -m 0755 /build/target/release/orion /build/bin/orion \
    && install -m 0755 /build/target/release/scorpio /build/bin/scorpio

# =============================================================================
# Stage 1b: Builder (no BuildKit cache) - Compile Rust binaries
# =============================================================================
FROM builder-base AS builder-nocache

RUN CARGO_TARGET_DIR=/build/target cargo build --release --package orion --package scorpio \
    && install -m 0755 /build/target/release/orion /build/bin/orion \
    && install -m 0755 /build/target/release/scorpio /build/bin/scorpio

# =============================================================================
# Stage 2: Buck2 Downloader - Fetch Buck2 binary
# =============================================================================
FROM debian:bookworm-slim AS buck2-downloader

ARG BUCK2_VERSION
ARG TARGETARCH

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    zstd \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /buck2

# Download Buck2 based on architecture
# Using official GitHub releases from facebook/buck2
RUN set -eux; \
    case "${TARGETARCH}" in \
        amd64) BUCK2_ARCH="x86_64-unknown-linux-musl" ;; \
        arm64) BUCK2_ARCH="aarch64-unknown-linux-musl" ;; \
        *) echo "Unsupported architecture: ${TARGETARCH}" && exit 1 ;; \
    esac; \
    BUCK2_URL="https://github.com/facebook/buck2/releases/download/${BUCK2_VERSION}/buck2-${BUCK2_ARCH}.zst"; \
    echo "Downloading Buck2 from: ${BUCK2_URL}"; \
    curl -fsSL -o buck2.zst "${BUCK2_URL}"; \
    zstd -d buck2.zst -o buck2; \
    chmod +x buck2; \
    ./buck2 --version

# =============================================================================
# Stage 3: Runtime Base - Minimal production image
# =============================================================================
FROM debian:bookworm-slim AS runtime-base

# Build arguments for labels
ARG GIT_COMMIT=unknown
ARG BUILD_DATE=unknown
ARG BUCK2_VERSION

# Image metadata
LABEL org.opencontainers.image.title="Mega Dev Image" \
      org.opencontainers.image.description="Unified development image with Orion Worker, Scorpio, and Buck2" \
      org.opencontainers.image.version="1.0.0" \
      org.opencontainers.image.revision="${GIT_COMMIT}" \
      org.opencontainers.image.created="${BUILD_DATE}" \
      org.opencontainers.image.source="https://github.com/web3infra-foundation/mega" \
      mega.orion.version="${GIT_COMMIT}" \
      mega.orion_worker.version="${GIT_COMMIT}" \
      mega.scorpio.version="${GIT_COMMIT}" \
      mega.buck2.version="${BUCK2_VERSION}"

# Install runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    fuse3 \
    libfuse3-3 \
    libssl3 \
    git \
    git-lfs \
    curl \
    gettext-base \
    netcat-openbsd \
    procps \
    && rm -rf /var/lib/apt/lists/* \
    # Configure FUSE to allow other users
    && echo "user_allow_other" >> /etc/fuse.conf

# Create necessary directories
RUN mkdir -p \
    /app/bin \
    /app/config \
    /data/scorpio/store \
    /data/scorpio/antares/upper \
    /data/scorpio/antares/cl \
    /data/scorpio/antares/mnt \
    /workspace/mount

# Copy Buck2 binary
COPY --from=buck2-downloader /buck2/buck2 /usr/local/bin/

# Copy configuration templates and scripts
COPY orion/scorpio.toml.template /app/config/
COPY orion/entrypoint.sh /app/

# Write a small metadata file that the entrypoint can print for debugging.
RUN printf "GIT_COMMIT=%s\nBUILD_DATE=%s\nBUCK2_VERSION=%s\n" "${GIT_COMMIT}" "${BUILD_DATE}" "${BUCK2_VERSION}" > /etc/mega-image-info

# Make entrypoint executable
RUN chmod +x /app/entrypoint.sh

# Set environment variables
ENV PATH="/app/bin:/usr/local/bin:${PATH}" \
    RUST_LOG=info \
    # Scorpio defaults
    SCORPIO_STORE_PATH=/data/scorpio/store \
    SCORPIO_WORKSPACE=/workspace/mount \
    # Buck2 defaults
    BUCK2_ISOLATION_DIR=/tmp/buck2-isolation

WORKDIR /workspace

ENTRYPOINT ["/app/entrypoint.sh"]
CMD ["help"]

# =============================================================================
# Stage 3a: Runtime - Copy binaries built with cache
# =============================================================================
FROM runtime-base AS runtime

COPY --from=builder /build/bin/orion /app/bin/
COPY --from=builder /build/bin/scorpio /app/bin/

# =============================================================================
# Stage 3b: Runtime (no BuildKit cache) - Copy binaries built without cache
# =============================================================================
FROM runtime-base AS runtime-nocache

COPY --from=builder-nocache /build/bin/orion /app/bin/
COPY --from=builder-nocache /build/bin/scorpio /app/bin/

# =============================================================================
# Stage 4: Dev - Full development image with Rust toolchain
# =============================================================================
FROM runtime AS dev

# Keep dev toolchain pinned (must be re-declared after FROM).
ARG RUST_TOOLCHAIN

# Install development tools
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    pkg-config \
    cmake \
    clang \
    llvm-dev \
    libclang-dev \
    libssl-dev \
    libfuse3-dev \
    vim \
    less \
    jq \
    htop \
    && rm -rf /var/lib/apt/lists/*

# Install Rust toolchain
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | \
    sh -s -- -y --default-toolchain ${RUST_TOOLCHAIN} \
    && . $HOME/.cargo/env \
    && rustup component add rustfmt clippy rust-analyzer

# Set Rust environment
ENV PATH="/root/.cargo/bin:${PATH}" \
    CARGO_HOME="/root/.cargo" \
    RUSTUP_HOME="/root/.rustup"

# Install additional Cargo tools
RUN cargo install cargo-watch cargo-expand

LABEL mega.image.type="dev"

# =============================================================================
# Stage 4b: Dev (no BuildKit cache) - Full development image with Rust toolchain
# =============================================================================
FROM runtime-nocache AS dev-nocache

# Keep dev toolchain pinned (must be re-declared after FROM).
ARG RUST_TOOLCHAIN

# Install development tools
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    pkg-config \
    cmake \
    clang \
    llvm-dev \
    libclang-dev \
    libssl-dev \
    libfuse3-dev \
    vim \
    less \
    jq \
    htop \
    && rm -rf /var/lib/apt/lists/*

# Install Rust toolchain
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | \
    sh -s -- -y --default-toolchain ${RUST_TOOLCHAIN} \
    && . $HOME/.cargo/env \
    && rustup component add rustfmt clippy rust-analyzer

# Set Rust environment
ENV PATH="/root/.cargo/bin:${PATH}" \
    CARGO_HOME="/root/.cargo" \
    RUSTUP_HOME="/root/.rustup"

# Install additional Cargo tools
RUN cargo install cargo-watch cargo-expand

LABEL mega.image.type="dev"
