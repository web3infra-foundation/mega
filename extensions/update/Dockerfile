# 选择轻量级 Python 基础镜像
FROM python:3.11-slim

# 安装系统依赖，包括 git 和 wget
RUN apt-get update && apt-get install -y --no-install-recommends \
    git \
    wget \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# 设置工作目录
WORKDIR /app

# 创建虚拟环境
RUN python3 -m venv /opt/venv

# 配置环境变量（以后所有 pip/python 都走 venv）
ENV PATH="/opt/venv/bin:$PATH"

# 安装 Python 依赖
COPY ./extensions/update/requirements.txt ./requirements.txt
RUN pip install --no-cache-dir -r requirements.txt

# 复制代码（只需要 update 模块和依赖的 extensions）
COPY ./extensions /app/extensions


COPY ./extensions/update/update.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

# 设置默认命令
ENTRYPOINT ["/entrypoint.sh"]
