#!/bin/bash
set -euxo pipefail

# source
CODE_DIR=$(pwd)
INFRA_DIR=/home/rust/workspace/crates-pro-infra
# deployment
NAMESPACE=crates-pro
DEPLOYMENT=analysis-sensleak
KAFKA_HOST=172.17.0.1:30092
KAFKA_CONSUMER_GROUP=sensleak-group24
# build
BUILD_DIR=$(mktemp -d)
TIMESTAMP=$(date +%Y%m%d-%H%M)
IMAGE_NAME=localhost:30500/cratespro-analysis-sensleak:local-$TIMESTAMP

### Preparation: Sync source directories
rsync --delete --archive $CODE_DIR/ $INFRA_DIR/project/crates-pro/analysis_sensleak --exclude="/.git" --exclude="/bin" --exclude="/target"

### Step 1: Compile, then copy artifacts to $BUILD_DIR
cd $INFRA_DIR
mkdir -p $BUILD_DIR/bin
buck2 build //project/crates-pro/analysis_sensleak:analysis_sensleak --out $BUILD_DIR/analysis_sensleak
buck2 build //project/sensleak-rs:scan --out $BUILD_DIR/bin/scan
cp $CODE_DIR/analyzers.json $BUILD_DIR/analyzers.json
cp $CODE_DIR/gitleaks.toml $BUILD_DIR/gitleaks.toml
cp $CODE_DIR/.env $BUILD_DIR/.env
cd $CODE_DIR

### Step 2: Build Docker images
docker build -t $IMAGE_NAME -f $CODE_DIR/Dockerfile $BUILD_DIR

### Step 3: Push Docker images
docker push $IMAGE_NAME

### Step 4: Stop current containers
# Scale deployment to 0 replicas
kubectl scale deployment $DEPLOYMENT -n $NAMESPACE --replicas=0

# Wait until all pods are terminated
while kubectl get pods -n $NAMESPACE | grep $DEPLOYMENT > /dev/null; do
    sleep 5
done

### Step 5: Set new images
kubectl set image deployment/$DEPLOYMENT -n $NAMESPACE container-0=$IMAGE_NAME

# # Wait until all kafka consumers are removed
# while docker run --rm -t bitnami/kafka -- kafka-consumer-groups.sh --bootstrap-server $KAFKA_HOST --group $KAFKA_CONSUMER_GROUP --describe | grep rdkafka > /dev/null; do
#     sleep 5
# done

### Step 6: Run new containers
# Scale deployment back to 1 replica
kubectl scale deployment $DEPLOYMENT -n $NAMESPACE --replicas=1
