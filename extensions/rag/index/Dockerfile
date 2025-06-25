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
WORKDIR /index

# 复制整个工作空间
COPY . .


# build
RUN if [ "$BUILD_TYPE" = "release" ]; then \
    cargo build -p index --release; \
else \
    cargo build -p index; \
fi

# 使用 debian 镜像作为运行时环境
FROM debian:latest

# 安装运行时依赖
RUN apt-get update && apt-get install -y --no-install-recommends \
    libssl-dev \
    libz-dev \
    git \
    python3 \
    python3-pip \
    wget \
    python3-venv \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*



    # 复制 requirements.txt
COPY ./extensions/rag/index/requirements.txt .

# 安装 Python 依赖

RUN python3 -m venv /opt/venv
ENV PATH="/opt/venv/bin:$PATH"
RUN pip3 install --no-cache-dir -r requirements.txt

# 复制构建的二进制文件
COPY --from=builder /index/target/release/index /usr/local/bin/index
COPY --from=builder /index/extensions/sync.py /usr/local/bin/sync.py

# 设置工作目录
WORKDIR /index

COPY ./extensions/rag/index/index.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

# 设置默认命令
ENTRYPOINT ["/entrypoint.sh"]
