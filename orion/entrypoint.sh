#!/bin/bash
# =============================================================================
# Mega Dev Image - Entrypoint Script
# =============================================================================
# This script handles container startup for different services:
# - orion: Start the Orion worker (build execution client)
# - scorpio: Start the Scorpio FUSE filesystem service (daemon)
# - buck2: Run Buck2 commands
# - bash/sh: Interactive shell
# - help: Show usage information
# =============================================================================

set -e

# -----------------------------------------------------------------------------
# Configuration
# -----------------------------------------------------------------------------
CONFIG_DIR="/app/config"
SCORPIO_CONFIG="/app/config/scorpio.toml"
SCORPIO_TEMPLATE="/app/config/scorpio.toml.template"

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
  scorpio         Start the Scorpio FUSE filesystem service
  buck2 [ARGS]    Run Buck2 with specified arguments
  bash, sh        Start an interactive shell
  help            Show this help message

Examples:
  # Start Scorpio (requires privileged mode for FUSE)
  docker run -d --name scorpio \
    --privileged \
    -e SCORPIO_BASE_URL=http://mega-server:8000 \
    -p 2725:2725 \
    mega-dev:latest scorpio

  # Mount repo and run Buck2 (mountpoints are visible only inside the Scorpio container)
  #   MOUNTPOINT=$(curl -s -X POST http://localhost:2725/antares/mounts \
  #     -H "Content-Type: application/json" \
  #     -d '{"path":"your/real/repo/path","job_id":"manual"}' | jq -r '.mountpoint')
  #   docker exec scorpio bash -lc "cd '${MOUNTPOINT}' && buck2 build //..."

  # Start Orion Worker (includes Buck2 execution; typically needs Scorpio + FUSE)
  docker run -d --name orion-worker \
    --privileged \
    -e SERVER_WS=ws://your-orion-server:8004/ws \
    -e SCORPIO_BASE_URL=http://mega-server:8000 \
    mega-dev:latest orion

  # Run Buck2 build
  docker run --rm -v $(pwd):/workspace mega-dev:latest buck2 build //...

  # Interactive development
  docker run -it --rm -v $(pwd):/workspace mega-dev:dev bash

Environment Variables:
  See .env.example for full list of configurable options.

For more information, see the README(orion-client-image).md in orion/
EOF
}

