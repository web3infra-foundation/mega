#!/usr/bin/bash
set -e

# Change to the directory of the script, so that relative paths work
cd "$(dirname "${BASH_SOURCE[0]}")"

# You can treat the `third-party` as a regular Cargo project, for example, you can run `cargo build`.

# This will resolve the new dependencies (creating or updating Cargo.lock)
# Vendor all the new code in the /vendor directory (also deleting unused code)
reindeer --third-party-dir . vendor

# Generate BUCK files for third-party dependencies
#reindeer --third-party-dir . buckify

./buckify.sh