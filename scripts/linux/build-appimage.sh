#!/usr/bin/env bash
# Build a Linux x86_64 AppImage for end users (not for day-to-day debug).
# Usage: scripts/linux/build-appimage.sh
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
TARGET="${TARGET:-x86_64-unknown-linux-gnu}"
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' "$ROOT/Cargo.toml" | head -1)"
OUT_DIR="$ROOT/dist/linux"
APPDIR="$OUT_DIR/AppDir"
TOOLS="$OUT_DIR/tools"
ARCH="${ARCH:-x86_64}"

mkdir -p "$OUT_DIR" "$TOOLS"

echo "==> cargo build --release --target $TARGET"
(cd "$ROOT" && cargo build --release --target "$TARGET" --bin flow-8-midi)

BIN="$ROOT/target/$TARGET/release/flow-8-midi"
if [[ ! -x "$BIN" ]]; then
  # host native path when --target matches host
  BIN="$ROOT/target/release/flow-8-midi"
fi
test -x "$BIN"

echo "==> Assemble AppDir"
rm -rf "$APPDIR"
mkdir -p \
  "$APPDIR/usr/bin" \
  "$APPDIR/usr/share/applications" \
  "$APPDIR/usr/share/icons/hicolor/256x256/apps"

cp -f "$BIN" "$APPDIR/usr/bin/flow-8-midi"
cp -f "$ROOT/installer/linux/flow-8-midi.desktop" \
  "$APPDIR/usr/share/applications/flow-8-midi.desktop"

# Icon from resources/icon.ico (256×256 PNG layer)
convert "$ROOT/resources/icon.ico[0]" -resize 256x256 \
  "$APPDIR/usr/share/icons/hicolor/256x256/apps/flow-8-midi.png"
cp -f "$APPDIR/usr/share/icons/hicolor/256x256/apps/flow-8-midi.png" \
  "$APPDIR/flow-8-midi.png"

cat >"$APPDIR/AppRun" <<'EOF'
#!/usr/bin/env bash
HERE="$(dirname "$(readlink -f "$0")")"
exec "$HERE/usr/bin/flow-8-midi" "$@"
EOF
chmod +x "$APPDIR/AppRun"

download() {
  local url="$1" dest="$2"
  if [[ -f "$dest" ]]; then return 0; fi
  echo "==> download $(basename "$dest")"
  curl -fsSL -o "$dest" "$url"
  chmod +x "$dest"
}

LINUXDEPLOY="$TOOLS/linuxdeploy-${ARCH}.AppImage"
APPIMAGETOOL="$TOOLS/appimagetool-${ARCH}.AppImage"
download \
  "https://github.com/linuxdeploy/linuxdeploy/releases/download/continuous/linuxdeploy-${ARCH}.AppImage" \
  "$LINUXDEPLOY"
download \
  "https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-${ARCH}.AppImage" \
  "$APPIMAGETOOL"

# Extract tools if FUSE is unavailable (common in CI)
run_tool() {
  local tool="$1"; shift
  if "$tool" --appimage-extract-and-run "$@" 2>/dev/null; then
    return 0
  fi
  "$tool" "$@"
}

# WSL often puts Windows paths in PATH; linuxdeploy walks them and can abort.
# Do not append the original PATH — /mnt/c/... entries trigger filesystem_error.
export PATH="/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin:${HOME}/.cargo/bin"

echo "==> linuxdeploy"
(
  cd "$OUT_DIR"
  run_tool "$LINUXDEPLOY" \
    --appdir "$APPDIR" \
    --executable "$APPDIR/usr/bin/flow-8-midi" \
    --desktop-file "$APPDIR/usr/share/applications/flow-8-midi.desktop" \
    --icon-file "$APPDIR/usr/share/icons/hicolor/256x256/apps/flow-8-midi.png"
)

APPIMAGE_NAME="FLOW8-MIDI-${VERSION}-${ARCH}.AppImage"
echo "==> appimagetool → $APPIMAGE_NAME"
(
  cd "$OUT_DIR"
  ARCH="$ARCH" run_tool "$APPIMAGETOOL" "$APPDIR" "$OUT_DIR/$APPIMAGE_NAME"
)

echo "OK — end-user artifact:"
echo "  $OUT_DIR/$APPIMAGE_NAME"
echo "Users: chmod +x that file, then double-click or run it."
