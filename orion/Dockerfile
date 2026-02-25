# =============================================================================
# Mega Dev Image - Multi-stage Dockerfile
# =============================================================================
# This Dockerfile builds a unified development image containing:
# - Orion Worker (build execution client)
# - Scorpio (FUSE filesystem service)
# - Buck2 (build tool)
#
# Supports: linux/amd64, linux/arm64
# Final stage: runtime (slim, default)
# NOTE: This Dockerfile assumes the Docker build context is the repo root.
# Example: `docker build -f orion/Dockerfile .`
# =============================================================================

# -----------------------------------------------------------------------------
# Build Arguments
# -----------------------------------------------------------------------------
# Use "bookworm" for latest stable, or specify version like "1.83"
ARG RUST_VERSION=1.92-bookworm
ARG BUCK2_VERSION=2025-06-01

# =============================================================================
# Stage 1: Chef Base - Tooling and system dependencies
# =============================================================================
FROM rust:${RUST_VERSION} AS chef-base

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
    binutils \
    pkg-config \
    cmake \
    clang \
    llvm-dev \
    libclang-dev \
    libssl-dev \
    libfuse3-dev \
    protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

# Install cargo-chef for dependency layer caching
RUN cargo install cargo-chef --version 0.1.73

# =============================================================================
# Stage 1a: Planner - Generate dependency recipe
# =============================================================================
FROM chef-base AS planner

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
COPY orion-server/Cargo.toml orion-server/
COPY orion-server/bellatrix/Cargo.toml orion-server/bellatrix/
COPY saturn/Cargo.toml saturn/
COPY scorpio/Cargo.toml scorpio/
COPY vault/Cargo.toml vault/

RUN cargo chef prepare --recipe-path recipe.json

# =============================================================================
# Stage 1b: Builder - Build cached deps, then compile binaries
# =============================================================================
FROM chef-base AS builder

COPY --from=planner /build/recipe.json /build/recipe.json

RUN --mount=type=cache,id=mega-cargo-registry-${TARGETARCH},target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=mega-cargo-git-${TARGETARCH},target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,id=mega-cargo-target-${TARGETARCH},target=/build/target,sharing=locked \
    CARGO_TARGET_DIR=/build/target \
    CARGO_INCREMENTAL=0 \
    CARGO_PROFILE_RELEASE_DEBUG=0 \
    cargo chef cook --release --recipe-path recipe.json

COPY . .

RUN --mount=type=cache,id=mega-cargo-registry-${TARGETARCH},target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=mega-cargo-git-${TARGETARCH},target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,id=mega-cargo-target-${TARGETARCH},target=/build/target,sharing=locked \
    CARGO_TARGET_DIR=/build/target \
    CARGO_INCREMENTAL=0 \
    CARGO_PROFILE_RELEASE_DEBUG=0 \
    cargo build --release --package orion --package scorpio \
    && strip /build/target/release/orion /build/target/release/scorpio \
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

# Install runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    fuse3 \
    libssl3 \
    gettext-base \
    netcat-openbsd \
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
    SCORPIO_WORKSPACE=/workspace/mount

WORKDIR /workspace

ENTRYPOINT ["/app/entrypoint.sh"]
CMD ["help"]

# =============================================================================
# Stage 3a: Runtime - Copy binaries built with cache
# =============================================================================
FROM runtime-base AS runtime

COPY --from=builder /build/bin/orion /app/bin/
COPY --from=builder /build/bin/scorpio /app/bin/
