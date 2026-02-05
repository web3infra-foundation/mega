#!/bin/bash
set -e

# ============================================================================
# Mega / Orion Local Development Environment Setup Script
# ============================================================================
# This script initializes the local environment by:
# 1. Creating .env from .env.example
# 2. Generating secure random passwords and keys
# 3. Starting the build process for local Docker images
# 4. Starting the services via Docker Compose
# ============================================================================

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo -e "${GREEN}[INFO]${NC} Initializing Local Development Environment..."

# 1. Setup .env
if [ -f .env ]; then
    echo -e "${YELLOW}[WARN]${NC} .env file already exists."
    read -p "Do you want to overwrite it? (y/N) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo -e "${GREEN}[INFO]${NC} Skipping .env generation."
    else
        cp .env.example .env
        echo -e "${GREEN}[INFO]${NC} Reset .env from .env.example."
    fi
else
    cp .env.example .env
    echo -e "${GREEN}[INFO]${NC} Created .env from .env.example."
fi

# 2. Generate Secrets (only if we just created/overwrote .env or force update)
# We'll check if the values are still the defaults to decide whether to update
if grep -q "POSTGRES_PASSWORD=postgres" .env; then
    echo -e "${GREEN}[INFO]${NC} Generating secure secrets..."

    # Generate random values
    PG_PASS=$(openssl rand -hex 12)
    MYSQL_PASS=$(openssl rand -hex 12)
    ACCESS_KEY=$(openssl rand -hex 8)
    SECRET_KEY=$(openssl rand -hex 16)
    # Rails Master Key (32 bytes hex)
    RAILS_KEY=$(openssl rand -hex 16)

    # Use sed to replace values (portable way for macOS/Linux)
    # Note: Using | as delimiter to avoid issues with special chars
    sed -i.bak "s|POSTGRES_PASSWORD=postgres|POSTGRES_PASSWORD=$PG_PASS|" .env
    sed -i.bak "s|MYSQL_ROOT_PASSWORD=mysqladmin|MYSQL_ROOT_PASSWORD=$MYSQL_PASS|" .env
    sed -i.bak "s|RUSTFS_ACCESS_KEY=rustfsadmin|RUSTFS_ACCESS_KEY=$ACCESS_KEY|" .env
    sed -i.bak "s|RUSTFS_SECRET_KEY=rustfsadmin|RUSTFS_SECRET_KEY=$SECRET_KEY|" .env
    sed -i.bak "s|S3_ACCESS_KEY_ID=rustfsadmin|S3_ACCESS_KEY_ID=$ACCESS_KEY|" .env
    sed -i.bak "s|S3_SECRET_ACCESS_KEY=rustfsadmin|S3_SECRET_ACCESS_KEY=$SECRET_KEY|" .env
    sed -i.bak "s|CAMPSITE_RAILS_MASTER_KEY=change_me_in_production|CAMPSITE_RAILS_MASTER_KEY=$RAILS_KEY|" .env

    rm .env.bak
    echo -e "${GREEN}[INFO]${NC} Secrets updated in .env"
fi

# 3. Start Build
echo -e "${GREEN}[INFO]${NC} Starting Docker image build..."
./build-dev-images-for-local.sh

# 4. Start Services
echo -e "${GREEN}[INFO]${NC} Starting services with Docker Compose..."
docker compose -f docker-compose-for-local.yml up -d

echo -e "${GREEN}[SUCCESS]${NC} Environment setup complete!"
echo -e "Services should be running at:"
echo -e "  - Mega UI: http://app.gitmono.local (or http://localhost:3000)"
echo -e "  - Mega Backend: http://git.gitmono.local (or http://localhost:8000)"
echo -e "  - Orion Server: http://orion.gitmono.local (or http://localhost:8004)"
echo -e "  - Campsite API: http://api.gitmono.local (or http://localhost:8080)"
