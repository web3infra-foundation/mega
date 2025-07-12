#!/bin/bash

set -e
# 参数
TEST_DIR_BASE="/tmp"      # 测试目录基础路径
TOTAL_FILES=1000          # 总文件数
MAX_DEPTH=6               # 最大目录深度

# 1. 创建一个时间相关的临时目录
timestamp=$(date +%Y%m%d%H%M)
git_dir="test_$timestamp"
base_dir="$TEST_DIR_BASE/$git_dir"
echo "创建目录：$base_dir"
mkdir -p "$base_dir"

# 2. 创建深层目录结构 (类似原始run.sh但支持更多文件)
generate_deep_structure() {
    local base_path=$1
    local prefix=$2
    local depth_2_3_files=$3     # 深度2和3的文件数量
    local depth_4_files=$4       # 深度4的文件数量
    local depth_5_6_files=$5     # 深度5和6的文件数量
    
    # 创建深层目录路径：prefix/2/3/4/5/6
    local current_path="$base_path/$prefix"
    for level in {2..6}; do
        current_path="$current_path/$level"
        mkdir -p "$current_path"
        
        # 根据深度决定文件大小 (缩小10倍，总大小控制在100MB)
        local file_size_kb
        local files_count
        case $level in
            2) file_size_kb=100; files_count=$depth_2_3_files ;;   # 原来1MB -> 100KB
            3) file_size_kb=80; files_count=$depth_2_3_files ;;    # 原来800KB -> 80KB  
            4) file_size_kb=60; files_count=$depth_4_files ;;      # 原来600KB -> 60KB
            5) file_size_kb=40; files_count=$depth_5_6_files ;;    # 原来400KB -> 40KB
            6) file_size_kb=20; files_count=$depth_5_6_files ;;    # 原来200KB -> 20KB
        esac
        
        # 在每个层级创建多个文件
        for ((i=1; i<=files_count; i++)); do
            local filename="${file_size_kb}K_${prefix}_${level}_${i}.bin"
            echo "创建文件：$current_path/$filename"
            head -c ${file_size_kb}K </dev/urandom >"$current_path/$filename"
        done
    done
}

# 3. 计算文件分布以达到1000个文件
calculate_file_distribution() {
    echo "文件分布规划（基于原始run.sh结构，但扩展到1000个文件）："
    echo "  两个主要分支：1/ 和 2/"
    echo "  每个分支各有5个深度层级 (2-6)"
    echo "  每个层级的文件数量："
    echo "    深度2: 每分支50个文件，100KB each (100个文件，10MB)"
    echo "    深度3: 每分支50个文件，80KB each (100个文件，8MB)"  
    echo "    深度4: 每分支100个文件，60KB each (200个文件，12MB)"
    echo "    深度5: 每分支150个文件，40KB each (300个文件，12MB)"
    echo "    深度6: 每分支150个文件，20KB each (300个文件，6MB)"
    echo "  总计: 1000个文件，约48MB"
}

# 4. 主要执行逻辑
main() {
    echo "开始创建1000个文件的测试结构..."
    calculate_file_distribution
    
    local created_files=0
    
    # 创建两个深层目录结构，每个结构500个文件
    echo "创建结构1..."
    # 深度2: 50个文件，深度3: 50个文件，深度4: 100个文件，深度5: 150个文件，深度6: 150个文件
    generate_deep_structure "$base_dir" "1" 50 100 150
    created_files=$((created_files + 500))
    
    echo "创建结构2..."
    generate_deep_structure "$base_dir" "2" 50 100 150
    created_files=$((created_files + 500))
    
    echo ""
    echo "文件创建完成！"
    echo "  深层目录结构1：1/2/3/4/5/6 (深度2,3各50个文件，深度4各100个文件，深度5,6各150个文件)"
    echo "  深层目录结构2：2/2/3/4/5/6 (深度2,3各50个文件，深度4各100个文件，深度5,6各150个文件)"
    echo "  总层数：6层"
    echo "  总文件数：$created_files"
    echo "  文件大小：20KB-100KB（随深度递减）"
    echo "  预估总大小：约48MB"
    echo "  基础目录: $base_dir"
    
    # 统计信息
    echo ""
    echo "统计信息："
    echo "  总目录数: $(find "$base_dir" -type d | wc -l)"
    echo "  总文件数: $(find "$base_dir" -type f | wc -l)"
    echo "  总大小: $(du -sh "$base_dir" | cut -f1)"
    
    # 显示目录结构示例
    echo ""
    echo "目录结构示例："
    tree "$base_dir" -L 3 -d 2>/dev/null || echo "  (tree命令未安装，无法显示结构)"
    
    echo ""
    echo "使用方法："
    echo "  cd $base_dir"
    echo "  ls -la"
    echo "  find . -name '*.bin' | head -10"
}

# 执行主函数
main
