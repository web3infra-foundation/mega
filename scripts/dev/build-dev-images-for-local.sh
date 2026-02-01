#!/usr/bin/env bash
set -euo pipefail

# ============================================================================
# Local Image Builder for Demo Environment
# ============================================================================
# This script builds Docker images for the demo environment on your local machine.
# The script automatically detects the platform (arm64/amd64) based on your machine.
#
# Prerequisites:
#   - Docker Desktop with Buildx enabled
#   - QEMU (usually auto-installed by Docker Desktop)
#   - git (for versioning)
#
# Usage:
#   ./scripts/dev/build-dev-images-for-local.sh [IMAGE_NAME]
#
#   Examples:
#     ./scripts/dev/build-dev-images-for-local.sh              # Build all images locally
#     ./scripts/dev/build-dev-images-for-local.sh mono-engine  # Build mono-engine locally
#
#   If IMAGE_NAME is provided, only that image will be built.
#   Otherwise, all 4 images will be built.
# ============================================================================

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
REPOSITORY="mega-dev"

# Auto-detect platform if not explicitly set
# The script automatically detects your machine architecture and sets the appropriate platform
if [ -z "${TARGET_PLATFORMS:-}" ]; then
    MACHINE_ARCH=$(uname -m)
    case "${MACHINE_ARCH}" in
        arm64|aarch64)
            TARGET_PLATFORMS="linux/arm64"
            ;;
        x86_64|amd64)
            TARGET_PLATFORMS="linux/amd64"
            ;;
        *)
            # Default to arm64 for compatibility (e.g., macOS Apple Silicon)
            TARGET_PLATFORMS="linux/arm64"
            printf "\033[1;33m[WARN]\033[0m Unknown machine architecture: %s, defaulting to %s\n" "${MACHINE_ARCH}" "${TARGET_PLATFORMS}"
            ;;
    esac
    printf "\033[0;32m[INFO]\033[0m Auto-detected platform: %s (machine: %s)\n" "${TARGET_PLATFORMS}" "${MACHINE_ARCH}"
else
    printf "\033[0;32m[INFO]\033[0m Using explicit TARGET_PLATFORMS: %s\n" "${TARGET_PLATFORMS}"
fi

# Get script directory and repo root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# Try to find repo root by looking for known files
if [ -f "${SCRIPT_DIR}/../../Cargo.toml" ]; then
    REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
elif [ -f "${SCRIPT_DIR}/../../moon/package.json" ]; then
    REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
elif [ -f "${PWD}/Cargo.toml" ] || [ -f "${PWD}/moon/package.json" ]; then
    REPO_ROOT="${PWD}"
else
    REPO_ROOT=""
fi

# Get git hash
if ! command -v git &> /dev/null; then
    printf "${RED}[ERROR]${NC} git is not installed.\n"
    exit 1
fi
GIT_HASH=$(git rev-parse --short HEAD)

# Image configurations (ordered for consistent build order)
declare -a IMAGE_ORDER=("mono-engine" "orion-server" "orion-client" "mega-ui")
get_image_config() {
    case "$1" in
        "mono-engine") echo "mono/Dockerfile:." ;;
        "mega-ui") echo "moon/apps/web/Dockerfile:moon" ;;
        "orion-server") echo "orion-server/Dockerfile:." ;;
        "orion-client") echo "orion/Dockerfile:." ;;
    esac
}

get_image_tag() {
    case "$1" in
        "mono-engine") echo "mono-${GIT_HASH}" ;;
        "mega-ui") echo "mega-ui-demo-${GIT_HASH}" ;;
        "orion-server") echo "orion-server-${GIT_HASH}" ;;
        "orion-client") echo "orion-client-${GIT_HASH}" ;;
    esac
}

is_valid_image() {
    case "$1" in
        "mono-engine"|"mega-ui"|"orion-server"|"orion-client") return 0 ;;
        *) return 1 ;;
    esac
}

# Functions
log_info() {
    printf "${GREEN}[INFO]${NC} %s\n" "$1"
}

log_warn() {
    printf "${YELLOW}[WARN]${NC} %s\n" "$1"
}

log_error() {
    printf "${RED}[ERROR]${NC} %s\n" "$1"
}

