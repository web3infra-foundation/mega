#!/bin/bash
set -euxo pipefail

MEGA_DIR=/home/rust/xiyueli/mega
CURRENT_DIR=$(pwd)
NAMESPACE=mega-rag
DEPLOYMENT=rag-index
TIMESTAMP=$(date +%Y%m%d-%H%M)
IMAGE_TAG=localhost:30500/mega-rag-index:local-${TIMESTAMP}

docker build -t ${IMAGE_TAG} -f ${CURRENT_DIR}/Dockerfile ${MEGA_DIR}
docker push ${IMAGE_TAG}

kubectl set image deployment/${DEPLOYMENT} -n ${NAMESPACE} rag-index=${IMAGE_TAG}
