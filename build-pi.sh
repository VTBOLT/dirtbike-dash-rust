#!/usr/bin/env bash
# Cross-compiles dirtbike_dash for a 64-bit Raspberry Pi (aarch64) inside Docker.
# No cross-compilation tooling touches the host system - everything lives in the image.
#
# Usage:
#   ./build-pi.sh                              # default features
#   ./build-pi.sh "can gps sim soc b6 release"  # custom feature list
#
# Output: dirtbike_dash/target/aarch64-unknown-linux-gnu/release/test
set -euo pipefail

FEATURES="${1:-can gps sim soc b6 release}"
IMAGE_TAG="dirtbike-dash-aarch64-builder"
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CACHE_DIR="$REPO_ROOT/.docker-cache"

mkdir -p "$CACHE_DIR/registry" "$CACHE_DIR/git"

docker build -t "$IMAGE_TAG" -f "$REPO_ROOT/docker/aarch64/Dockerfile" "$REPO_ROOT/docker/aarch64"

docker run --rm \
    --user "$(id -u):$(id -g)" \
    -e HOME=/tmp \
    -v "$REPO_ROOT/dirtbike_dash:/work" \
    -v "$CACHE_DIR/registry:/usr/local/cargo/registry" \
    -v "$CACHE_DIR/git:/usr/local/cargo/git" \
    -w /work \
    "$IMAGE_TAG" \
    cargo build --release --target aarch64-unknown-linux-gnu --features "$FEATURES"

echo
echo "Built: dirtbike_dash/target/aarch64-unknown-linux-gnu/release/test"
