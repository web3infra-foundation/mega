#!/bin/bash

# 追加到现有 no_proxy 列表中，避免覆盖已有值
export no_proxy="${no_proxy:+$no_proxy,}mono-engine,ollama,qdrant,.gitmega.nju"
export NO_PROXY="${NO_PROXY:+$NO_PROXY,}mono-engine,ollama,qdrant,.gitmega.nju"

export PYTHONPATH="/app/extensions:$PYTHONPATH"
echo "[entrypoint] PYTHONPATH=$PYTHONPATH"
python3 -m update.sync