check_prerequisites() {
    log_info "Checking prerequisites..."
    
    # Check if we found repo root
    if [ -z "${REPO_ROOT}" ] || [ ! -d "${REPO_ROOT}" ]; then
        log_error "Could not determine repository root."
        log_error "Please run this script from the mega repository root, or ensure Cargo.toml or moon/package.json exists."
        exit 1
    fi
    
    # Verify repo root
    if [ ! -f "${REPO_ROOT}/Cargo.toml" ] && [ ! -f "${REPO_ROOT}/moon/package.json" ]; then
        log_error "Repository root validation failed: ${REPO_ROOT}"
        log_error "Expected to find Cargo.toml or moon/package.json in repository root."
        exit 1
    fi
    
    log_info "Repository root: ${REPO_ROOT}"
    
    # Check Docker
    if ! command -v docker &> /dev/null; then
        log_error "Docker is not installed. Please install Docker Desktop."
        exit 1
    fi
    
    # Check if Docker is running
    if ! docker info &> /dev/null; then
        log_error "Docker daemon is not running. Please start Docker Desktop."
        exit 1
    fi
    
    # Check Docker Buildx
    if ! docker buildx version &> /dev/null; then
        log_error "Docker Buildx is not available. Please enable it in Docker Desktop."
        exit 1
    fi
    
    log_info "All prerequisites met ✓"
}

