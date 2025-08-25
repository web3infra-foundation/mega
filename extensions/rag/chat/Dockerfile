FROM rust:latest AS builder

# build args, to specify the build type, release or debug
ARG BUILD_TYPE=release

# 安装构建依赖
RUN apt-get update && apt-get install -y --no-install-recommends \
    cmake \
    pkg-config \
    libssl-dev \
    libz-dev \
    && rm -rf /var/lib/apt/lists/*

# 创建工作目录
WORKDIR /chat

# 复制整个工作空间
COPY . .

# build
RUN if [ "$BUILD_TYPE" = "release" ]; then \
    cargo build -p chat --release; \
else \
    cargo build -p chat;\
fi

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
COPY --from=builder /chat/target/release/chat /usr/local/bin/chat
COPY ./extensions/rag/chat/chat.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

# 设置默认命令
ENTRYPOINT ["/entrypoint.sh"]