# Generate Scorpio config from template
generate_scorpio_config() {
    log_info "Generating Scorpio configuration..."

    if [ -f "$SCORPIO_TEMPLATE" ]; then
        # Defaults (must be set before envsubst; it does not support "${VAR:-default}" syntax)
        : "${SCORPIO_BASE_URL:=http://host.docker.internal:8000}"
        : "${SCORPIO_LFS_URL:=${SCORPIO_BASE_URL}}"
        : "${SCORPIO_STORE_PATH:=/data/scorpio/store}"
        : "${SCORPIO_WORKSPACE:=/workspace/mount}"
        : "${SCORPIO_GIT_AUTHOR:=MEGA}"
        : "${SCORPIO_GIT_EMAIL:=admin@mega.org}"
        : "${SCORPIO_DICFUSE_READABLE:=true}"
        : "${SCORPIO_LOAD_DIR_DEPTH:=3}"
        : "${SCORPIO_FETCH_FILE_THREAD:=10}"
        : "${SCORPIO_DICFUSE_IMPORT_CONCURRENCY:=4}"
        : "${SCORPIO_DICFUSE_DIR_SYNC_TTL_SECS:=5}"
        : "${SCORPIO_DICFUSE_STAT_MODE:=accurate}"
        : "${SCORPIO_DICFUSE_OPEN_BUFF_MAX_BYTES:=268435456}"
        : "${SCORPIO_DICFUSE_OPEN_BUFF_MAX_FILES:=4096}"

        : "${ANTARES_LOAD_DIR_DEPTH:=0}"
        : "${ANTARES_DICFUSE_STAT_MODE:=fast}"
        : "${ANTARES_DICFUSE_OPEN_BUFF_MAX_BYTES:=67108864}"
        : "${ANTARES_DICFUSE_OPEN_BUFF_MAX_FILES:=1024}"
        : "${ANTARES_DICFUSE_DIR_SYNC_TTL_SECS:=5}"
        : "${ANTARES_UPPER_ROOT:=/data/scorpio/antares/upper}"
        : "${ANTARES_CL_ROOT:=/data/scorpio/antares/cl}"
        : "${ANTARES_MOUNT_ROOT:=/data/scorpio/antares/mnt}"
        : "${ANTARES_STATE_FILE:=/data/scorpio/antares/state.toml}"

        export \
            SCORPIO_BASE_URL \
            SCORPIO_LFS_URL \
            SCORPIO_STORE_PATH \
            SCORPIO_WORKSPACE \
            SCORPIO_GIT_AUTHOR \
            SCORPIO_GIT_EMAIL \
            SCORPIO_DICFUSE_READABLE \
            SCORPIO_LOAD_DIR_DEPTH \
            SCORPIO_FETCH_FILE_THREAD \
            SCORPIO_DICFUSE_IMPORT_CONCURRENCY \
            SCORPIO_DICFUSE_DIR_SYNC_TTL_SECS \
            SCORPIO_DICFUSE_STAT_MODE \
            SCORPIO_DICFUSE_OPEN_BUFF_MAX_BYTES \
            SCORPIO_DICFUSE_OPEN_BUFF_MAX_FILES \
            ANTARES_LOAD_DIR_DEPTH \
            ANTARES_DICFUSE_STAT_MODE \
            ANTARES_DICFUSE_OPEN_BUFF_MAX_BYTES \
            ANTARES_DICFUSE_OPEN_BUFF_MAX_FILES \
            ANTARES_DICFUSE_DIR_SYNC_TTL_SECS \
            ANTARES_UPPER_ROOT \
            ANTARES_CL_ROOT \
            ANTARES_MOUNT_ROOT \
            ANTARES_STATE_FILE

        # Use envsubst to replace environment variables in template
        envsubst < "$SCORPIO_TEMPLATE" > "$SCORPIO_CONFIG"
        log_info "Scorpio config generated at: $SCORPIO_CONFIG"
    else
        log_warn "Scorpio template not found at $SCORPIO_TEMPLATE"
    fi
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

# Create required directories
setup_directories() {
    log_info "Setting up directories..."

    mkdir -p "${SCORPIO_STORE_PATH:-/data/scorpio/store}"
    mkdir -p "${SCORPIO_WORKSPACE:-/workspace/mount}"
    mkdir -p "${ANTARES_UPPER_ROOT:-/data/scorpio/antares/upper}"
    mkdir -p "${ANTARES_CL_ROOT:-/data/scorpio/antares/cl}"
    mkdir -p "${ANTARES_MOUNT_ROOT:-/data/scorpio/antares/mnt}"
}

# Check FUSE availability for Scorpio
check_fuse() {
    if [ ! -e /dev/fuse ]; then
        log_error "/dev/fuse not found!"
        log_error "Scorpio requires FUSE support. Please run the container with:"
        log_error "  --privileged"
        log_error "  or: --device /dev/fuse --cap-add SYS_ADMIN"
        exit 1
    fi
    log_info "FUSE device available"
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
    if command -v scorpio &>/dev/null; then
        echo "  Scorpio:      installed"
    else
        echo "  Scorpio:      missing"
    fi
    echo "  Buck2:        $(buck2 --version 2>/dev/null || echo 'unknown')"
    echo ""
    echo "Image Labels:"
    if [ -f /etc/mega-image-info ]; then
        cat /etc/mega-image-info
    fi
    echo ""
}

# -----------------------------------------------------------------------------
# Service Start Functions
# -----------------------------------------------------------------------------

start_scorpio() {
    log_header "Starting Scorpio"

    print_versions
    check_fuse
    setup_directories
    generate_scorpio_config

    log_info "Configuration:"
    echo "  BASE_URL: ${SCORPIO_BASE_URL:-http://localhost:8000}"
    echo "  STORE_PATH: ${SCORPIO_STORE_PATH:-/data/scorpio/store}"
    echo "  WORKSPACE: ${SCORPIO_WORKSPACE:-/workspace/mount}"
    echo ""

    log_info "Starting scorpio..."
    local extra_args=()
    # Optional: bind HTTP server to a custom address/port (supported by newer scorpio binaries).
    # If user already passed --http-addr in args, do not add a duplicate flag.
    if [ -n "${SCORPIO_HTTP_ADDR:-}" ]; then
        case " $* " in
            *" --http-addr "*|*" --http-addr="*) ;;
            *) extra_args+=(--http-addr "${SCORPIO_HTTP_ADDR}") ;;
        esac
    fi
    exec scorpio -c "$SCORPIO_CONFIG" "${extra_args[@]}" "$@"
}

