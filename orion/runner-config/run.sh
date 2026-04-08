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
# 仅在缺少执行位时 chmod。NFS/EFS（如 root_squash）上对已可执行文件再 chmod 可能 EPERM，
# 且 run.sh 使用 set -e，会导致服务秒退。
if [ ! -x "./orion" ]; then
    if ! chmod +x ./orion; then
        echo "错误：无法为 orion 添加执行权限（常见于 NFS root_squash 等）。"
        echo "请在部署或存储端执行: sudo chmod +x $(pwd)/orion"
        exit 1
    fi
fi

# 设置 scorpio 配置路径
export SCORPIO_CONFIG="${SCORPIO_CONFIG:-$PWD/scorpio.toml}"
if [ ! -f "${SCORPIO_CONFIG}" ]; then
    echo "错误：找不到 SCORPIO_CONFIG 指定的文件: ${SCORPIO_CONFIG}"
    echo "请设置 SCORPIO_CONFIG，或在当前目录放置 scorpio.toml。"
    exit 1
fi

# ==============================================================================
#  Scorpiofs 本地存储锁检查（避免 sled DB 被其它残留进程占用导致 panic 退出 status=101）
# ==============================================================================
if command -v flock >/dev/null 2>&1; then
    # 注意：这些目录来自 runner-config/scorpio.toml 的 store_path（生产是 /data/scorpio/store）
    STORE_ROOT="/data/scorpio/store"
    for sub in path.db content.db meta.db; do
        db_file="${STORE_ROOT}/${sub}/db"
        # 如果文件不存在，sled 会在初始化时创建；这里先创建空文件用于 flock 探测。
        mkdir -p "$(dirname "${db_file}")" 2>/dev/null || true
        : > "${db_file}" 2>/dev/null || true
        if ! flock -n "${db_file}" -c "true" 2>/dev/null; then
            ts="$(date +%s)"
            echo "警告：检测到 sled DB 锁被占用：${db_file}"
            echo "  - 尝试将 ${STORE_ROOT}/${sub} 迁移为备份目录以恢复启动..."
            mv "${STORE_ROOT}/${sub}" "${STORE_ROOT}/${sub}.bak.${ts}" 2>/dev/null || true
            mkdir -p "${STORE_ROOT}/${sub}" 2>/dev/null || true
        fi
    done
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
