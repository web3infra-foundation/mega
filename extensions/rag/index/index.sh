#!/bin/bash

# 追加到现有 no_proxy 列表中，避免覆盖已有值
export no_proxy="${no_proxy:+$no_proxy,}mono-engine,ollama,qdrant,.gitmega.nju"
export NO_PROXY="${NO_PROXY:+$NO_PROXY,}mono-engine,ollama,qdrant,.gitmega.nju"

echo "[entrypoint] no_proxy=$no_proxy"
echo "[entrypoint] NO_PROXY=$NO_PROXY"
echo "[entrypoint] http_proxy=$http_proxy"
echo "[entrypoint] https_proxy=$https_proxy"

export PYTHONPATH="/usr/local/bin/extensions:$PYTHONPATH"
python3 -m update.sync

# 启动主服务
exec /usr/local/bin/index
