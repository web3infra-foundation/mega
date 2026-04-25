# ---------- planner stage ----------
# This stage analyzes the Cargo workspace and produces a dependency recipe.
# The recipe will change ONLY when Cargo.toml changes.
FROM rust:1.94.0-bookworm AS chef

ARG TARGETARCH=amd64
ARG CARGO_CHEF_VERSION=0.1.73

WORKDIR /opt/mega

# Prebuilt cargo-chef (avoids compiling it on every cold planner layer).
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && set -eux \
    && case "$TARGETARCH" in \
         amd64) chef_arch=x86_64-unknown-linux-musl ;; \
         arm64) chef_arch=aarch64-unknown-linux-gnu ;; \
         *) echo "unsupported TARGETARCH=$TARGETARCH" >&2; exit 1 ;; \
       esac \
    && curl -fsSL "https://github.com/LukeMathWalker/cargo-chef/releases/download/v${CARGO_CHEF_VERSION}/cargo-chef-${chef_arch}.tar.gz" \
         | tar xz -C /usr/local/cargo/bin

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
COPY vault/Cargo.toml vault/


# Generate a recipe describing the dependency graph
RUN cargo chef prepare --bin mono --recipe-path recipe.json


# ---------- builder stage ----------
# This stage builds dependencies first (cached),
# then builds the actual application binary.
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

# Faster linking (release deps + final mono link).
ENV RUSTFLAGS="-C link-arg=-fuse-ld=mold"

# Copy the dependency recipe from the planner stage
COPY --from=chef /opt/mega/recipe.json recipe.json

# Build and cache all Rust dependencies
# This layer will be reused as long as dependencies do not change
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    cargo chef cook --bin mono --release --recipe-path recipe.json

# Copy only the workspace sources AFTER dependencies are cached.
# This avoids sending the entire repo context (which can be multiple GB) to the builder.
COPY Cargo.toml ./
COPY api-model ./api-model
COPY ceres ./ceres
COPY common ./common
COPY config ./config
COPY context ./context
COPY io-orbit ./io-orbit
COPY jupiter ./jupiter
COPY mono ./mono
COPY orion ./orion
COPY orion-server ./orion-server
COPY saturn ./saturn
COPY vault ./vault

# Build the mono binary (fixed release)
#
# NOTE: Build output must be persisted into the image layer for the runtime stage `COPY`.
# A buildkit cache mount is not committed into the layer, so we build into a cached dir
# then copy the final binary into `/opt/mega/target/...` (regular filesystem).
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/opt/mega/target-cache \
    CARGO_TARGET_DIR=/opt/mega/target-cache cargo build --release -p mono && \
    mkdir -p /opt/mega/target/release && \
    cp /opt/mega/target-cache/release/mono /opt/mega/target/release/mono


# ---------- runtime stage ----------
# This is the minimal runtime image containing only the binary and runtime deps
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    less \
    libssl3 \
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
