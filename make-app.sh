#!/bin/bash
# Builds ClipStack and wraps it in a .app bundle at build/ClipStack.app.
# Uses swiftc directly so it only needs the Xcode Command Line Tools.
# Usage:  ./make-app.sh            build only
#         ./make-app.sh --install  build, copy to /Applications, and launch
set -euo pipefail
cd "$(dirname "$0")"

ARCH="$(uname -m)"                       # arm64 or x86_64
APP="build/ClipStack.app"

mkdir -p build
swiftc -O -swift-version 5 \
    -target "${ARCH}-apple-macosx12.0" \
    -framework Cocoa -framework Carbon -framework ApplicationServices \
    -o build/ClipStack \
    Sources/ClipStack/*.swift

rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
mv build/ClipStack "$APP/Contents/MacOS/ClipStack"
cp Resources/Info.plist "$APP/Contents/Info.plist"

# Sign with a real certificate when one is available. macOS ties the Accessibility
# grant to the signing identity: with an ad-hoc signature that identity is the hash
# of the binary, so every rebuild would look like a new app and lose the grant.
# Override with:  SIGN_IDENTITY="Apple Development: Name (TEAMID)" ./make-app.sh
if [[ -z "${SIGN_IDENTITY:-}" ]]; then
    IDENTITIES="$(security find-identity -v -p codesigning 2>/dev/null)"
    for kind in "Developer ID Application" "Apple Development" "Apple Distribution"; do
        SIGN_IDENTITY="$(grep -oE "\"${kind}[^\"]*\"" <<<"$IDENTITIES" | head -1 | tr -d '"')"
        [[ -n "$SIGN_IDENTITY" ]] && break
    done
fi
if [[ -n "$SIGN_IDENTITY" ]]; then
    codesign --force --options runtime --sign "$SIGN_IDENTITY" "$APP"
    echo "Signed with: $SIGN_IDENTITY"
else
    codesign --force --sign - "$APP" >/dev/null 2>&1 || true
    echo "Signed ad-hoc (no certificate found; Accessibility must be re-granted after each rebuild)"
fi

echo "Built $APP"

if [[ "${1:-}" == "--install" ]]; then
    pkill -x ClipStack 2>/dev/null || true
    rm -rf "/Applications/ClipStack.app"
    cp -R "$APP" "/Applications/ClipStack.app"
    # LaunchServices occasionally refuses (-600) right after the old process was killed; retry once.
    open "/Applications/ClipStack.app" || { sleep 1; open "/Applications/ClipStack.app"; }
    echo "Installed and launched /Applications/ClipStack.app"
fi
