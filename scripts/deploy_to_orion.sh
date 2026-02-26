#!/bin/bash
set -e

# Ensure we are in the project root
# This moves from scripts/ to the parent directory
cd "$(dirname "$0")/.."

echo "Building orion in release mode..."
# Note: scorpio has been moved to https://github.com/web3infra-foundation/scorpiofs
cargo build --release -p orion

echo "Ensuring remote directory exists..."
ssh orion "mkdir -p /root/orion-runner"

echo "Transferring binary to orion:/root/orion-runner..."
scp target/release/orion orion:/root/orion-runner/

echo "Deployment complete!"
