#!/bin/bash
# Script to create RustFS bucket for Mega demo
# This can be run manually or as part of initialization

ENDPOINT_URL="${OBJECT_STORAGE_S3__ENDPOINT_URL:-http://rustfs:9000}"
ACCESS_KEY="${S3_ACCESS_KEY_ID:-rustfsadmin}"
SECRET_KEY="${S3_SECRET_ACCESS_KEY:-rustfsadmin}"
BUCKET_NAME="${OBJECT_STORAGE_S3__BUCKET:-mega}"
REGION="${OBJECT_STORAGE_S3__REGION:-us-east-1}"

echo "Creating bucket '${BUCKET_NAME}' in RustFS at ${ENDPOINT_URL}..."

# Try using AWS CLI if available
if command -v aws >/dev/null 2>&1; then
    AWS_ACCESS_KEY_ID="${ACCESS_KEY}" \
    AWS_SECRET_ACCESS_KEY="${SECRET_KEY}" \
    AWS_DEFAULT_REGION="${REGION}" \
    aws --endpoint-url="${ENDPOINT_URL}" s3 mb "s3://${BUCKET_NAME}" 2>&1
    exit $?
else
    echo "AWS CLI not found. Please install AWS CLI or create the bucket manually:"
    echo "  aws --endpoint-url=${ENDPOINT_URL} s3 mb s3://${BUCKET_NAME} \\"
    echo "    --profile rustfs"
    echo ""
    echo "Or use the RustFS console at http://localhost:9001"
    exit 1
fi

