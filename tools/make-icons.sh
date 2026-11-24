#!/bin/bash
# Regenerates every icon asset from assets/icon.svg and assets/icon-mono.svg.
# macOS only (uses AppKit to rasterise and iconutil for the .icns). Outputs land in assets/.
#
#   assets/AppIcon.icns            macOS app bundle icon
#   assets/icon.ico                Windows executable icon
#   assets/icon-<n>.png            colour icon at 16…1024 px (README, Linux tray)
#   assets/menubar-18.png / -36    monochrome template glyph for the macOS menubar (1x / 2x)
#   assets/tray-mono-<n>.png       monochrome glyph for the Rust tray on macOS
set -euo pipefail
cd "$(dirname "$0")/.."

OUT=assets
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

echo "• rasterising colour icon"
swift tools/render-svg.swift assets/icon.svg "$OUT" icon 16,32,48,64,128,256,512,1024 >/dev/null

echo "• rasterising mono glyph"
swift tools/render-svg.swift assets/icon-mono.svg "$TMP" mono 18,36,32,64 >/dev/null
cp "$TMP/mono-18.png" "$OUT/menubar-18.png"
cp "$TMP/mono-36.png" "$OUT/menubar-36.png"
cp "$TMP/mono-32.png" "$OUT/tray-mono-32.png"
cp "$TMP/mono-64.png" "$OUT/tray-mono-64.png"

echo "• building AppIcon.icns"
ICONSET="$TMP/AppIcon.iconset"
mkdir -p "$ICONSET"
cp "$OUT/icon-16.png"   "$ICONSET/icon_16x16.png"
cp "$OUT/icon-32.png"   "$ICONSET/icon_16x16@2x.png"
cp "$OUT/icon-32.png"   "$ICONSET/icon_32x32.png"
cp "$OUT/icon-64.png"   "$ICONSET/icon_32x32@2x.png"
cp "$OUT/icon-128.png"  "$ICONSET/icon_128x128.png"
cp "$OUT/icon-256.png"  "$ICONSET/icon_128x128@2x.png"
cp "$OUT/icon-256.png"  "$ICONSET/icon_256x256.png"
cp "$OUT/icon-512.png"  "$ICONSET/icon_256x256@2x.png"
cp "$OUT/icon-512.png"  "$ICONSET/icon_512x512.png"
cp "$OUT/icon-1024.png" "$ICONSET/icon_512x512@2x.png"
iconutil -c icns "$ICONSET" -o "$OUT/AppIcon.icns"

echo "• building icon.ico"
python3 - "$OUT" <<'EOF'
import struct, sys
out = sys.argv[1]
sizes = [16, 32, 48, 256]
images = [(s, open(f"{out}/icon-{s}.png", "rb").read()) for s in sizes]
header = struct.pack("<HHH", 0, 1, len(images))
offset = 6 + 16 * len(images)
entries, blobs = b"", b""
for s, data in images:
    dim = 0 if s >= 256 else s
    entries += struct.pack("<BBBBHHII", dim, dim, 0, 0, 1, 32, len(data), offset)
    offset += len(data)
    blobs += data
open(f"{out}/icon.ico", "wb").write(header + entries + blobs)
EOF

echo "done:"
ls -la "$OUT" | grep -E 'icns|ico$|png' | awk '{printf "  %-22s %7d bytes\n", $9, $5}'
