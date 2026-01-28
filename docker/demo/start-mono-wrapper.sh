#!/bin/bash
# Wrapper script to fix config.toml before starting mono
# This script modifies the S3 endpoint URL from localhost to rustfs
# and sets the access credentials for RustFS

CONFIG_FILE="/opt/mega/etc/config.toml"

# Get endpoint & credentials from environment variables or use defaults
ENDPOINT_URL="${OBJECT_STORAGE_S3__ENDPOINT_URL:-http://rustfs:9000}"
ACCESS_KEY="${S3_ACCESS_KEY_ID:-rustfsadmin}"
SECRET_KEY="${S3_SECRET_ACCESS_KEY:-rustfsadmin}"
BUCKET_NAME="${OBJECT_STORAGE_S3__BUCKET:-mega}"

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

