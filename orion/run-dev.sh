#!/bin/bash

# ==============================================================================
#  Orion 本地开发启动脚本
#
#  用法：
#    ./run-dev.sh          # 编译并运行
#    ./run-dev.sh --release # 编译 release 版本并运行
#
#  前置条件：
#  - 本地运行 mono 服务（默认 http://localhost:8000）
#  - 本地运行 orion-server（默认 ws://127.0.0.1:3000/ws）
# ==============================================================================

set -euo pipefail

cd "$(dirname "$0")"

SCORPIO_TOML="scorpio.toml"

# 从 scorpio.toml 读取路径
read_scorpio_path() {
    local key="$1"
    local file="$2"
    if [ -f "$file" ]; then
        awk -F'"' -v k="$key" '$1 ~ "^"k"[[:space:]]*=" {print $2; exit}' "$file"
    fi
}

# 加载环境变量
if [ -f "./.env" ]; then
    echo "==> 加载 .env..."
    set -a
    # shellcheck disable=SC1091
    source "./.env"
    set +a
fi

# 设置 scorpio 配置路径
export SCORPIO_CONFIG="${SCORPIO_CONFIG:-$PWD/scorpio.toml}"
if [ ! -f "${SCORPIO_CONFIG}" ]; then
    echo "错误：找不到 SCORPIO_CONFIG 指定的文件: ${SCORPIO_CONFIG}"
    exit 1
fi

# ==============================================================================
#  清理旧进程和 FUSE
# ==============================================================================

echo "==> [1/4] 停止旧进程..."
if command -v buck2 &>/dev/null; then
    buck2 killall 2>/dev/null || true
fi
pkill -f "target/.*orion" 2>/dev/null || echo "  - 没有找到 orion 进程。"

echo "==> [2/4] 卸载 FUSE 挂载点..."
MOUNT_DIR="$(read_scorpio_path "workspace" "${SCORPIO_TOML}")"
MOUNT_DIR="${MOUNT_DIR:-/tmp/megadir/mount}"

fusermount -u "${MOUNT_DIR}" 2>/dev/null \
    || umount -l "${MOUNT_DIR}" 2>/dev/null \
    || umount -f "${MOUNT_DIR}" 2>/dev/null \
    || true

# 清理并重建挂载目录
if ! mountpoint -q "${MOUNT_DIR}" 2>/dev/null; then
    rm -rf "${MOUNT_DIR}" 2>/dev/null || true
    mkdir -p "${MOUNT_DIR}"
fi

# 创建其他必要目录
STORE_DIR="$(read_scorpio_path "store_path" "${SCORPIO_TOML}")"
UPPER_DIR="$(read_scorpio_path "antares_upper_root" "${SCORPIO_TOML}")"
CL_DIR="$(read_scorpio_path "antares_cl_root" "${SCORPIO_TOML}")"
MNT_DIR="$(read_scorpio_path "antares_mount_root" "${SCORPIO_TOML}")"

mkdir -p "${STORE_DIR:-/tmp/megadir/store}"
mkdir -p "${UPPER_DIR:-/tmp/megadir/antares/upper}"
mkdir -p "${CL_DIR:-/tmp/megadir/antares/cl}"
mkdir -p "${MNT_DIR:-/tmp/megadir/antares/mnt}"
mkdir -p "${TMP_BUCKOUT_DIR:-/tmp/megadir/tmp_build}"

echo "  - 目录已准备就绪。"

# ==============================================================================
#  编译并运行
# ==============================================================================

BUILD_MODE="debug"
CARGO_FLAGS=""
if [[ "${1:-}" == "--release" ]]; then
    BUILD_MODE="release"
    CARGO_FLAGS="--release"
fi

echo "==> [3/4] 编译 orion（${BUILD_MODE}）..."
cargo build -p orion ${CARGO_FLAGS}

echo "==> [4/4] 启动 orion..."
BINARY="../target/${BUILD_MODE}/orion"
if [ ! -f "${BINARY}" ]; then
    BINARY="target/${BUILD_MODE}/orion"
fi

exec "${BINARY}"
