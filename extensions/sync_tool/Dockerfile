FROM rust:latest AS builder

# 安装构建依赖
RUN apt-get update && apt-get install -y --no-install-recommends \
    cmake \
    pkg-config \
    libssl-dev \
    libz-dev \
    && rm -rf /var/lib/apt/lists/*

# 复制源码并构建
COPY . /sync_tool
WORKDIR /sync_tool
RUN cargo build --release

# 使用 debian 镜像作为运行时环境
FROM debian:latest

# 安装运行时依赖
RUN apt-get update && apt-get install -y --no-install-recommends \
    libssl-dev \
    libz-dev \
    && rm -rf /var/lib/apt/lists/*

# 复制构建的二进制文件
COPY --from=builder /sync_tool /workspace

WORKDIR /workspace
# 设置默认命令
CMD ["sync_tool"]

