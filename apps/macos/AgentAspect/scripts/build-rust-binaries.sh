#!/usr/bin/env bash
# build-rust-binaries.sh — Build all Agent Aspect Rust binaries in release mode
#
# Builds the 4 workspace binaries:
#   - agent-aspect (CLI)
#   - agent-aspectd (daemon)
#   - agent-aspect-bridge
#   - agent-aspect-hook
#
# Usage: ./scripts/build-rust-binaries.sh [--features <features>]

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"

echo "==> Building Rust workspace (release)..."
cd "$PROJECT_ROOT"
cargo build --release "$@"

echo "==> Done. Binaries in: target/release/"
ls -la target/release/agent-aspect target/release/agent-aspectd \
      target/release/agent-aspect-bridge target/release/agent-aspect-hook 2>/dev/null || {
    echo "WARNING: Some expected binaries not found. Check cargo output above."
    exit 1
}
