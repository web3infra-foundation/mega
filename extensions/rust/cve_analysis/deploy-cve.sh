#!/bin/bash
set -euxo pipefail

# source
CODE_DIR=$(pwd)
CRATESPRO_DIR=/home/rust/crates-pro
INFRA_DIR=/home/rust/workspace/crates-pro-infra
MEGA_DIR=/home/rust/lhw/mega
# deployment
NAMESPACE=crates-pro
DEPLOYMENT=cve
KAFKA_HOST=10.42.0.1:30092
KAFKA_CONSUMER_GROUP=lhwtest9
# build
BUILD_DIR=$(mktemp -d)
TIMESTAMP=$(date +%Y%m%d-%H%M)
IMAGE_NAME=localhost:30500/crates-pro-cve:local-$TIMESTAMP

### Preparation: Sync source directories
rsync --delete --archive $CRATESPRO_DIR/ $INFRA_DIR/project/crates-pro/ --exclude="/.git" --exclude="/buck-out" --exclude="/build" --exclude="/target"
rsync --delete --archive $CODE_DIR/ $INFRA_DIR/project/crates-pro/cve/ --exclude="/log" --exclude="/target"
rsync --delete --archive $MEGA_DIR/extensions/cratespro/common/data_transporter/ $INFRA_DIR/project/crates-pro/data_transporter/
### Step 1: Compile, then copy artifacts to $BUILD_DIR
cd $INFRA_DIR
buck2 build //project/crates-pro/cve:cve --out $BUILD_DIR/cve
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
while kubectl get pods -n $NAMESPACE -o name | grep "^pod/$DEPLOYMENT-" > /dev/null; do
    sleep 5
done

### Step 5: Set new images
kubectl set image deployment/$DEPLOYMENT -n $NAMESPACE container-0=$IMAGE_NAME

# Wait until all kafka consumers are removed
while docker run --rm -t bitnami/kafka -- kafka-consumer-groups.sh --bootstrap-server $KAFKA_HOST --group $KAFKA_CONSUMER_GROUP --describe | grep rdkafka > /dev/null; do
    sleep 5
done

### Step 6: Run new containers
# Scale deployment back to 1 replica
kubectl scale deployment $DEPLOYMENT -n $NAMESPACE --replicas=1
