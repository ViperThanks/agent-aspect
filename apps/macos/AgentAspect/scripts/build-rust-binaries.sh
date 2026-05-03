#!/usr/bin/env bash
# build-rust-binaries.sh — Build all checkpoint Rust binaries in release mode
#
# Builds the 4 workspace binaries:
#   - checkpoint (CLI)
#   - checkpointd (daemon)
#   - checkpoint-bridge
#   - checkpoint-hook-cli
#
# Usage: ./scripts/build-rust-binaries.sh [--features <features>]

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"

echo "==> Building Rust workspace (release)..."
cd "$PROJECT_ROOT"
cargo build --release "$@"

echo "==> Done. Binaries in: target/release/"
ls -la target/release/checkpoint target/release/checkpointd \
      target/release/checkpoint-bridge target/release/checkpoint-hook-cli 2>/dev/null || {
    echo "WARNING: Some expected binaries not found. Check cargo output above."
    exit 1
}
