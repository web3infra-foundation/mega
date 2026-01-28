#!/bin/bash
# Wrapper script to fix config.toml before starting mono
# This script modifies the S3 endpoint URL from localhost to rustfs
# and sets the access credentials for RustFS
# It also creates the required bucket in RustFS if it doesn't exist

CONFIG_FILE="/opt/mega/etc/config.toml"

# Get credentials from environment variables or use defaults
ACCESS_KEY="${S3_ACCESS_KEY_ID:-${OBJECT_STORAGE_S3__ACCESS_KEY_ID:-rustfsadmin}}"
SECRET_KEY="${S3_SECRET_ACCESS_KEY:-${OBJECT_STORAGE_S3__SECRET_ACCESS_KEY:-rustfsadmin}}"
BUCKET_NAME="${OBJECT_STORAGE_S3__BUCKET:-mega}"
ENDPOINT_URL="${OBJECT_STORAGE_S3__ENDPOINT_URL:-http://rustfs:9000}"

# Wait for RustFS to be ready (with timeout)
echo "Waiting for RustFS to be ready..."
for i in {1..30}; do
    if curl -s -f "${ENDPOINT_URL}/health" > /dev/null 2>&1; then
        echo "RustFS is ready"
        break
    fi
    if [ $i -eq 30 ]; then
        echo "Warning: RustFS may not be ready, continuing anyway..."
    fi
    sleep 1
done

# Create bucket in RustFS if it doesn't exist
echo "Checking if bucket '${BUCKET_NAME}' exists in RustFS..."
# Try to create bucket using AWS CLI if available (most reliable method)
if command -v aws >/dev/null 2>&1; then
    echo "Using AWS CLI to create bucket..."
    AWS_ACCESS_KEY_ID="${ACCESS_KEY}" \
    AWS_SECRET_ACCESS_KEY="${SECRET_KEY}" \
    AWS_DEFAULT_REGION="${OBJECT_STORAGE_S3__REGION:-us-east-1}" \
    aws --endpoint-url="${ENDPOINT_URL}" s3 mb "s3://${BUCKET_NAME}" 2>&1
    if [ $? -eq 0 ]; then
        echo "Bucket '${BUCKET_NAME}' created successfully"
    else
        echo "Bucket '${BUCKET_NAME}' may already exist or creation failed (this is OK if bucket exists)"
    fi
else
    echo "AWS CLI not available in container. Bucket '${BUCKET_NAME}' needs to be created manually."
    echo ""
    echo "To create the bucket, run one of the following:"
    echo "  1. From host machine (if AWS CLI installed):"
    echo "     AWS_ACCESS_KEY_ID=${ACCESS_KEY} AWS_SECRET_ACCESS_KEY=${SECRET_KEY} \\"
    echo "     aws --endpoint-url=${ENDPOINT_URL} s3 mb s3://${BUCKET_NAME}"
    echo ""
    echo "  2. Via RustFS console: http://localhost:9001"
    echo ""
    echo "  3. Using a temporary container:"
    echo "     docker run --rm --network mega-demo-network \\"
    echo "       amazon/aws-cli:latest s3 mb s3://${BUCKET_NAME} \\"
    echo "       --endpoint-url ${ENDPOINT_URL} \\"
    echo "       --profile default <<< \"[default]\\naws_access_key_id = ${ACCESS_KEY}\\naws_secret_access_key = ${SECRET_KEY}\""
    echo ""
    echo "Continuing startup (application will fail if bucket doesn't exist)..."
fi

# Fix config.toml if it exists
if [ -f "$CONFIG_FILE" ]; then
    # Replace localhost:9000 with rustfs:9000 in all sections
    sed -i 's|endpoint_url = "http://localhost:9000"|endpoint_url = "http://rustfs:9000"|g' "$CONFIG_FILE" || true
    sed -i 's|endpoint_url = "http://127.0.0.1:9000"|endpoint_url = "http://rustfs:9000"|g' "$CONFIG_FILE" || true
    
    # Fix access_key_id if it's empty (in [object_storage.s3] section)
    sed -i '/\[object_storage\.s3\]/,/^\[/ {
        s|^access_key_id = ""|access_key_id = "'"${ACCESS_KEY}"'"|
    }' "$CONFIG_FILE" || true
    
    # Fix secret_access_key if it's empty (in [object_storage.s3] section)
    sed -i '/\[object_storage\.s3\]/,/^\[/ {
        s|^secret_access_key = ""|secret_access_key = "'"${SECRET_KEY}"'"|
    }' "$CONFIG_FILE" || true
    
    # Fix in [lfs.s3] section if it exists
    sed -i '/\[lfs\.s3\]/,/^\[/ {
        s|endpoint_url = "http://localhost:9000"|endpoint_url = "http://rustfs:9000"|g
        s|endpoint_url = "http://127.0.0.1:9000"|endpoint_url = "http://rustfs:9000"|g
        s|^access_key_id = ""|access_key_id = "'"${ACCESS_KEY}"'"|
        s|^secret_access_key = ""|secret_access_key = "'"${SECRET_KEY}"'"|
    }' "$CONFIG_FILE" || true
    
    echo "Fixed S3 configuration in $CONFIG_FILE (endpoint: rustfs:9000, access_key: ${ACCESS_KEY}, bucket: ${BUCKET_NAME})"
fi

# Execute the original start script
exec /usr/local/bin/start-mono.sh

