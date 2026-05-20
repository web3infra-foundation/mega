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

echo "==> [预检] 检查 buck2 ..."
if ! command -v buck2 >/dev/null 2>&1; then
    echo "错误：buck2 不在 PATH 中，请安装 buck2 到 /usr/local/bin/buck2。"
    exit 1
fi
echo "  - buck2 路径: $(which buck2)"

# Use buck2 --version (not buck2 version)
set +e
BUCK2_OUTPUT=$(buck2 --version 2>&1)
BUCK2_STATUS=$?
set -e
echo "  - buck2 版本: $BUCK2_OUTPUT"
if [ $BUCK2_STATUS -ne 0 ]; then
    echo "错误：buck2 执行失败 (exit $BUCK2_STATUS)。"
    exit 1
fi
echo "  - buck2 就绪"

echo "==> [预检] 通过"
