#!/bin/bash

# ==============================================================================
#  Orion Runner 启动脚本（systemd ExecStart）
#
#  说明：
#  - Scorpio 已集成进 Orion（通过 scorpiofs）
#  - 清理工作由 cleanup.sh（ExecStartPre）完成
#  - 此脚本只负责启动 orion 进程
# ==============================================================================

set -euo pipefail

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
#  启动 Orion
# ==============================================================================

echo "==> 启动 orion..."
LOG_DIR="${PWD}/log"
ORION_LOG="${LOG_DIR}/orion.log"
mkdir -p "${LOG_DIR}"

echo "  - 配置: ${SCORPIO_CONFIG}"
echo "  - 日志: ${ORION_LOG}"
exec ./orion >>"${ORION_LOG}" 2>&1
