# Local Development Environment Guide

## ⚠️ Security Warning

**This environment is for local development and testing purposes ONLY.**

- **Do NOT deploy this configuration to production.**
- Default passwords and keys are publicly known and insecure.
- Security features like SSL/TLS are disabled.
- Debug mode is enabled for many services.
- `orion_build_client` runs in **privileged mode** (see below).

## Overview

- **Build Script**: `build-dev-images-for-local.sh` - Builds Docker images locally.
- **Docker Compose**: `docker-compose-for-local.yml` - Defines the services stack using local images.
- **Environment**: `.env` - Configuration for the local environment.

## Prerequisites

- **Docker Desktop** (with Buildx enabled)
- **Git**

### Privileged Mode Requirement

The `orion_build_client` service requires **privileged mode** (`privileged: true`) to support nested container builds (Docker-in-Docker via Scorpio). This grants the container root capabilities on the host system. Ensure you trust the code being built and run in this environment.

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

### 3. Configure & Initialize Environment

**Recommended Method: Automated Setup**

Use the initialization script to automatically setup `.env`, generate secure secrets, build images, and start services:

```bash
./scripts/dev/init-dev-env.sh
```

**Alternative: Manual Configuration**

If you prefer to configure manually:

1. Copy the example environment file:
   ```bash
   cp scripts/dev/.env.example scripts/dev/.env
   ```
2. Edit `.env` to customize variables.

The main configurable environment variables include:

- **Database Configuration**:
  - `POSTGRES_USER`: PostgreSQL username (default: `postgres`)
  - `POSTGRES_PASSWORD`: PostgreSQL password (default: `postgres`)
  - `POSTGRES_DB_MONO`: PostgreSQL database name (default: `mono`, shared by Mega + Orion Server)
  - `MYSQL_ROOT_PASSWORD`: MySQL root password (default: `mysqladmin`)
    - ⚠️ For production, create a **dedicated low-privilege user** and update the MySQL health-check accordingly (avoid embedding root password).
  - `MYSQL_DATABASE`: Campsite database name (default: `campsite`, uses MySQL)

- **Service Images**:
  - `MEGA_ENGINE_IMAGE`: Mega backend image (default: `mega-dev:mono-latest` - locally built)
  - `MEGA_UI_IMAGE`: Mega UI image (default: `mega-dev:mega-ui-demo-latest` - locally built)
  - `ORION_SERVER_IMAGE`: Orion Server image (default: `mega-dev:orion-server-latest` - locally built)
  - `ORION_CLIENT_IMAGE`: Orion Build Client image (default: `mega-dev:orion-client-latest` - locally built)
  - `CAMPSITE_API_IMAGE`: Campsite API image (default: `public.ecr.aws/m8q5m4u3/mega:campsite-0.1.0-pre-release`)
  - `CAMPSITE_RUN_MIGRATIONS`: Whether to run database migrations when the container starts; `1` (default) to run, can be changed to `0` after the first successful migration to skip and speed up subsequent starts.

- **RustFS Configuration**:
  - `RUSTFS_ACCESS_KEY`: RustFS access key (default: `rustfsadmin`)
  - `RUSTFS_SECRET_KEY`: RustFS secret key (default: `rustfsadmin`)

**Note:** The `.env` file is configured to use local images by default.

### 4. Start Services (Manual)

If you used the `init-dev-env.sh` script, your services are already running.

If you are setting up manually or restarting services:

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

## Troubleshooting

- **Database Connection Issues**: Ensure PostgreSQL and MySQL containers are healthy (`docker ps`). Check logs: `docker compose logs postgres` or `docker compose logs mysql`.
- **S3/RustFS Issues**: If services fail to connect to object storage, ensure `rustfs` is healthy and the initialization script `init-rustfs-bucket` completed successfully.
- **Build Failures**: Check `orion-worker` logs. Ensure `privileged: true` is enabled in docker-compose.
- **Port Conflicts**: Ensure ports 80, 3000, 8000, 8004, 8080, 5432, 6379, 3306, 9000 are available on your host.
- **Domain Resolution**: If `*.gitmono.local` domains don't work, check your `/etc/hosts` file.
- **Migration Errors**: If Campsite API fails to start, check if migrations failed. You can manually run migrations inside the container:
  ```bash
  docker compose exec campsite_api bin/rails db:migrate
  ```
- **Login Issues**: If you cannot log in, ensure `MEGA_AUTHENTICATION__ENABLE_TEST_USER=true` is set in `.env` for development.

## Advanced Usage

### Volume Lifecycle

The Docker Compose setup uses named volumes to persist data (PostgreSQL, MySQL, Redis, RustFS, etc.).

**To reset the environment completely (delete all data):**

```bash
docker compose -f docker-compose-for-local.yml down -v
```

**Warning**: This will permanently delete all database records, repositories, and logs stored in the named volumes.

### Resource Limits

Services are configured with default resource limits (CPU/Memory) in `docker-compose-for-local.yml`. You can adjust these in the `deploy.resources.limits` section of the compose file if your machine has different constraints.
