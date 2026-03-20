# ---------- planner stage ----------
# This stage analyzes the Cargo workspace and produces a dependency recipe.
# The recipe will change ONLY when Cargo.toml changes.
FROM rust:1.94.0-bookworm AS chef

WORKDIR /opt/mega

# Install cargo-chef, a tool to cache Rust dependencies efficiently
RUN cargo install cargo-chef --version 0.1.73

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
    protobuf-compiler \
    libprotobuf-dev \
    && rm -rf /var/lib/apt/lists/*


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
RUN cargo build --release -p mono


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
