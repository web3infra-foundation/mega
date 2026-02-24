#!/bin/bash
set -e

# Ensure we are in the project root
# This moves from scripts/ to the parent directory
cd "$(dirname "$0")/.."

echo "Building orion and scorpio in release mode..."
# Build both packages
cargo build --release -p orion -p scorpio

echo "Ensuring remote directory exists..."
ssh orion "mkdir -p /root/orion-runner"

echo "Transferring binaries to orion:/root/orion-runner..."
scp target/release/orion target/release/scorpio orion:/root/orion-runner/

echo "Deployment complete!"
