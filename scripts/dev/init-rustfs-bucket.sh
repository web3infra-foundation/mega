#!/bin/sh

# Initialize RustFS bucket for the demo using S3-compatible API.
# This script is designed to run inside a lightweight Alpine-based image.

set -eu

# Environment variables:
#   BUCKET     - bucket name to create (e.g. "mega")
#   ENDPOINT   - RustFS endpoint URL (e.g. "http://rustfs:9000")
#   ACCESS_KEY - S3 access key ID for RustFS
#   SECRET_KEY - S3 secret access key for RustFS

BUCKET="${BUCKET:-mega}"
ENDPOINT="${ENDPOINT:-http://rustfs:9000}"
ACCESS_KEY="${ACCESS_KEY:-rustfsadmin}"
SECRET_KEY="${SECRET_KEY:-rustfsadmin}"

echo "Installing curl and openssl if missing..."
if ! command -v curl >/dev/null 2>&1 || ! command -v openssl >/dev/null 2>&1; then
  # Alpine-based image: install curl and openssl
  if command -v apk >/dev/null 2>&1; then
    apk add --no-cache curl openssl >/dev/null 2>&1
  else
    echo "Error: curl or openssl is not available and apk is not present to install them." >&2
    exit 1
  fi
fi

echo "Waiting for RustFS to be healthy at ${ENDPOINT}..."
for i in $(seq 1 60); do
  if curl -s -f "${ENDPOINT}/health" >/dev/null 2>&1; then
    echo "RustFS is healthy."
    break
  fi

  if [ "${i}" -eq 60 ]; then
    echo "RustFS did not become healthy in time." >&2
    exit 1
  fi

  sleep 1
done

BUCKET_PATH="/${BUCKET}"
DATE_HEADER="$(date -u +'%a, %d %b %Y %H:%M:%S GMT')"

# Build an S3 Signature V2 style string to sign for PUT Bucket
STRING_TO_SIGN="PUT\n\n\n${DATE_HEADER}\n${BUCKET_PATH}"
SIGNATURE="$(printf "%b" "${STRING_TO_SIGN}" | openssl sha1 -hmac "${SECRET_KEY}" -binary | base64)"

echo "Creating bucket '${BUCKET}' at ${ENDPOINT}..."
for i in $(seq 1 10); do
  HTTP_CODE="$(
    curl -s -o /dev/null -w "%{http_code}" \
      -X PUT "${ENDPOINT}${BUCKET_PATH}" \
      -H "Date: ${DATE_HEADER}" \
      -H "Authorization: AWS ${ACCESS_KEY}:${SIGNATURE}" \
      || echo "000"
  )"

  # 200/201: created; 409: already exists / conflict
  if [ "${HTTP_CODE}" = "200" ] || [ "${HTTP_CODE}" = "201" ] || [ "${HTTP_CODE}" = "409" ]; then
    echo "Bucket init finished (HTTP ${HTTP_CODE})."
    exit 0
  fi

  echo "Bucket init attempt ${i}/10 returned HTTP ${HTTP_CODE}, retrying..." >&2
  sleep 1
done

echo "Bucket init failed after retries; last HTTP ${HTTP_CODE}." >&2
exit 1


