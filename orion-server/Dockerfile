# ────── Stage 1: Build ──────
FROM rust:1.88 as builder

# 构建上下文中的父目录将作为 /app 挂载
WORKDIR /app

# 复制整个项目结构（从根目录开始）
COPY . .

# 进入子项目目录
WORKDIR /app/orion-server

# 构建 release 二进制
RUN cargo build --release

# ────── Stage 2: Runtime ──────
FROM debian:bookworm-slim

# 安装运行依赖
RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates && \
    rm -rf /var/lib/apt/lists/*

ENV BUILD_LOG_DIR="/tmp/buck2ctl"
RUN mkdir -p ${BUILD_LOG_DIR}

ENV PORT=8004

# 拷贝构建好的可执行文件
COPY --from=builder /app/target/release/orion-server /usr/local/bin/app

# 减少体积（可选）
RUN strip /usr/local/bin/app || true

EXPOSE 8004

CMD ["app"]