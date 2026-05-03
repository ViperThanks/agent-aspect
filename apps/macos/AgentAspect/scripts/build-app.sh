#!/usr/bin/env bash
# build-app.sh — Build the AgentAspect SwiftPM app and assemble a .app bundle
#
# Steps:
#   1. swift build -c release
#   2. Create AgentAspect.app bundle structure
#   3. Copy binary and resources
#
# Usage: ./scripts/build-app.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
APP_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

APP_NAME="AgentAspect"
BUILD_DIR="$APP_DIR/.build"
APP_BUNDLE="$APP_DIR/.build/$APP_NAME.app"

echo "==> Building $APP_NAME (swift build -c release)..."
cd "$APP_DIR"
swift build -c release

BINARY="$BUILD_DIR/release/$APP_NAME"
if [[ ! -f "$BINARY" ]]; then
    echo "ERROR: Binary not found at $BINARY"
    exit 1
fi

echo "==> Assembling $APP_NAME.app bundle..."
rm -rf "$APP_BUNDLE"
mkdir -p "$APP_BUNDLE/Contents/MacOS"
mkdir -p "$APP_BUNDLE/Contents/Resources/Binaries"

# Copy the executable
cp "$BINARY" "$APP_BUNDLE/Contents/MacOS/$APP_NAME"

# Copy bundled binaries if present
if ls "$APP_DIR/Resources/Binaries/"* 1>/dev/null 2>&1; then
    cp "$APP_DIR/Resources/Binaries/"* "$APP_BUNDLE/Contents/Resources/Binaries/"
    echo "  Copied bundled binaries"
fi

# Create Info.plist
cat > "$APP_BUNDLE/Contents/Info.plist" << 'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>AgentAspect</string>
    <key>CFBundleIdentifier</key>
    <string>com.agent-aspect.AgentAspect</string>
    <key>CFBundleName</key>
    <string>AgentAspect</string>
    <key>CFBundleDisplayName</key>
    <string>Agent Aspect</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>0.1.0</string>
    <key>CFBundleVersion</key>
    <string>1</string>
    <key>LSMinimumSystemVersion</key>
    <string>13.0</string>
    <key>NSPrincipalClass</key>
    <string>NSApplication</string>
</dict>
</plist>
PLIST

echo "==> Done. App bundle at: $APP_BUNDLE"
echo "    Run with: open $APP_BUNDLE"
