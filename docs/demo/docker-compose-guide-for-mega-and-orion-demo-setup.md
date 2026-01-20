# Mega / Orion Demo Environment – Docker Compose Guide

> ⚠️ **Important Notice**: This Docker Compose setup is **for local demo/development only**. Do **NOT** run it in production. Default passwords and test users are included.

## Table of Contents

- [Prerequisites](#prerequisites)
- [Quick Start](#quick-start)
- [Demo Walk-Through](#demo-walk-through)
- [Service Endpoints](#service-endpoints)
- [FAQ](#faq)
- [Stopping & Cleanup](#stopping--cleanup)
- [Log Streaming](#log-streaming)
- [Architecture Overview](#architecture-overview)

---

## Prerequisites

### Supported OS

* Linux – native Docker & Compose
* macOS – Docker Desktop
* Windows – Docker Desktop (WSL2 recommended)

### Required Tools

* Docker ≥ 20.10
* Docker Compose ≥ 2 (bundled with Docker Desktop)

```bash
docker --version
docker compose version
```

---

## Quick Start

### 1  Clone the repo

```bash
git clone https://github.com/web3infra-foundation/mega.git
cd mega
```

### 2  Configure environment *(optional)*

The demo environment already has sensible default values and can be used as-is. To customize any settings, create a `.env` file under `docker/demo/`:

```bash
cd docker/demo
# (Optional) copy `.env.example` to `.env` and edit as needed
```

The main configurable environment variables include:

- **Database Configuration**:
  - `POSTGRES_USER`: PostgreSQL username (default: `postgres`)
  - `POSTGRES_PASSWORD`: PostgreSQL password (default: `postgres`)
  - `POSTGRES_DB_MONO`: PostgreSQL database name (default: `mono`, shared by Mega + Orion Server)
  - `MYSQL_ROOT_PASSWORD`: MySQL root password (default: `mysqladmin`)
  - `MYSQL_DATABASE`: Campsite database name (default: `campsite`, uses MySQL)

- **Service Images**：
  - `MEGA_ENGINE_IMAGE`：Mega backend image (default: `public.ecr.aws/m8q5m4u3/mega:mono-0.1.0-pre-release`)
  - `MEGA_UI_IMAGE`：Mega UI image (default: `public.ecr.aws/m8q5m4u3/mega:mega-ui-staging-0.1.0-pre-release`)
  - `MEGA_DEV_IMAGE`：Orion Build Client image（default: `public.ecr.aws/m8q5m4u3/mega:mega-dev-0.1.0-pre-release`）
  - `CAMPSITE_API_IMAGE`：Campsite API image (default uses locally built `campsite-api:latest`, source repo: `https://github.com/web3infra-foundation/campsite`; if development environment encryption configuration (Master Key, etc.) is ready, you can directly set it to public.ecr.aws/m8q5m4u3/mega:campsite-0.1.0-pre-release to pull the pre-built image)
  - Before building the image, you need to generate a Rails `master key` and fill it into the `CAMPSITE_RAILS_MASTER_KEY` field in the `.env` file:
    1. Navigate to the Campsite source code directory.
    2. Execute `VISUAL="code -n --wait" ./bin/rails credentials:edit --environment development`.
    3. After saving, copy the generated `master key` from the prompt in the terminal.
    4. Write this key to `CAMPSITE_RAILS_MASTER_KEY=` in `docker/demo/.env` (copied from `.env.example`).
  - `CAMPSITE_RUN_MIGRATIONS`: Whether to run database migrations when the container starts; `1` (default) to run, can be changed to `0` after the first successful migration to skip and speed up subsequent starts.

- **RustFS Configuration**:
  - `RUSTFS_ACCESS_KEY`: RustFS access key (default: `rustfsadmin`)
  - `RUSTFS_SECRET_KEY`: RustFS secret key (default: `rustfsadmin`)

> **Note**: The demo environment uses default passwords and test users for demonstration purposes only.

### 3. Start all services

Execute in the project root directory:

```bash
docker compose -f docker/demo/docker-compose.demo.yml up -d
```

This command will:

1. Pull the required Docker images (may take a long time for the first run)
2. Create Docker networks and volumes
3. Start all services in dependency order:
   - First, start infrastructure services (PostgreSQL, MySQL, Redis, RustFS)
   - Then, start application services (Mega, Orion Server, Campsite API)
   - Finally, start client services (Mega UI, Orion Build Client)

### 4. Check service status

View the status of all services:

```bash
docker compose -f docker/demo/docker-compose.demo.yml ps
```

View service logs (follow):

```bash
docker compose -f docker/demo/docker-compose.demo.yml logs -f
```

View logs for specific services:

```bash
docker compose -f docker/demo/docker-compose.demo.yml logs -f mega
docker compose -f docker/demo/docker-compose.demo.yml logs -f orion_server
```

### 5. Wait for services to become ready

On the first startup, services may take some time to finish:

- **Database initialization**: PostgreSQL and MySQL need to initialize the databases
- **Service health checks**: Each service waits for its dependencies to become healthy before starting
- **Image build**: If using locally built images, the `mega` and `orion_server` services need to be built from source (slower on the first run)
- **PostgreSQL init script**: On the very first launch the container runs `docker/demo/init-db.sh` automatically (mounted into `/docker-entrypoint-initdb.d/`).  The script does not create extra schemas; it simply prints helpful hints and reminds you that the `mono` database is auto-created by the `POSTGRES_DB` variable.  Because the PostgreSQL data directory is persisted in the `postgres-data` volume, this script is executed only **once** unless you delete the volume.

Typically you should wait **2–5 minutes**. You can monitor service health with the following command:

```bash
# View the health status of all services
docker compose -f docker/demo/docker-compose.demo.yml ps
```

When all services show a status of `healthy` or `running`, you can start using the demo.

---

## Demo walk-through

### 1. Open Mega UI

Open your browser and visit:

```
http://localhost:3000
```

### 2. Sign in with the test user

The demo environment includes a built-in test user you can use directly:

- **Username**: `mega` (or as configured by `MEGA_AUTHENTICATION__TEST_USER_NAME`)
- **Token**: `mega` (or as configured by `MEGA_AUTHENTICATION__TEST_USER_TOKEN`)

### 3. Trigger an Orion build

In Mega UI:

1. Create a new monorepo project or select an existing one
2. On the project page, find the build-related features
3. Trigger a Buck2 build task
4. The build request will be sent to Orion Server and executed by Orion Build Client

### 4. View build results

- **View in the UI**: Build status and logs are displayed in Mega UI
- **View build client logs**:
  ```bash
  docker compose -f docker/demo/docker-compose.demo.yml logs -f orion_build_client
  ```
- **View Orion Server logs**:
  ```bash
  docker compose -f docker/demo/docker-compose.demo.yml logs -f orion_server
  ```

### 5. Access the RustFS console (optional)

RustFS object storage provides a web console:

```
http://localhost:9001/rustfs/console/access-keys
```

Log in with the following credentials:
- **Access Key**: `rustfsadmin` (or the value of `RUSTFS_ACCESS_KEY`)
- **Secret Key**: `rustfsadmin` (or the value of `RUSTFS_SECRET_KEY`)

---

## Service Endpoints

| Service | URL | Description |
|------|---------|------|
| **Mega UI** | http://localhost:3000 | Web Frontend UI |
| **Mega API** | http://localhost:8000 | Mega backend API |
| **Orion Server** | http://localhost:8004 | Orion build server API |
| **Campsite API** | http://localhost:8080 | Campsite OAuth/SSO API |
| **PostgreSQL** | localhost:5432 | Database (used by Mega & Orion, mapped to host port 5432 in demo) |
| **MySQL** | localhost:3306 | Database (used by Campsite API, mapped to host port 3306 in demo) |
| **Redis** | localhost:6379 | Cache service (mapped to host port 6379 in demo) |
| **RustFS** | http://localhost:9001 | Object storage service & console |

### API Health Check Endpoints

- **Mega API**：`GET http://localhost:8000/api/v1/status`
- **Orion Server**：`GET http://localhost:8004/v2/health`
- **Campsite API**：`GET http://localhost:8080/health`

---

## Frequently Asked Questions (FAQ)

### Port Conflict

**Issue**: Docker reports the port is already allocated

**Solution**:

1. **Update the port mapping in the compose file**:
   Edit `docker/demo/docker-compose.demo.yml` and adjust the `ports` section, e.g.:
   ```yaml
   ports:
     - "8001:8000"  # Change host port to 8001
   ```

2. **Stop the service occupying the port**:
   ```bash
   # Find the process occupying the port (Linux/macOS)
   lsof -i :8000
   # or use netstat (Windows)
   netstat -ano | findstr :8000
   ```

### Slow First-Time Start

**Issue**: First run of `docker compose up` takes a long time

**Reason**:
- Images must be pulled from remote registries (may be large)
- If you are using locally built images, the `mega` and `orion_server` services need to be built from source
- PostgreSQL and MySQL databases need to initialize

**Solution**:
- Be patient; the first startup usually takes **5–15 minutes** (depending on network speed and hardware)
- You can view progress in real time with `docker compose logs -f`
- Subsequent starts will be much faster (images are cached)

### Service Start Failure or Health Check Failure

**Issue**: Some services remain in `unhealthy` or `restarting` state

**Troubleshooting Steps**:

1. **View service logs**:
   ```bash
   docker compose -f docker/demo/docker-compose.demo.yml logs <service_name>
   ```

2. **Check dependency services**:
   Ensure infrastructure services (PostgreSQL, MySQL, Redis, RustFS) are healthy:
   ```bash
   docker compose -f docker/demo/docker-compose.demo.yml ps
   ```

3. **Check environment variables**:
   Verify the `.env` file (if present) has correct settings

4. **Check network connectivity**:
   Ensure container-to-container network communication is normal:
   ```bash
   docker network inspect mega-demo-network
   ```

5. **Restart a service**:
   ```bash
   docker compose -f docker/demo/docker-compose.demo.yml restart <service_name>
   ```

### Orion Build Client cannot connect to Orion Server

**Problem**: The `orion_build_client` container cannot connect to `orion_server`.

**Possible causes**:
- `orion_server` has not fully started yet
- Incorrect WebSocket address configuration
- Network issues

**Solution**:

1. Check whether `orion_server` is healthy:
   ```bash
   docker compose -f docker/demo/docker-compose.demo.yml ps orion_server
   ```

2. Inspect `orion_build_client` logs:
   ```bash
   docker compose -f docker/demo/docker-compose.demo.yml logs orion_build_client
   ```

3. Verify that the environment variable `ORION_WORKER_SERVER_WS` is configured correctly (default: `ws://orion_server:8004/ws`).

### Database connection failure

**Problem**: Mega, Orion, or Campsite cannot connect to the database.

**Troubleshooting steps**:

1. **Check whether PostgreSQL is healthy** (used by Mega and Orion):
   ```bash
   docker compose -f docker/demo/docker-compose.demo.yml ps postgres
   ```

2. **Check whether MySQL is healthy** (used by the Campsite API):
   ```bash
   docker compose -f docker/demo/docker-compose.demo.yml ps mysql
   ```

3. **Check database connection strings**:
   - Mega/Orion: ensure `DATABASE_URL` or `MEGA_DATABASE__DB_URL` is correctly formatted (PostgreSQL)
   - Campsite: ensure `CAMPSITE_DATABASE_URL` is correctly formatted (MySQL; format: `mysql2://user:password@host:port/database`)

4. **Test connectivity manually**:
   ```bash
   # Test PostgreSQL connection (Mega/Orion)
   docker compose -f docker/demo/docker-compose.demo.yml exec postgres psql -U postgres -d mono
   
   # Test MySQL connection (Campsite)
   docker compose -f docker/demo/docker-compose.demo.yml exec mysql mysql -u root -p${MYSQL_ROOT_PASSWORD:-mysqladmin} -e "USE campsite; SELECT 1;"
   ```

### RustFS access failure

**Problem**: Mega or Orion cannot access RustFS object storage.

**Troubleshooting steps**:

1. Check whether RustFS is healthy:
   ```bash
   docker compose -f docker/demo/docker-compose.demo.yml ps rustfs
   ```

2. Check S3 configuration:
   Ensure `MEGA_S3__ENDPOINT_URL` points to `http://rustfs:9000` (in-container address).

3. Check access keys:
   Ensure `S3_ACCESS_KEY_ID` and `S3_SECRET_ACCESS_KEY` match the RustFS configuration.

### Image build failure

**Problem**: `orion_server` failed to build.

**Possible causes**:
- Docker build context issues
- Network issues (unable to download dependencies)
- Insufficient disk space

**Solution**:

1. View detailed build logs:
   ```bash
   docker compose -f docker/demo/docker-compose.demo.yml build --no-cache orion_server
   ```

2. Check disk space:
   ```bash
   df -h  # Linux/macOS
   ```

3. Prune Docker cache:
   ```bash
   docker system prune -a
   ```

---

## Stopping and Cleanup

### Stop services

Stop all services (keep data):

```bash
docker compose -f docker/demo/docker-compose.demo.yml stop
```

Stop and remove containers (keep volumes):

```bash
docker compose -f docker/demo/docker-compose.demo.yml down
```

### Full cleanup (remove all data)

⚠️ **Warning**: The following command will delete all volumes, including database and object storage data. Proceed with caution!

```bash
docker compose -f docker/demo/docker-compose.demo.yml down -v
```

### Clean images (optional)

To remove demo-related Docker images:

```bash
# List images
docker images | grep mega

# Remove specific image
docker rmi <image_id>
```

---

## View logs

### View logs for all services

```bash
docker compose -f docker/demo/docker-compose.demo.yml logs -f
```

### View logs for a specific service

```bash
# Mega backend
docker compose -f docker/demo/docker-compose.demo.yml logs -f mega

# Mega UI
docker compose -f docker/demo/docker-compose.demo.yml logs -f mega_ui

# Orion Server
docker compose -f docker/demo/docker-compose.demo.yml logs -f orion_server

# Orion Build Client
docker compose -f docker/demo/docker-compose.demo.yml logs -f orion_build_client

# Campsite API
docker compose -f docker/demo/docker-compose.demo.yml logs -f campsite_api

# PostgreSQL
docker compose -f docker/demo/docker-compose.demo.yml logs -f postgres

# MySQL
docker compose -f docker/demo/docker-compose.demo.yml logs -f mysql

# Redis
docker compose -f docker/demo/docker-compose.demo.yml logs -f redis

# RustFS
docker compose -f docker/demo/docker-compose.demo.yml logs -f rustfs
```

### View the last N lines of logs

```bash
docker compose -f docker/demo/docker-compose.demo.yml logs --tail=100 <service_name>
```

### View logs for a specific time range

```bash
docker compose -f docker/demo/docker-compose.demo.yml logs --since 10m <service_name>
```

---

## Architecture overview

The demo environment includes the following services:

- **Infrastructure**:
  - `postgres`: PostgreSQL database (used by Mega and Orion Server)
  - `mysql`: MySQL database (used by the Campsite API)
  - `redis`: Redis cache
  - `rustfs`: RustFS object storage (S3-compatible)

- **Application services**:
  - `mega`: Mega backend (Rust)
  - `mega_ui`: Mega Web UI (Next.js)
  - `orion_server`: Orion build server (Rust)
  - `orion_build_client`: Orion build client (based on the mega-dev image)
  - `campsite_api`: Campsite API (Ruby/Rails, built locally by default; if you have the encrypted development credentials configured you can pull the pre-built image directly via `CAMPSITE_API_IMAGE=public.ecr.aws/m8q5m4u3/mega:campsite-0.1.0-pre-release`)

For a detailed architecture diagram and dependency list, see the [Mega / Orion Demo architecture design document](./mega-orion-demo-compose-arch.md).

---

## Getting help

If you run into issues, you can:

1. Read the [FAQ](#常见问题-faq) section of this document.
2. Check the service logs for troubleshooting.

---

## Warning!

⚠️ **This Docker Compose configuration is intended for local demo / evaluation only and is NOT suitable for production.**

The demo environment includes the following insecure settings:
- Default passwords and test users
- HTTPS disabled
- No security policies configured
- Simple data-persistence setup

**Do NOT use this configuration in production!**

