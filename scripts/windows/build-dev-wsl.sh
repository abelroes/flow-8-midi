#!/usr/bin/env bash
# Full Windows BLE stack build from WSL (mirrors GHA windows_ble job).
#
# Layout (same as CI):
#   parent/flow-8-midi
#   parent/DirectHCI
#
# Uses one CARGO_TARGET_DIR so app, helper and DirectHCI EXEs land in the same
# folder — no manual copies between crates.
#
# Usage: scripts/windows/build-dev-wsl.sh [debug|release]
set -euo pipefail

PROFILE="${1:-debug}"
TARGET="x86_64-pc-windows-gnu"
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
DIRECTHCI="$(cd "$ROOT/../DirectHCI" && pwd)"
export CARGO_TARGET_DIR="$ROOT/target"

CARGO_FLAGS=(--target "$TARGET")
if [[ "$PROFILE" == "release" ]]; then
  CARGO_FLAGS+=(--release)
fi

OUT="$CARGO_TARGET_DIR/$TARGET/$PROFILE"

echo "CARGO_TARGET_DIR=$CARGO_TARGET_DIR"
echo "DirectHCI=$DIRECTHCI"

echo "==> 1/3 flow-8-midi"
(cd "$ROOT" && cargo build "${CARGO_FLAGS[@]}" --bin flow-8-midi)

echo "==> 2/3 flow8-ble-helper"
(cd "$ROOT/helper/flow8-ble-helper" && cargo build "${CARGO_FLAGS[@]}")

echo "==> 3/3 DirectHCI runtime"
(cd "$DIRECTHCI" && cargo build "${CARGO_FLAGS[@]}" \
  -p directhci \
  -p directhcid \
  -p directhci-control-panel \
  -p directhci-ble-cli)

for f in \
  flow-8-midi.exe \
  flow8-ble-helper.exe \
  directhci.exe \
  directhcid.exe \
  directhci-control-panel.exe \
  directhci-ble.exe
do
  if [[ ! -f "$OUT/$f" ]]; then
    echo "error: missing $OUT/$f" >&2
    exit 1
  fi
done

# Installer-oriented stage (single tree consumed by flow8-midi.iss)
STAGE="$ROOT/dist/windows/$PROFILE"
mkdir -p "$STAGE/app" "$STAGE/directhci"
cp -f "$OUT/flow-8-midi.exe" "$OUT/flow8-ble-helper.exe" "$STAGE/app/"
cp -f "$OUT/directhci.exe" "$OUT/directhcid.exe" \
  "$OUT/directhci-control-panel.exe" "$OUT/directhci-ble.exe" \
  "$STAGE/directhci/"

# Optional: stage libwdi from the Windows host if already built once
LIBWDI_STAGE="$STAGE/libwdi"
mkdir -p "$LIBWDI_STAGE"
for cand in \
  "${LOCALAPPDATA:-}/DirectHCI/native/libwdi.dll" \
  /mnt/c/Users/*/AppData/Local/DirectHCI/native/libwdi.dll
do
  if [[ -f "$cand" ]]; then
    cp -f "$cand" "$LIBWDI_STAGE/libwdi.dll"
    echo "Staged libwdi.dll from $cand"
    break
  fi
done
# Keep DLL next to EXEs so the elevated helper can install without hunting
if [[ -f "$LIBWDI_STAGE/libwdi.dll" ]]; then
  cp -f "$LIBWDI_STAGE/libwdi.dll" "$OUT/libwdi.dll"
fi

echo
echo "OK — Flow 8 + DirectHCI in one cargo output (sibling DirectHCI project):"
echo "  $OUT/"
ls -1 "$OUT"/*.exe | xargs -n1 basename
echo
echo "Stage: $STAGE/{app,directhci}/"
echo
echo "GUI Setup (Windows host, same as GitHub Actions):"
echo "  .\\scripts\\windows\\build-installer.ps1 -Profile $PROFILE -Target x86_64-pc-windows-gnu -SkipBuild -OpenOutput"
echo "  → FLOW8-MIDI-Setup-*.exe (needs Inno Setup 6 + libwdi once)"
