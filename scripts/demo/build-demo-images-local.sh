#!/usr/bin/env bash
set -euo pipefail

# ============================================================================
# Local Image Builder for Demo Environment
# ============================================================================
# This script builds Docker images for the demo environment on your local machine.
# The script automatically detects the platform (arm64/amd64) based on your machine.
# By default, images are only built locally. Use --push to push to AWS ECR.
#
# Prerequisites:
#   - Docker Desktop with Buildx enabled
#   - (Optional) AWS CLI configured with credentials (only needed for --push)
#   - (Optional) Access to Amazon ECR Public (only needed for --push)
#   - QEMU (usually auto-installed by Docker Desktop)
#
# Usage:
#   ./scripts/demo/build-demo-images-local.sh [IMAGE_NAME] [--push]
#
#   Examples:
#     ./scripts/demo/build-demo-images-local.sh              # Build all images locally
#     ./scripts/demo/build-demo-images-local.sh --push       # Build and push all images
#     ./scripts/demo/build-demo-images-local.sh mono-engine  # Build mono-engine locally
#     ./scripts/demo/build-demo-images-local.sh mono-engine --push  # Build and push mono-engine
#
#   If IMAGE_NAME is provided, only that image will be built.
#   Otherwise, all 4 images will be built.
#   Use --push flag to push images to AWS ECR (requires AWS credentials).
# ============================================================================

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
REGISTRY_ALIAS="m8q5m4u3"
REPOSITORY="mega"
REGISTRY="public.ecr.aws"
SHOULD_PUSH=false  # Default: only build, don't push

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
            echo -e "\033[1;33m[WARN]\033[0m Unknown machine architecture: ${MACHINE_ARCH}, defaulting to ${TARGET_PLATFORMS}"
            ;;
    esac
    echo -e "\033[0;32m[INFO]\033[0m Auto-detected platform: ${TARGET_PLATFORMS} (machine: ${MACHINE_ARCH})"
else
    echo -e "\033[0;32m[INFO]\033[0m Using explicit TARGET_PLATFORMS: ${TARGET_PLATFORMS}"
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

# Image configurations (ordered for consistent build order)
declare -a IMAGE_ORDER=("mono-engine" "orion-server" "orion-client" "mega-ui")
declare -A IMAGES
IMAGES[mono-engine]="mono/Dockerfile:."
IMAGES[mega-ui]="moon/apps/web/Dockerfile:moon"
IMAGES[orion-server]="orion-server/Dockerfile:."
IMAGES[orion-client]="orion/Dockerfile:."

declare -A TAGS
TAGS[mono-engine]="mono-0.1.0-pre-release"
TAGS[mega-ui]="mega-ui-demo-0.1.0-pre-release"
TAGS[orion-server]="orion-server-0.1.0-pre-release"
TAGS[orion-client]="orion-client-0.1.0-pre-release"

# Functions
log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
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
    
    # Only check AWS prerequisites if we need to push
    if [ "$SHOULD_PUSH" = "true" ]; then
        # Check AWS CLI
        if ! command -v aws &> /dev/null; then
            log_error "AWS CLI is not installed. Please install it: brew install awscli"
            exit 1
        fi
        
        # Check AWS credentials
        if ! aws sts get-caller-identity &> /dev/null; then
            log_error "AWS credentials are not configured. Please run: aws configure"
            exit 1
        fi
    fi
    
    log_info "All prerequisites met ✓"
}

