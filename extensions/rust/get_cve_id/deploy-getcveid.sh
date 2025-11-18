#!/bin/bash
set -euxo pipefail

# source
CODE_DIR=$(pwd)
CRATESPRO_DIR=/home/rust/crates-pro
INFRA_DIR=/home/rust/workspace/crates-pro-infra
MEGA_DIR=/home/rust/lhw/mega
# deployment
NAMESPACE=crates-pro
CRONJOB=getcveid
# build
BUILD_DIR=$(mktemp -d)
TIMESTAMP=$(date +%Y%m%d-%H%M)
IMAGE_NAME=localhost:30500/crates-pro-getcveid:local-$TIMESTAMP

### Preparation: Sync source directories
rsync --delete --archive $CRATESPRO_DIR/ $INFRA_DIR/project/crates-pro/ --exclude="/.git" --exclude="/buck-out" --exclude="/build" --exclude="/target"
rsync --delete --archive $CODE_DIR/ $INFRA_DIR/project/crates-pro/getcveid/ --exclude="/log" --exclude="/target"

### Step 1: Compile, then copy artifacts to $BUILD_DIR
cd $INFRA_DIR
buck2 build //project/crates-pro/getcveid:getcveid --out $BUILD_DIR/getcveid
cp $CODE_DIR/.env $BUILD_DIR/.env
cd $CODE_DIR

### Step 2: Build Docker image
docker build -t $IMAGE_NAME -f $CODE_DIR/Dockerfile $BUILD_DIR

### Step 3: Push Docker image
docker push $IMAGE_NAME

### Step 4: Update CronJob image
kubectl set image cronjob/$CRONJOB -n $NAMESPACE container-0=$IMAGE_NAME

### Step 5: Manually trigger the CronJob once
kubectl create job $CRONJOB-manual-$(date +%s) --from=cronjob/$CRONJOB -n $NAMESPACE

echo "CronJob updated with image: $IMAGE_NAME"
echo "Manual job triggered"