start_orion_worker() {
    log_header "Starting Orion Worker"

    print_versions

    # Orion worker configuration
    : "${SERVER_WS:=ws://127.0.0.1:8004/ws}"
    : "${BUCK_PROJECT_ROOT:=/workspace}"
    : "${BUILD_TMP:=/tmp/orion-builds}"
    : "${SCORPIO_API_BASE_URL:=http://127.0.0.1:2725}"
    : "${ORION_WORKER_START_SCORPIO:=true}"

    setup_directories
    mkdir -p "${BUILD_TMP}"

    # Most workflows require Scorpio to run in the same container namespace as the worker,
    # so the mounted Antares/FUSE mount points are accessible to Buck2.
    if [ "${ORION_WORKER_START_SCORPIO}" != "false" ] && [ "${ORION_WORKER_START_SCORPIO}" != "0" ]; then
        log_info "Starting embedded Scorpio for worker..."
        check_fuse
        generate_scorpio_config

        local extra_args=()
        local http_addr="${SCORPIO_HTTP_ADDR:-0.0.0.0:2725}"
        # NOTE: Simple IPv4-style parsing. For IPv6 bind addresses, set SCORPIO_API_BASE_URL manually.
        local http_port="${http_addr##*:}"
        case "$http_port" in
            ''|*[!0-9]*)
                http_port="2725"
                ;;
        esac

        # Ensure the worker talks to the embedded Scorpio by default (respecting http port override).
        export SCORPIO_API_BASE_URL="http://127.0.0.1:${http_port}"

        # Optional: pass through --http-addr if provided (and not already present in args).
        if [ -n "${SCORPIO_HTTP_ADDR:-}" ]; then
            case " $* " in
                *" --http-addr "*|*" --http-addr="*) ;;
                *) extra_args+=(--http-addr "${SCORPIO_HTTP_ADDR}") ;;
            esac
        fi

        scorpio -c "$SCORPIO_CONFIG" "${extra_args[@]}" &
        local scorpio_pid=$!

        cleanup() {
            if kill -0 "${scorpio_pid}" 2>/dev/null; then
                log_info "Stopping embedded Scorpio (pid=${scorpio_pid})..."
                kill "${scorpio_pid}" 2>/dev/null || true
            fi
        }

        trap cleanup EXIT INT TERM

        wait_for_service "127.0.0.1" "${http_port}" 60 || exit 1
    else
        log_warn "ORION_WORKER_START_SCORPIO disabled; ensure Scorpio mountpoints are accessible to this container."
    fi

    log_info "Configuration:"
    echo "  SERVER_WS: ${SERVER_WS}"
    echo "  ORION_WORKER_ID: ${ORION_WORKER_ID:-(auto)}"
    echo "  BUCK_PROJECT_ROOT: ${BUCK_PROJECT_ROOT}"
    echo "  BUILD_TMP: ${BUILD_TMP}"
    echo "  SCORPIO_API_BASE_URL: ${SCORPIO_API_BASE_URL}"
    echo "  SCORPIO_BASE_URL: ${SCORPIO_BASE_URL:-(not set)}"
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
        scorpio)
            start_scorpio "$@"
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
