#!/bin/bash
# Pulls each app's upstream into its folder (git subtree, squashed).
#   scripts/sync-upstream.sh            every app
#   scripts/sync-upstream.sh filmcraft  one app
# ArtCraft comes from the reefbreak fork (branch local-providers), which carries the local-mode
# changes; sync that fork with storytold/artcraft first, then run this.
set -euo pipefail
cd "$(dirname "$0")/.."

declare -A SOURCES=(
  [artcraft]="https://github.com/reefbreak/artcraft local-providers"
  [filmcraft]="https://github.com/storytold/filmcraft main"
  [photocraft]="https://github.com/storytold/photocraft main"
  [vectorcraft]="https://github.com/storytold/vectorcraft main"
  [lightcraft]="https://github.com/storytold/lightcraft main"
  [printcraft]="https://github.com/storytold/printcraft main"
  [effectcraft]="https://github.com/storytold/effectcraft main"
  [designcraft]="https://github.com/storytold/designcraft main"
  [craft-launcher]="https://github.com/storytold/craft-launcher main"
)

if [ -n "$(git status --porcelain)" ]; then
  echo "Commit or stash your changes first." >&2
  exit 1
fi

apps=("$@")
[ ${#apps[@]} -eq 0 ] && apps=(artcraft filmcraft photocraft vectorcraft lightcraft printcraft effectcraft designcraft craft-launcher)
for app in "${apps[@]}"; do
  read -r url branch <<< "${SOURCES[$app]:?unknown app: $app}"
  echo "== $app <- $url $branch"
  git subtree pull --prefix "$app" "$url" "$branch" --squash -m "Sync $app from upstream"
done