setup_buildx() {
    log_info "Setting up Docker Buildx..."
    
    local builder_name="mega-builder"

    # Reuse existing builder if present; otherwise create it.
    # NOTE: `docker buildx ls | grep` is not reliable enough (can false-match), so prefer `inspect`.
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

login_ecr() {
    log_info "Logging in to Amazon ECR Public..."
    
    if ! aws ecr-public get-login-password --region us-east-1 2>/dev/null | \
        docker login --username AWS --password-stdin "${REGISTRY}/${REGISTRY_ALIAS}" 2>/dev/null; then
        log_error "Failed to login to ECR Public. Please check your AWS credentials."
        exit 1
    fi
    
    log_info "ECR login successful ✓"
}

build_and_push() {
    local image_name=$1
    local dockerfile_path=$(echo "${IMAGES[$image_name]}" | cut -d':' -f1)
    local build_context=$(echo "${IMAGES[$image_name]}" | cut -d':' -f2)
    local image_tag="${TAGS[$image_name]}"
    
    # Validate single platform build (local script only supports single platform)
    # Check if TARGET_PLATFORMS contains comma (multiple platforms)
    if [[ "${TARGET_PLATFORMS}" == *","* ]]; then
        log_error "Multiple platforms detected in TARGET_PLATFORMS: ${TARGET_PLATFORMS}"
        log_error "This local script only supports single platform builds."
        log_error "Please set TARGET_PLATFORMS to a single platform (e.g., linux/arm64 or linux/amd64)."
        exit 1
    fi
    
    # Extract architecture suffix from single platform (e.g., linux/arm64 -> arm64)
    # This assumes TARGET_PLATFORMS is a single platform in format "os/arch"
    local arch_suffix=$(echo "${TARGET_PLATFORMS}" | awk -F'/' '{print $NF}')
    
    # Validate that we successfully extracted an architecture suffix
    if [ -z "${arch_suffix}" ]; then
        log_error "Failed to extract architecture suffix from TARGET_PLATFORMS: ${TARGET_PLATFORMS}"
        log_error "Expected format: os/arch (e.g., linux/arm64 or linux/amd64)"
        exit 1
    fi
    local image_tag_with_arch="${image_tag}-${arch_suffix}"
    local image_base="${REGISTRY}/${REGISTRY_ALIAS}/${REPOSITORY}"
    
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
    
    # Additional validation: check if Dockerfile is within or accessible from build context
    # For most cases, Dockerfile should be accessible from the build context
    local dockerfile_in_context="${full_context}/${dockerfile_path}"
    local dockerfile_relative_to_context=""
    
    # Try to find Dockerfile relative to context
    if [ -f "${dockerfile_in_context}" ]; then
        dockerfile_relative_to_context="${dockerfile_path}"
    elif [ -f "${full_dockerfile}" ]; then
        # Dockerfile is outside context, which is fine for buildx
        dockerfile_relative_to_context="${dockerfile_path}"
    else
        log_error "Cannot resolve Dockerfile path relative to build context"
        return 1
    fi
    
    if [ "$SHOULD_PUSH" = "true" ]; then
        log_info "Building and pushing ${image_name} (${image_tag_with_arch})..."
    else
        log_info "Building ${image_name} (${image_tag_with_arch})..."
    fi
    log_info "  Dockerfile: ${dockerfile_path}"
    log_info "  Context: ${build_context}"
    log_info "  Platforms: ${TARGET_PLATFORMS}"
    
    # Change to repo root for build
    cd "${REPO_ROOT}"
    
    # Build cache args - try to use registry cache, but don't fail if it doesn't exist
    # Only check registry cache if we're pushing (requires AWS access)
    local cache_from_args=()
    if [ "$SHOULD_PUSH" = "true" ]; then
        if docker manifest inspect "${image_base}:${image_tag_with_arch}" &> /dev/null 2>&1; then
            # Verify that existing image is a single-architecture image, not a manifest list
            local manifest_output
            manifest_output=$(docker manifest inspect "${image_base}:${image_tag_with_arch}" 2>/dev/null || echo "")
            
            if echo "$manifest_output" | grep -q '"manifests"'; then
                log_error "Error: ${image_base}:${image_tag_with_arch} is already a manifest list!"
                log_error "This script only builds single-architecture images."
                log_error "Please delete the incorrect manifest list from ECR first:"
                log_error "  aws ecr-public batch-delete-image \\"
                log_error "    --repository-name ${REPOSITORY} \\"
                log_error "    --registry-id ${REGISTRY_ALIAS} \\"
                log_error "    --image-ids imageTag=${image_tag_with_arch} \\"
                log_error "    --region us-east-1"
                log_error ""
                log_error "Or continue anyway - the build will overwrite it with a single-architecture image."
                log_warn "Continuing with build (will overwrite manifest list)..."
                # Continue anyway - buildx build will overwrite it
            else
                log_info "  Using registry cache from existing image (single-architecture)"
            fi
            cache_from_args=("--cache-from" "type=registry,ref=${image_base}:${image_tag_with_arch}")
        else
            log_info "  No existing image found, building from scratch"
        fi
    fi
    
    # Build command arguments
    local build_args=(
        --builder mega-builder
        --platform "${TARGET_PLATFORMS}"
        --file "${dockerfile_path}"
        --tag "${image_base}:${image_tag_with_arch}"
        --progress=plain
        --build-arg BUILDKIT_INLINE_CACHE=1
    )
    
    # Always load the image into the local Docker engine first.
    # This guarantees the pushed artifact is a single-architecture image manifest.
    build_args+=(--load)
    
    if [ "$image_name" = "mega-ui" ]; then
        build_args+=(--build-arg APP_ENV=demo)
    fi

    # Add cache arguments
    build_args+=("${cache_from_args[@]}")
    
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

    # Push if requested (ensures single-arch manifest is uploaded)
    if [ "$SHOULD_PUSH" = "true" ]; then
        if ! docker push "${image_base}:${image_tag_with_arch}"; then
            log_error "Failed to push ${image_name}"
            return 1
        fi
        log_info "${image_name} built and pushed successfully ✓"
        log_info "  Image: ${image_base}:${image_tag_with_arch}"
    else
        log_info "${image_name} built successfully ✓"
        log_info "  Image: ${image_base}:${image_tag_with_arch} (local only)"
    fi
    return 0
}

build_all() {
    if [ "$SHOULD_PUSH" = "true" ]; then
        log_info "Building and pushing all demo images..."
    else
        log_info "Building all demo images..."
    fi
    
    local failed_images=()
    for image_name in "${IMAGE_ORDER[@]}"; do
        echo ""
        log_info "=========================================="
        log_info "Building: ${image_name}"
        log_info "=========================================="
        if ! build_and_push "${image_name}"; then
            failed_images+=("${image_name}")
            log_error "Failed to build ${image_name}, continuing with next image..."
        fi
    done
    
    echo ""
    if [ ${#failed_images[@]} -eq 0 ]; then
        log_info "=========================================="
        if [ "$SHOULD_PUSH" = "true" ]; then
            log_info "All images built and pushed successfully!"
        else
            log_info "All images built successfully!"
        fi
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
    # Parse command line arguments
    # Extract --push flag and collect other arguments
    local args=()
    for arg in "$@"; do
        if [ "$arg" = "--push" ]; then
            SHOULD_PUSH=true
        else
            args+=("$arg")
        fi
    done
    
    # Determine operation mode
    local mode_info=""
    if [ "$SHOULD_PUSH" = "true" ]; then
        mode_info="build and push"
    else
        mode_info="build (local only)"
    fi
    
    log_info "Starting local image ${mode_info}..."
    log_info "Registry: ${REGISTRY}/${REGISTRY_ALIAS}/${REPOSITORY}"
    if [ "$SHOULD_PUSH" = "true" ]; then
        log_info "Mode: Build and push to ECR"
    else
        log_info "Mode: Build only (local)"
    fi
    echo ""
    
    check_prerequisites
    setup_buildx
    
    # Only login to ECR if we need to push
    if [ "$SHOULD_PUSH" = "true" ]; then
        login_ecr
    fi
    
    # Build specific image or all images
    if [ ${#args[@]} -eq 1 ]; then
        local image_name="${args[0]}"
        if [[ -v IMAGES[$image_name] ]]; then
            if build_and_push "$image_name"; then
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
    elif [ ${#args[@]} -eq 0 ]; then
        if build_all; then
            log_info "Done!"
            exit 0
        else
            log_error "Some builds failed"
            exit 1
        fi
    else
        log_error "Too many arguments. Usage: $0 [IMAGE_NAME] [--push]"
        log_info "Available images: ${IMAGE_ORDER[*]}"
        exit 1
    fi
}

# Run main function
main "$@"