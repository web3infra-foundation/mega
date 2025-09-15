#!/bin/bash
set -euxo pipefail

# source
CURRENT_DIR=$(pwd)
MEGA_ROOT_DIR=$(dirname $(dirname "$CURRENT_DIR"))
# deployment
NAMESPACE=mega-rag
CRONJOB_NAME=rag-update-cronjob
CONTAINER_NAME=rag-update
# build
TIMESTAMP=$(date +%Y%m%d-%H%M)
IMAGE_NAME=localhost:30500/mega-rag-update:local-$TIMESTAMP

### Step 0: Verify we're in the correct directory
if [[ ! "$CURRENT_DIR" == *"/mega/extensions/update" ]]; then
    echo "Error: Must be run from .../mega/extensions/update directory"
    exit 1
fi

### Step 1: Build Docker image
docker build -t $IMAGE_NAME -f $CURRENT_DIR/Dockerfile $MEGA_ROOT_DIR

### Step 2: Push Docker image
docker push $IMAGE_NAME

### Step 3: Update cronjob image
kubectl set image cronjob/$CRONJOB_NAME -n $NAMESPACE $CONTAINER_NAME=$IMAGE_NAME

### Step 4: Stop all running jobs from this cronjob and related pods
# Get all jobs created by this cronjob
JOBS=$(kubectl get jobs -n $NAMESPACE -o json | jq -r ".items[] | select(.metadata.ownerReferences[]?.name==\"$CRONJOB_NAME\") | .metadata.name")

# Delete all running jobs
for job in $JOBS; do
    echo "Deleting job: $job"
    kubectl delete job $job -n $NAMESPACE --ignore-not-found=true
done

# Delete all pods with label app=rag-update
echo "Deleting pods with label app=rag-update..."
kubectl delete pods -n $NAMESPACE -l app=rag-update --ignore-not-found=true

# Wait until all jobs are terminated
while kubectl get jobs -n $NAMESPACE -o json | jq -r ".items[] | select(.metadata.ownerReferences[]?.name==\"$CRONJOB_NAME\") | .metadata.name" | grep . > /dev/null; do
    echo "Waiting for jobs to terminate..."
    sleep 5
done

# Wait until all pods with label app=rag-update are terminated
while kubectl get pods -n $NAMESPACE -l app=rag-update --no-headers 2>/dev/null | grep . > /dev/null; do
    echo "Waiting for pods to terminate..."
    sleep 5
done

### Step 5: Trigger a new job immediately
kubectl create job --from=cronjob/$CRONJOB_NAME -n $NAMESPACE manual-$(date +%Y%m%d-%H%M%S)

echo "Deployment completed successfully!"