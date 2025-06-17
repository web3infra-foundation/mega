#!/bin/bash

set -e
# 参数
TEST_DIR_BASE="/tmp"      # 测试目录基础路径

# 1. 创建一个时间相关的临时目录
timestamp=$(date +%Y%m%d%H%M)
git_dir="test_$timestamp"
base_dir="$TEST_DIR_BASE/$git_dir"
echo "创建目录：$base_dir"

# 2. 创建深层目录结构
generate_deep_structure() {
    local base_path=$1
    local prefix=$2
    
    # 创建深层目录路径：prefix/2/3/4/5/6
    local current_path="$base_path/$prefix"
    for level in {2..6}; do
        current_path="$current_path/$level"
        mkdir -p "$current_path"
        
        # 在每个层级创建一个1MB文件
        echo "创建文件：$current_path/1M_${prefix}_${level}.bin"
        head -c 1M </dev/urandom >"$current_path/1M_${prefix}_${level}.bin"
    done
}

# 创建两个深层目录结构
generate_deep_structure "$base_dir" "1"
generate_deep_structure "$base_dir" "2"

echo "文件结构创建完成："
echo "  深层目录结构1：1/2/3/4/5/6 (每层1个1MB文件)"
echo "  深层目录结构2：2/2/3/4/5/6 (每层1个1MB文件)"
echo "  总层数：6层"
echo "  总文件数：10个 (每个结构5个文件)"
echo "  总大小：10MB"