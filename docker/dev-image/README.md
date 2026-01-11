# Mega Dev Image (Client Toolchain) - Development and Testing Guide

## Table of Contents

- [Image Overview](#image-overview)
- [Quick Start](#quick-start)
- [Image Build](#image-build)
- [Image Testing](#image-testing)
- [Configuration](#configuration)
- [Service Usage](#service-usage)
- [Development Workflow Examples](#development-workflow-examples)
- [Platform Compatibility](#platform-compatibility)
- [FAQ](#faq)

---

## Image Overview

Mega Dev Image is a unified Linux development image that packages the **client-side** tooling used in the Mega build pipeline:

| Component | Description | Purpose |
|-----------|-------------|---------|
| **Orion Worker (`orion`)** | Build execution client | Connects to an external Orion Server, mounts workspace via Scorpio, runs Buck2 |
| **Scorpio (`scorpio`)** | FUSE filesystem daemon | Virtual filesystem mounting / Build workspace support |
| **Buck2 (`buck2`)** | Build tool | High-performance incremental builds |

This image intentionally does **not** start Orion Server or PostgreSQL. It is designed for:
- Linux developers doing local mounting + Buck2 debugging
- macOS/Windows developers using a remote Linux server as a consistent test environment
- CI and automated checks that only need worker/scorpio/buck2 tooling

### Image Tags

| Tag | Description | Use Case |
|-----|-------------|----------|
| `mega-dev:latest` / `mega-dev:runtime` | Minimal runtime image | CI/CD, remote tooling |
| `mega-dev:dev` | Development image (includes Rust toolchain) | Local development and debugging |

### Supported Architectures

- `linux/amd64` (x86_64)
- `linux/arm64` (aarch64)

---

## Quick Start

### Prerequisites

- Docker 20.10+ or Podman 4.0+
- Linux host (Scorpio needs FUSE; macOS/Windows requires remote Linux server)
- `jq` (recommended for parsing JSON responses)

### 1) Build the Image

```bash
cd /path/to/mega

docker build -t mega-dev:latest \
  --target runtime \
  --build-arg GIT_COMMIT=$(git rev-parse HEAD) \
  --build-arg BUILD_DATE=$(date -u +%Y-%m-%dT%H:%M:%SZ) \
  -f docker/dev-image/Dockerfile .
```

### 2) Start Scorpio (Docker Compose)

```bash
# Start Scorpio daemon (requires FUSE)
docker run -d --name scorpio \
  --privileged \
  -p 2725:2725 \
  -e SCORPIO_BASE_URL=http://host.docker.internal:8000 \
  -e SCORPIO_LFS_URL=http://host.docker.internal:8000 \
  mega-dev:latest scorpio

# Check health
curl -s http://localhost:2725/antares/health
```

### 3) Mount a Repo and Run Buck2 (inside Scorpio container)

Scorpio needs Mono (Mega main service) to fetch repository content. Ensure `SCORPIO_BASE_URL` points to a reachable Mono service.

```bash
export REPO_PATH="your/real/repo/path"

# Create mount (returns mountpoint)
MOUNTPOINT=$(curl -s -X POST http://localhost:2725/antares/mounts \
  -H "Content-Type: application/json" \
  -d "{\"path\":\"${REPO_PATH}\",\"job_id\":\"manual-test\"}" | jq -r '.mountpoint')

echo "Mountpoint: ${MOUNTPOINT}"

# Run Buck2 inside the same container namespace as the mount
docker exec scorpio bash -lc "cd '${MOUNTPOINT}' && buck2 --version && buck2 build //..."
```

### 4) (Optional) Start Orion Worker

Orion Worker connects to an external Orion Server WebSocket endpoint (`SERVER_WS`). If no Orion Server is available, the worker will keep retrying the connection.

```bash
docker run -d --name orion-worker \
  --privileged \
  -e SERVER_WS=ws://your-orion-server:8004/ws \
  -e SCORPIO_BASE_URL=http://host.docker.internal:8000 \
  mega-dev:latest orion

docker logs -f orion-worker
```

---

## Image Build

### Build Arguments

| Argument | Default | Description |
|----------|---------|-------------|
| `RUST_VERSION` | `1.92-bookworm` | Rust builder base image tag |
| `RUST_TOOLCHAIN` | `1.92.0` | dev image rustup toolchain |
| `BUCK2_VERSION` | `2025-06-01` | Buck2 release version (GitHub release tag) |
| `GIT_COMMIT` | `unknown` | Git commit hash (written to image LABEL) |
| `BUILD_DATE` | `unknown` | Build date (ISO 8601 format) |

### Multi-Architecture Build

```bash
docker buildx create --name mega-builder --use

docker buildx build \
  --platform linux/amd64,linux/arm64 \
  --target runtime \
  --build-arg GIT_COMMIT=$(git rev-parse HEAD) \
  --build-arg BUILD_DATE=$(date -u +%Y-%m-%dT%H:%M:%SZ) \
  -t mega-dev:latest \
  -f docker/dev-image/Dockerfile \
  --push .
```

---

## Image Testing

### 1) Basic Validation (No FUSE Required)

```bash
docker run --rm mega-dev:latest version
docker run --rm mega-dev:latest buck2 --version
docker run --rm mega-dev:latest bash -lc "command -v scorpio && command -v orion"
```

### 2) Scorpio Validation (Requires FUSE)

```bash
docker run -d --name scorpio \
  --privileged \
  -p 2725:2725 \
  -e SCORPIO_BASE_URL=http://host.docker.internal:8000 \
  -e SCORPIO_LFS_URL=http://host.docker.internal:8000 \
  mega-dev:latest scorpio

curl -s http://localhost:2725/antares/health
```

### 3) End-to-End Validation: Scorpio + Mono + Buck2 (Requires FUSE + Mono)

```bash
export REPO_PATH="your/real/repo/path"

MOUNTPOINT=$(curl -s -X POST http://localhost:2725/antares/mounts \
  -H "Content-Type: application/json" \
  -d "{\"path\":\"${REPO_PATH}\",\"job_id\":\"smoke\"}" | jq -r '.mountpoint')

docker exec scorpio bash -lc "cd '${MOUNTPOINT}' && buck2 build //..."
```

---

## Configuration

### Scorpio Configuration

Scorpio uses a `scorpio.toml` configuration file; the built-in template supports environment variable substitution.

| Environment Variable | Default | Description |
|---------------------|---------|-------------|
| `SCORPIO_BASE_URL` | `http://localhost:8000` | Mono service address (required for mounts) |
| `SCORPIO_LFS_URL` | `http://localhost:8000` | LFS service address |
| `SCORPIO_STORE_PATH` | `/data/scorpio/store` | Data storage path |
| `SCORPIO_WORKSPACE` | `/workspace/mount` | Workspace mount root |
| `SCORPIO_GIT_AUTHOR` | `MEGA` | Git author name |
| `SCORPIO_GIT_EMAIL` | `admin@mega.org` | Git email |

Custom configuration file:

```bash
docker run -d --name scorpio \
  --privileged \
  -v /path/to/scorpio.toml:/app/config/scorpio.toml:ro \
  -p 2725:2725 \
  mega-dev:latest scorpio -c /app/config/scorpio.toml
```

### Orion Worker Configuration

| Environment Variable | Default | Description |
|---------------------|---------|-------------|
| `SERVER_WS` | `ws://127.0.0.1:8004/ws` | Orion Server WebSocket URL (external) |
| `ORION_WORKER_ID` | - | Optional worker id (generated if not set) |
| `BUCK_PROJECT_ROOT` | `/workspace` | Root dir that contains the Scorpio mount dir `mount/` |
| `BUILD_TMP` | `/tmp/orion-builds` | Worker temp/build directory |
| `SCORPIO_API_BASE_URL` | `http://localhost:2725` | Scorpio API base URL used by worker |
| `ORION_WORKER_START_SCORPIO` | `true` | Whether to start embedded Scorpio inside worker container |

---

## Service Usage

### Scorpio

#### Dependency Notes

- Scorpio requires a running Mono service to fetch repository content (configure `SCORPIO_BASE_URL`).
- Scorpio requires FUSE support and container privileges (see FAQ).

#### Start Service

```bash
docker run -d --name scorpio \
  --privileged \
  -e SCORPIO_BASE_URL=http://host.docker.internal:8000 \
  -p 2725:2725 \
  mega-dev:latest scorpio
```

#### Mount a Repo (Antares API)

```bash
curl -s -X POST http://localhost:2725/antares/mounts \
  -H "Content-Type: application/json" \
  -d '{"path":"your/real/repo/path","job_id":"manual-test"}' | jq
```

### Buck2

```bash
docker run --rm mega-dev:latest buck2 --version
```

When building from a Scorpio mount, run Buck2 inside the Scorpio container:

```bash
docker exec scorpio bash -lc "cd /workspace/mount && buck2 build //..."
```

### Orion Worker

Orion Worker connects to an external Orion Server. It does not expose an HTTP API.

```bash
docker run -d --name orion-worker \
  --privileged \
  -e SERVER_WS=ws://your-orion-server:8004/ws \
  -e SCORPIO_BASE_URL=http://host.docker.internal:8000 \
  mega-dev:latest orion
```

---

## Development Workflow Examples

### Example 1: Scorpio Mount + Buck2 Build

```bash
docker run -d --name scorpio \
  --privileged \
  -p 2725:2725 \
  -e SCORPIO_BASE_URL=http://host.docker.internal:8000 \
  -e SCORPIO_LFS_URL=http://host.docker.internal:8000 \
  mega-dev:latest scorpio

export REPO_PATH="your/real/repo/path"
MOUNTPOINT=$(curl -s -X POST http://localhost:2725/antares/mounts \
  -H "Content-Type: application/json" \
  -d "{\"path\":\"${REPO_PATH}\",\"job_id\":\"example\"}" | jq -r '.mountpoint')

docker exec scorpio bash -lc "cd '${MOUNTPOINT}' && buck2 build //..."
```

### Example 2: Buck2 Debugging on a Local Workspace (No Scorpio)

```bash
docker run -it --rm \
  -v $(pwd):/workspace \
  -w /workspace \
  mega-dev:latest bash

# inside container
buck2 --version
```

### Example 3: Run Orion Worker Against a Remote Orion Server

```bash
docker run -d --name orion-worker \
  --privileged \
  -e SERVER_WS=ws://your-orion-server:8004/ws \
  -e SCORPIO_BASE_URL=http://host.docker.internal:8000 \
  mega-dev:latest orion

docker logs -f orion-worker
```

---

## Platform Compatibility

### Linux (Full Support)

- Scorpio FUSE mount
- Buck2 build
- Orion Worker (connect to external server)

### macOS / Windows (Partial Support)

Due to FUSE and kernel differences, Scorpio cannot run locally on macOS/Windows.

Recommended approach:
- Run this image on a remote Linux server
- Use SSH port forwarding to access Scorpio API

```bash
ssh -L 2725:localhost:2725 user@remote-server
```

---

## FAQ

### Q1: Scorpio startup fails with "/dev/fuse not found"

Cause: container does not have access to the FUSE device.

```bash
# Recommended
docker run --privileged ...

# Minimal privilege mode
docker run \
  --device /dev/fuse \
  --cap-add SYS_ADMIN \
  --security-opt apparmor:unconfined \
  ...
```

### Q2: Scorpio mount fails / cannot fetch repository content

Cause: Scorpio requires Mono service to fetch tree/blob/commit data, but `SCORPIO_BASE_URL` is not reachable or incorrect.

Action:
- Ensure Mono is running and reachable from the container
- Set `SCORPIO_BASE_URL` / `SCORPIO_LFS_URL` accordingly (see `.env.example`)

### Q3: Buck2 build fails with missing `//third-party:*`

Cause: BUCK files depend on `//third-party:*`, but the `third-party` directory may need to be generated via Reindeer.

```bash
cargo install --locked --git https://github.com/facebookincubator/reindeer reindeer
reindeer buckify
```

### Q4: Mountpoint not visible in another container

Cause: FUSE mountpoints are tied to a container's mount namespace.

Action:
- Run `buck2` via `docker exec` inside the `scorpio` container, or
- Run Scorpio embedded in the same container as the process that consumes the mount.

---

## File Structure

```
docker/dev-image/
├── Dockerfile              # Multi-stage build file
├── docker-compose.yml      # Optional orchestration (scorpio/worker/dev)
├── entrypoint.sh           # Container entrypoint script
├── scorpio.toml.template   # Scorpio configuration template
├── .env.example            # Environment variable example
└── README.md               # This document
```
