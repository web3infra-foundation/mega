# Local Development Environment Guide

This directory contains scripts and configuration for setting up a local development environment for Mega and Orion.

## Overview

- **Build Script**: `build-dev-images-for-local.sh` - Builds Docker images locally.
- **Docker Compose**: `docker-compose-for-local.yml` - Defines the services stack using local images.
- **Environment**: `.env` - Configuration for the local environment.

## Prerequisites

- **Docker Desktop** (with Buildx enabled)
- **Git**

## Getting Started

### 1. Build Local Images

Run the build script to create local Docker images. This script uses your current git short hash to tag images.

```bash
# Build all images
./scripts/dev/build-dev-images-for-local.sh

# Build a specific image
./scripts/dev/build-dev-images-for-local.sh mono-engine
```

**Available Images:**
- `mono-engine` (Tag: `mega-dev:mono-{hash}`)
- `orion-server` (Tag: `mega-dev:orion-server-{hash}`)
- `orion-client` (Tag: `mega-dev:orion-client-{hash}`)
- `mega-ui` (Tag: `mega-dev:mega-ui-demo-{hash}`)

### 2. Configure Virtual Domains

The local environment uses virtual domains under `gitmono.local`. Add the following line to your hosts file (`/etc/hosts` on macOS/Linux or `C:\Windows\System32\drivers\etc\hosts` on Windows) to resolve them to your local machine:

```
127.0.0.1 app.gitmono.local git.gitmono.local api.gitmono.local auth.gitmono.local orion.gitmono.local
```

### 3. Configure Environment

Copy the example environment file and customize it if needed.

```bash
cp scripts/dev/.env.example scripts/dev/.env
```
The main configurable environment variables include:

- **Database Configuration**:
  - `POSTGRES_USER`: PostgreSQL username (default: `postgres`)
  - `POSTGRES_PASSWORD`: PostgreSQL password (default: `postgres`)
  - `POSTGRES_DB_MONO`: PostgreSQL database name (default: `mono`, shared by Mega + Orion Server)
  - `MYSQL_ROOT_PASSWORD`: MySQL root password (default: `mysqladmin`)
    - ⚠️ For production, create a **dedicated low-privilege user** and update the MySQL health-check accordingly (avoid embedding root password).
  - `MYSQL_DATABASE`: Campsite database name (default: `campsite`, uses MySQL)

- **Service Images**:
  - `MEGA_ENGINE_IMAGE`: Mega backend image (default: `public.ecr.aws/m8q5m4u3/mega:mono-0.1.0-pre-release`)
  - `MEGA_UI_IMAGE`: Mega UI image (default: `public.ecr.aws/m8q5m4u3/mega:mega-ui-demo-0.1.0-pre-release`)
  - `ORION_CLIENT_IMAGE`: Orion Build Client image (default: `public.ecr.aws/m8q5m4u3/mega:orion-client-0.1.0-pre-release`)
  - `CAMPSITE_API_IMAGE`: Campsite API image (default: `public.ecr.aws/m8q5m4u3/mega:campsite-0.1.0-pre-release`)
  - `CAMPSITE_RUN_MIGRATIONS`: Whether to run database migrations when the container starts; `1` (default) to run, can be changed to `0` after the first successful migration to skip and speed up subsequent starts.

- **RustFS Configuration**:
  - `RUSTFS_ACCESS_KEY`: RustFS access key (default: `rustfsadmin`)
  - `RUSTFS_SECRET_KEY`: RustFS secret key (default: `rustfsadmin`)

**Note:** The `.env` file is configured to use local images by default.

### 4. Start Services

Use Docker Compose to start the environment.

```bash
cd scripts/dev
docker compose -f docker-compose-for-local.yml up -d
```

### 5. Verify Services

The environment includes an Nginx gateway that routes traffic based on the configured virtual domains.

- **Mega UI**: http://app.gitmono.local
- **Mega Backend (Git)**: http://git.gitmono.local
- **Orion Server**: http://orion.gitmono.local
- **Campsite API**: http://api.gitmono.local
- **Auth Service**: http://auth.gitmono.local

You can still access services directly via localhost ports for debugging:
- **Mega UI**: http://localhost:3000
- **Mega Backend**: http://localhost:8000
- **Orion Server**: http://localhost:8004
- **Campsite API**: http://localhost:8080

## Advanced Usage

### Rebuilding Images

If you make code changes, rebuild the relevant image and restart the service:

```bash
# Rebuild mono-engine
./scripts/dev/build-dev-images-for-local.sh mono-engine

# Restart the service
cd scripts/dev
docker compose -f docker-compose-for-local.yml up -d mega
```

### Clean Up

To stop and remove containers, networks, and volumes:

```bash
cd scripts/dev
docker compose -f docker-compose-for-local.yml down -v
```

## Troubleshooting

- **Image not found**: Ensure you ran the build script successfully. Check `docker images` to see available `mega-dev` images.
- **Unbound variable error**: Ensure you are using the latest version of the build script.
