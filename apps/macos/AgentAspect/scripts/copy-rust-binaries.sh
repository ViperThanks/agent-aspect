#!/usr/bin/env bash
# copy-rust-binaries.sh — Copy built Rust binaries into the app bundle Resources/Binaries/
#
# Prerequisite: run build-rust-binaries.sh first (or cargo build --release).
#
# Usage: ./scripts/copy-rust-binaries.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"
APP_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
DEST="$APP_DIR/Resources/Binaries"

BINARIES=(
    "agent-aspect"
    "agent-aspectd"
    "agent-aspect-bridge"
    "agent-aspect-hook"
)

echo "==> Copying binaries to $DEST/"
mkdir -p "$DEST"

for bin in "${BINARIES[@]}"; do
    src="$PROJECT_ROOT/target/release/$bin"
    if [[ -f "$src" ]]; then
        cp "$src" "$DEST/$bin"
        chmod +x "$DEST/$bin"
        echo "  Copied: $bin"
    else
        echo "  SKIP (not found): $src"
    fi
done

echo "==> Done. Contents of Resources/Binaries/:"
ls -la "$DEST/"
