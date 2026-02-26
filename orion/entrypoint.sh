#!/bin/bash
# =============================================================================
# Mega Dev Image - Entrypoint Script
# =============================================================================
# This script handles container startup for different services:
# - orion: Start the Orion worker (build execution client)
# - buck2: Run Buck2 commands
# - bash/sh: Interactive shell
# - help: Show usage information
#
# Note: Scorpio (FUSE filesystem) has been moved to:
# https://github.com/web3infra-foundation/scorpiofs
# =============================================================================

set -e

# -----------------------------------------------------------------------------
# Configuration
# -----------------------------------------------------------------------------
CONFIG_DIR="/app/config"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# -----------------------------------------------------------------------------
# Helper Functions
# -----------------------------------------------------------------------------

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

log_header() {
    echo -e "${BLUE}============================================================${NC}"
    echo -e "${BLUE} $1${NC}"
    echo -e "${BLUE}============================================================${NC}"
}

show_help() {
    cat << 'EOF'
Mega Dev Image - Unified Development Environment
=================================================

Usage: docker run [OPTIONS] mega-dev:<tag> <COMMAND> [ARGS...]

Commands:
  orion           Start the Orion worker (build execution client)
  orion-worker    Alias for `orion`
  buck2 [ARGS]    Run Buck2 with specified arguments
  bash, sh        Start an interactive shell
  help            Show this help message

Examples:
  # Start Orion Worker (build execution client)
  docker run -d --name orion-worker \
    -e SERVER_WS=ws://your-orion-server:8004/ws \
    mega-dev:latest orion

  # Run Buck2 build
  docker run --rm -v $(pwd):/workspace mega-dev:latest buck2 build //...

  # Interactive shell
  docker run --rm -it mega-dev:latest bash

Environment Variables:
  SERVER_WS             - WebSocket URL for Orion server
  BUCK_PROJECT_ROOT     - Buck2 project root (default: /workspace)
  BUILD_TMP             - Temporary build directory
  RUST_LOG              - Logging level (default: info)
  MEGA_BASE_URL         - Mega server base URL (default: http://localhost:8000)
  MEGA_LFS_URL          - Mega LFS URL (default: same as MEGA_BASE_URL)
  SCORPIO_CONFIG        - Custom scorpio.toml path (optional, auto-generated if not set)

Note: Scorpio (FUSE filesystem) has been moved to a separate repository:
      https://github.com/web3infra-foundation/scorpiofs

For more information, see the documentation in orion/
EOF
}



# Wait for a TCP service to be available
wait_for_service() {
    local host=$1
    local port=$2
    local timeout=${3:-30}
    local elapsed=0

    log_info "Waiting for $host:$port (timeout: ${timeout}s)..."

    while ! nc -z "$host" "$port" 2>/dev/null; do
        sleep 1
        elapsed=$((elapsed + 1))
        if [ $elapsed -ge $timeout ]; then
            log_error "Timeout waiting for $host:$port"
            return 1
        fi
    done

    log_info "$host:$port is available"
    return 0
}

setup_directories() {
    log_info "Setting up directories..."
    mkdir -p /workspace
    
    # Setup scorpio directories for Antares overlay filesystem
    mkdir -p /var/lib/scorpio/{store,antares/{upper,cl,mnt}}
    mkdir -p /etc/scorpio
}

# Generate scorpio.toml from template
setup_scorpio_config() {
    local template="/app/config/scorpio.toml.template"
    local config="/etc/scorpio/scorpio.toml"
    
    if [ -f "$template" ]; then
        log_info "Generating scorpio configuration from template..."
        envsubst < "$template" > "$config"
        export SCORPIO_CONFIG="$config"
        log_info "Scorpio config written to: $config"
    elif [ -n "${SCORPIO_CONFIG:-}" ] && [ -f "${SCORPIO_CONFIG}" ]; then
        log_info "Using existing scorpio config: ${SCORPIO_CONFIG}"
    else
        log_warn "No scorpio config template or SCORPIO_CONFIG found"
    fi
}

# Print version information
print_versions() {
    log_header "Mega Dev Image - Version Information"
    echo ""
    echo "Components:"
    if command -v orion &>/dev/null; then
        echo "  Orion Worker: installed"
    else
        echo "  Orion Worker: missing"
    fi
    echo "  Buck2:        $(buck2 --version 2>/dev/null || echo 'unknown')"
    echo ""
    echo "Image Labels:"
    if [ -f /etc/mega-image-info ]; then
        cat /etc/mega-image-info
    fi
    echo ""
    echo "Note: Scorpio (FUSE) moved to https://github.com/web3infra-foundation/scorpiofs"
    echo ""
}

# -----------------------------------------------------------------------------
# Service Start Functions
# -----------------------------------------------------------------------------

start_orion_worker() {
    log_header "Starting Orion Worker"

    print_versions

    # Orion worker configuration
    : "${SERVER_WS:=ws://127.0.0.1:8004/ws}"
    : "${BUCK_PROJECT_ROOT:=/workspace}"
    : "${BUILD_TMP:=/tmp/orion-builds}"

    # Scorpiofs config defaults
    : "${MEGA_BASE_URL:=http://git.gitmega.com}"
    : "${MEGA_LFS_URL:=${MEGA_BASE_URL}}"

    setup_directories
    setup_scorpio_config
    mkdir -p "${BUILD_TMP}"

    log_info "Configuration:"
    echo "  SERVER_WS: ${SERVER_WS}"
    echo "  ORION_WORKER_ID: ${ORION_WORKER_ID:-(auto)}"
    echo "  BUCK_PROJECT_ROOT: ${BUCK_PROJECT_ROOT}"
    echo "  BUILD_TMP: ${BUILD_TMP}"
    echo "  MEGA_BASE_URL: ${MEGA_BASE_URL}"
    echo "  MEGA_LFS_URL: ${MEGA_LFS_URL}"
    echo "  SCORPIO_CONFIG: ${SCORPIO_CONFIG:-not set}"
    echo ""

    log_info "Starting orion worker..."
    exec orion "$@"
}

run_buck2() {
    log_info "Running Buck2..."
    exec buck2 "$@"
}

# -----------------------------------------------------------------------------
# Main Entry Point
# -----------------------------------------------------------------------------

main() {
    local command="${1:-help}"
    shift 1 2>/dev/null || true

    case "$command" in
        orion|orion-worker)
            start_orion_worker "$@"
            ;;
        buck2)
            run_buck2 "$@"
            ;;
        bash|sh)
            print_versions
            exec /bin/bash "$@"
            ;;
        version|--version|-v)
            print_versions
            ;;
        help|--help|-h)
            show_help
            ;;
        *)
            # If the command is an executable, run it directly
            if command -v "$command" &>/dev/null; then
                exec "$command" "$@"
            else
                log_error "Unknown command: $command"
                echo ""
                show_help
                exit 1
            fi
            ;;
    esac
}

# Run main function
main "$@"
