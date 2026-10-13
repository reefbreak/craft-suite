#!/bin/bash
# Pulls each app's upstream into its folder (git subtree, squashed).
#   scripts/sync-upstream.sh            every app
#   scripts/sync-upstream.sh filmcraft  one app
# Gesso comes from the reefbreak/artcraft fork (branch local-providers): ArtCraft renamed Gesso,
# with the local-mode changes. Sync that fork with storytold/artcraft first, then run this.
set -euo pipefail
cd "$(dirname "$0")/.."

declare -A SOURCES=(
  [gesso]="https://github.com/reefbreak/artcraft local-providers"
  [filmcraft]="https://github.com/storytold/filmcraft main"
  [photocraft]="https://github.com/storytold/photocraft main"
  [vectorcraft]="https://github.com/storytold/vectorcraft main"
  [lightcraft]="https://github.com/storytold/lightcraft main"
  [printcraft]="https://github.com/storytold/pdfcraft main"
  [effectcraft]="https://github.com/storytold/effectcraft main"
  [designcraft]="https://github.com/storytold/designcraft main"
  [deckcraft]="https://github.com/storytold/deckcraft main"
  [wordcraft]="https://github.com/storytold/wordcraft main"
  [gridcraft]="https://github.com/storytold/gridcraft main"
  [cadcraft]="https://github.com/storytold/cadcraft main"
  [craft-launcher]="https://github.com/storytold/craft-launcher main"
)

if [ -n "$(git status --porcelain)" ]; then
  echo "Commit or stash your changes first." >&2
  exit 1
fi

apps=("$@")
[ ${#apps[@]} -eq 0 ] && apps=(gesso filmcraft photocraft vectorcraft lightcraft printcraft effectcraft designcraft deckcraft wordcraft gridcraft cadcraft craft-launcher)
for app in "${apps[@]}"; do
  read -r url branch <<< "${SOURCES[$app]:?unknown app: $app}"
  echo "== $app <- $url $branch"
  git subtree pull --prefix "$app" "$url" "$branch" --squash -m "Sync $app from upstream"
done
