#!/bin/bash

# ==============================================================================
#  Orion Runner 预检脚本（systemd ExecStartPre）
#
#  说明：
#  - 启动前检查 FUSE 依赖是否就绪，避免运行中触发 panic
#  - 任一检查失败时直接退出非 0，让 systemd 阻断启动
# ==============================================================================

set -euo pipefail

CAP_SYS_ADMIN_MASK=$((1 << 21))

echo "==> [预检] 检查 /dev/fuse ..."
if [ ! -e /dev/fuse ]; then
    echo "错误：/dev/fuse 不存在，无法执行 FUSE 挂载。"
    exit 1
fi
if [ ! -r /dev/fuse ] || [ ! -w /dev/fuse ]; then
    echo "错误：当前进程对 /dev/fuse 缺少读写权限。"
    exit 1
fi

echo "==> [预检] 检查 CAP_SYS_ADMIN ..."
CAP_EFF_HEX="$(awk '/^CapEff:/ {print $2}' /proc/self/status)"
if [ -z "${CAP_EFF_HEX}" ]; then
    echo "错误：无法从 /proc/self/status 读取 CapEff。"
    exit 1
fi

if (( (16#${CAP_EFF_HEX} & CAP_SYS_ADMIN_MASK) == 0 )); then
    echo "错误：当前进程缺少 CAP_SYS_ADMIN，无法执行 FUSE 挂载。CapEff=${CAP_EFF_HEX}"
    exit 1
fi

echo "==> [预检] 通过"
