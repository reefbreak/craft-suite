#!/usr/bin/env bash
# Regenerate every app and tray icon from the Gesso mark in assets/app-icon/ (copied from Gesso's
# gesso/assets/brand/; original artwork for Gesso, not an ArtCraft mark: assets/app-icon/LICENSE.txt).
#
# - App icon (Dock, taskbar, Start menu, Linux menus, installers): gesso-launcher.svg, a copy of
#   Gesso's app icon (gesso/assets/brand/gesso.svg).
# - Tray: gesso-mark-black.svg (the mark alone, black on transparent) as the macOS menu-bar
#   template; the app icon on Windows, Linux and BSD.
#
# Needs: resvg (brew install resvg / cargo install resvg). The .ico and .icns are packed by
# `cargo xtask ico` / `cargo xtask icns`, so this runs on any OS. The outputs are committed, so
# packaging (and the Rust build, which include_bytes!s some of them) never needs these tools.
#
#   packaging/icons.sh
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DIR="$ROOT/assets/app-icon"
SVG="$DIR/gesso-launcher.svg"
TEMPLATE="$DIR/gesso-mark-black.svg"
ID="us.reefbreak.gesso-launcher"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

command -v resvg >/dev/null || { echo "error: resvg not found (brew install resvg)" >&2; exit 1; }

# The artwork is untouched. macOS icons get Apple's transparent margin (the 824/1024 icon grid) by
# widening the view box around the 480-unit rounded square; everything else uses the file as is.
BOX='viewBox="0 0 512 512"'
grep -q "$BOX" "$SVG" || { echo "error: expected $BOX in $SVG" >&2; exit 1; }
MAC="$TMP/mac.svg"
sed "s/$BOX/viewBox=\"-42.25 -42.25 596.5 596.5\"/" "$SVG" >"$MAC"
FLAT="$SVG"

render() {
  local src="$1" size="$2" out="$3"
  resvg -w "$size" -h "$size" "$src" "$out" </dev/null
}

render "$MAC" 1024 "$DIR/gesso-launcher-1024.png"

# Linux hicolor theme.
for s in 16 24 32 48 64 128 256 512; do
  mkdir -p "$DIR/hicolor/${s}x${s}/apps"
  render "$FLAT" "$s" "$DIR/hicolor/${s}x${s}/apps/$ID.png"
done
mkdir -p "$DIR/hicolor/scalable/apps"
cp "$SVG" "$DIR/hicolor/scalable/apps/$ID.svg"

# Windows .ico.
ICO_PNGS=()
for s in 16 20 24 32 40 48 64 128 256; do
  render "$FLAT" "$s" "$TMP/ico-$s.png"
  ICO_PNGS+=("$TMP/ico-$s.png")
done
(cd "$ROOT" && cargo run -q -p xtask -- ico "$DIR/gesso-launcher.ico" "${ICO_PNGS[@]}")

# macOS .icns.
ICNS_PNGS=()
for s in 16 32 64 128 256 512 1024; do
  render "$MAC" "$s" "$TMP/icns-$s.png"
  ICNS_PNGS+=("$TMP/icns-$s.png")
done
(cd "$ROOT" && cargo run -q -p xtask -- icns "$DIR/gesso-launcher.icns" "${ICNS_PNGS[@]}")

# Tray icons: the black mark is the macOS menu-bar template (44 px = 22 pt @2x; macOS tints it);
# the app icon is for the Windows notification area and Linux/BSD status notifiers.
render "$TEMPLATE" 44 "$DIR/tray-template-44.png"
render "$FLAT" 64 "$DIR/tray-64.png"
echo "icons written to $DIR"
