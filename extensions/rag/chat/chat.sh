#!/bin/bash

# 追加到现有 no_proxy 列表中，避免覆盖已有值
export no_proxy="${no_proxy:+$no_proxy,}mono-engine,ollama,qdrant"
export NO_PROXY="${NO_PROXY:+$NO_PROXY,}mono-engine,ollama,qdrant"

echo "[entrypoint] no_proxy=$no_proxy"
echo "[entrypoint] NO_PROXY=$NO_PROXY"
echo "[entrypoint] http_proxy=$http_proxy"
echo "[entrypoint] https_proxy=$https_proxy"

# 启动主服务
exec /usr/local/bin/chat chat
