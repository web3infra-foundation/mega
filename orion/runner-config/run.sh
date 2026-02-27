#!/bin/bash

# ==============================================================================
#  Orion Runner 启动脚本（systemd ExecStart）
#
#  说明：
#  - Scorpio 已集成进 Orion（通过 scorpiofs），此脚本不再启动 scorpio 进程
#  - 每次启动前会清理旧进程和 FUSE 挂载，确保环境干净
#  - 配置：SCORPIO_CONFIG 环境变量 → 当前目录 scorpio.toml
# ==============================================================================

set -euo pipefail

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

# 检查 orion 二进制
if [ ! -f "./orion" ]; then
    echo "错误：未找到 orion 程序。"
    echo "请确保您在 orion-runner 目录下执行此脚本。"
    exit 1
fi
chmod +x ./orion

# 设置 scorpio 配置路径
export SCORPIO_CONFIG="${SCORPIO_CONFIG:-$PWD/scorpio.toml}"
if [ ! -f "${SCORPIO_CONFIG}" ]; then
    echo "错误：找不到 SCORPIO_CONFIG 指定的文件: ${SCORPIO_CONFIG}"
    echo "请设置 SCORPIO_CONFIG，或在当前目录放置 scorpio.toml。"
    exit 1
fi

# ==============================================================================
#  启动前清理：停止旧进程、卸载 FUSE
# ==============================================================================

echo "==> [1/3] 停止旧进程..."
if command -v buck2 &>/dev/null; then
    echo "  - 正在执行 'buck2 killall'..."
    buck2 killall || echo "  - 'buck2 killall' 执行完毕或没有进程可杀。"
fi
pkill -f "./orion" || echo "  - 没有找到 orion 进程。"

echo "==> [2/3] 卸载 FUSE 挂载点..."
MOUNT_DIR="$(read_scorpio_path "workspace" "${SCORPIO_TOML}")"
MOUNT_DIR="${MOUNT_DIR:-/workspace/mount}"

echo "  - 正在卸载 ${MOUNT_DIR}..."
fusermount -u "${MOUNT_DIR}" \
    || umount -l "${MOUNT_DIR}" \
    || umount -f "${MOUNT_DIR}" \
    || echo "  - 卸载可能未完全成功，将继续执行清理。"

# 若已卸载，则删除并重建目录，解决 "Transport endpoint" 问题
echo "  - 正在清理并重建挂载目录..."
if mountpoint -q "${MOUNT_DIR}"; then
    echo "  - ${MOUNT_DIR} 仍是挂载点，跳过删除目录。"
else
    rm -rf "${MOUNT_DIR}" || true
    mkdir -p "${MOUNT_DIR}"
fi
echo "  - 挂载点已清理。"

# ==============================================================================
#  启动 Orion
# ==============================================================================

echo "==> [3/3] 启动 orion..."
LOG_DIR="${PWD}/log"
ORION_LOG="${LOG_DIR}/orion.log"
mkdir -p "${LOG_DIR}"

echo "  - 日志: ${ORION_LOG}"
exec ./orion >>"${ORION_LOG}" 2>&1