setup_buildx() {
    log_info "Setting up Docker Buildx..."
    
    local builder_name="mega-builder"

    # Reuse existing builder if present; otherwise create it.
    if docker buildx inspect "${builder_name}" >/dev/null 2>&1; then
        log_info "Using existing buildx builder: ${builder_name}"
        docker buildx use "${builder_name}" >/dev/null
    else
        log_info "Creating new buildx builder: ${builder_name}"
        if ! docker buildx create --name "${builder_name}" --driver docker-container --use >/dev/null; then
            log_error "Failed to create buildx builder: ${builder_name}"
            exit 1
        fi
    fi

    # Ensure the builder is bootstrapped (idempotent)
    if ! docker buildx inspect "${builder_name}" --bootstrap >/dev/null 2>&1; then
        log_error "Failed to bootstrap buildx builder: ${builder_name}"
        exit 1
    fi
    
    # Verify platform support
    local platforms_output
    platforms_output=$(docker buildx inspect "${builder_name}" --bootstrap 2>/dev/null | grep -i "platforms" || echo "")
    
    # Parse TARGET_PLATFORMS (comma-separated) and check each platform
    IFS=',' read -ra REQUESTED_PLATFORMS <<< "${TARGET_PLATFORMS}"
    local missing_platforms=()
    
    for platform in "${REQUESTED_PLATFORMS[@]}"; do
        # Trim whitespace
        platform=$(echo "$platform" | xargs)
        if [[ ! "$platforms_output" =~ "$platform" ]]; then
            missing_platforms+=("$platform")
        fi
    done
    
    if [ ${#missing_platforms[@]} -gt 0 ]; then
        log_warn "Some requested platforms may not be fully enabled:"
        for missing in "${missing_platforms[@]}"; do
            log_warn "  - ${missing}"
        done
        log_warn "Platforms available: $platforms_output"
        log_warn "Continuing anyway..."
    fi
    
    log_info "Buildx setup complete ✓"
}

build_image() {
    local image_name=$1
    local config=$(get_image_config "$image_name")
    local dockerfile_path=$(echo "$config" | cut -d':' -f1)
    local build_context=$(echo "$config" | cut -d':' -f2)
    local image_tag=$(get_image_tag "$image_name")
    
    # Validate single platform build (local script only supports single platform)
    # Check if TARGET_PLATFORMS contains comma (multiple platforms)
    if [[ "${TARGET_PLATFORMS}" == *","* ]]; then
        log_error "Multiple platforms detected in TARGET_PLATFORMS: ${TARGET_PLATFORMS}"
        log_error "This local script only supports single platform builds."
        log_error "Please set TARGET_PLATFORMS to a single platform (e.g., linux/arm64 or linux/amd64)."
        exit 1
    fi
    
    # Extract architecture suffix from single platform (e.g., linux/arm64 -> arm64)
    local arch_suffix=$(echo "${TARGET_PLATFORMS}" | awk -F'/' '{print $NF}')
    
    # Validate that we successfully extracted an architecture suffix
    if [ -z "${arch_suffix}" ]; then
        log_error "Failed to extract architecture suffix from TARGET_PLATFORMS: ${TARGET_PLATFORMS}"
        log_error "Expected format: os/arch (e.g., linux/arm64 or linux/amd64)"
        exit 1
    fi
    local image_tag_with_arch="${image_tag}-${arch_suffix}"
    local image_base="${REPOSITORY}"
    local latest_tag="${image_tag%-${GIT_HASH}}-latest"
    
    # Verify paths exist (use absolute paths)
    local full_dockerfile="${REPO_ROOT}/${dockerfile_path}"
    local full_context="${REPO_ROOT}/${build_context}"
    
    if [ ! -f "${full_dockerfile}" ]; then
        log_error "Dockerfile not found: ${full_dockerfile}"
        return 1
    fi
    
    if [ ! -d "${full_context}" ]; then
        log_error "Build context not found: ${full_context}"
        return 1
    fi
    
    log_info "Building ${image_name} (${image_tag_with_arch})..."
    log_info "  Dockerfile: ${dockerfile_path}"
    log_info "  Context: ${build_context}"
    log_info "  Platforms: ${TARGET_PLATFORMS}"
    
    # Change to repo root for build
    cd "${REPO_ROOT}"
    
    # Build command arguments
    local build_args=(
        --builder mega-builder
        --platform "${TARGET_PLATFORMS}"
        --file "${dockerfile_path}"
        --tag "${image_base}:${image_tag_with_arch}"
        --tag "${image_base}:${latest_tag}"
        --progress=plain
        --build-arg BUILDKIT_INLINE_CACHE=1
    )
    
    # Always load the image into the local Docker engine first.
    build_args+=(--load)
    
    if [ "$image_name" = "mega-ui" ]; then
        build_args+=(--build-arg APP_ENV=demo)
    fi

    # Add cache-to (inline cache is always useful)
    build_args+=(--cache-to type=inline)
    
    # Add build context
    build_args+=("${build_context}")
    
    log_info "output build args: ${build_args[*]}"
    
    # Build (load into local engine)
    if ! docker buildx build "${build_args[@]}"; then
        log_error "Failed to build ${image_name}"
        return 1
    fi

    log_info "${image_name} built successfully ✓"
    log_info "  Image: ${image_base}:${image_tag_with_arch} (local only)"
    log_info "  Latest: ${image_base}:${latest_tag}"
    return 0
}

build_all() {
    log_info "Building all demo images..."
    
    local failed_images=()
    for image_name in "${IMAGE_ORDER[@]}"; do
        echo ""
        log_info "=========================================="
        log_info "Building: ${image_name}"
        log_info "=========================================="
        if ! build_image "${image_name}"; then
            failed_images+=("${image_name}")
            log_error "Failed to build ${image_name}, continuing with next image..."
        fi
    done
    
    echo ""
    if [ ${#failed_images[@]} -eq 0 ]; then
        log_info "=========================================="
        log_info "All images built successfully!"
        log_info "=========================================="
        return 0
    else
        log_error "=========================================="
        log_error "Some images failed to build:"
        for img in "${failed_images[@]}"; do
            log_error "  - ${img}"
        done
        log_error "=========================================="
        return 1
    fi
}

# Main execution
main() {
    log_info "Starting local image build..."
    log_info "Repository: ${REPOSITORY}"
    log_info "Git Hash: ${GIT_HASH}"
    echo ""
    
    check_prerequisites
    setup_buildx
    
    # Build specific image or all images
    if [ $# -eq 1 ]; then
        local image_name="$1"
        if is_valid_image "$image_name"; then
            if build_image "$image_name"; then
                log_info "Done!"
                exit 0
            else
                log_error "Build failed"
                exit 1
            fi
        else
            log_error "Unknown image: $image_name"
            log_info "Available images: ${IMAGE_ORDER[*]}"
            exit 1
        fi
    elif [ $# -eq 0 ]; then
        if build_all; then
            log_info "Done!"
            exit 0
        else
            log_error "Some builds failed"
            exit 1
        fi
    else
        log_error "Too many arguments. Usage: $0 [IMAGE_NAME]"
        log_info "Available images: ${IMAGE_ORDER[*]}"
        exit 1
    fi
}

# Run main function
main "$@"
