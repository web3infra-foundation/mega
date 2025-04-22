FROM rust:latest AS builder

# 安装构建依赖
RUN apt-get update && apt-get install -y --no-install-recommends \
    cmake \
    pkg-config \
    libssl-dev \
    libz-dev \
    && rm -rf /var/lib/apt/lists/*

# 创建工作目录
WORKDIR /index

# 复制整个工作空间
COPY . .

# 构建项目
RUN cargo build --release

# 使用 debian 镜像作为运行时环境
FROM debian:latest

# 安装运行时依赖
RUN apt-get update && apt-get install -y --no-install-recommends \
    libssl-dev \
    libz-dev \
    git \   
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# 复制构建的二进制文件
COPY --from=builder /build/target/release/index /usr/local/bin/index

# 复制环境配置文件
COPY --from=builder /index /test

# 创建工作目录
WORKDIR /test

# 设置环境变量
ENV MEGA_DB_SQLX_LOGGING=false
ENV MEGA_DB_POSTGRESQL_URL="postgres://mega:mega@172.17.0.1:5432/repo_sync_status"

# 设置默认命令
CMD ["/bin/bash"]