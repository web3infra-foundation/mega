#!/bin/bash

# ==============================================================================
#  Orion Runner 清理脚本（systemd ExecStartPre）
#
#  说明：
#  - 在服务启动前执行，清理旧进程和 FUSE 挂载
#  - 即使失败也不会阻止服务启动
# ==============================================================================

set +e  # 允许命令失败，不中断脚本

SCORPIO_TOML="${SCORPIO_CONFIG:-/home/orion/orion-runner/scorpio.toml}"

# 从 scorpio.toml 读取路径
read_scorpio_path() {
    local key="$1"
    local file="$2"
    if [ -f "$file" ]; then
        awk -F'"' -v k="$key" '$1 ~ "^"k"[[:space:]]*=" {print $2; exit}' "$file"
    fi
}

echo "==> [清理] 停止旧进程..."
if command -v buck2 &>/dev/null; then
    echo "  - 正在执行 'buck2 killall'..."
    buck2 killall 2>&1 || echo "  - buck2 killall 完成"
fi

# Only kill the orion binary, not this cleanup script
# Use full path match to avoid killing ourselves
if pgrep -f "/orion-runner/orion" >/dev/null 2>&1; then
    echo "  - 正在终止旧的 orion 进程..."
    pkill -9 -f "/orion-runner/orion" 2>&1 || echo "  - 进程清理完成"
else
    echo "  - 没有找到运行中的 orion 进程"
fi

echo "==> [清理] 卸载 FUSE 挂载点..."
MOUNT_DIR="$(read_scorpio_path "workspace" "${SCORPIO_TOML}")"
MOUNT_DIR="${MOUNT_DIR:-/workspace/mount}"

echo "  - 正在卸载 ${MOUNT_DIR}..."
fusermount -uz "${MOUNT_DIR}" 2>/dev/null || true
umount -lf "${MOUNT_DIR}" 2>/dev/null || true

# 清理并重建挂载目录
if ! mountpoint -q "${MOUNT_DIR}" 2>/dev/null; then
    rm -rf "${MOUNT_DIR}" 2>/dev/null || true
    mkdir -p "${MOUNT_DIR}" 2>/dev/null || true
    echo "  - 挂载点已清理"
else
    echo "  - ${MOUNT_DIR} 仍是挂载点，跳过删除"
fi

echo "==> [清理] 完成"
exit 0  # 总是返回成功
