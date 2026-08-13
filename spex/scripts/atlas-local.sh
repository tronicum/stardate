#!/usr/bin/env bash
# Build every Atlas site and open them all in one page.
#
#   ./scripts/atlas-local.sh            # build what is missing, then serve
#   ./scripts/atlas-local.sh --rebuild  # rebuild every site from its sheet
#
# Three steps per site, and each one is a real command you can run yourself:
#
#   gen-atlas.py            heritage/<slug>.json  ->  recipes/heritage/<slug>.json
#   spex build              the recipe            ->  a real .ldr
#   spex mesh-model         the .ldr              ->  a mesh bundle the viewer reads
#
# The bundles land in demos/atlas-<slug>/tileset, which is the shape
# `spex gallery` indexes. Skips a site whose bundle is already there, so the
# second run is instant.
set -euo pipefail
cd "$(dirname "$0")/.."

SPEX=./target/release/spex
[ -x "$SPEX" ] || { echo "build the workspace first:  cargo build --release"; exit 1; }

if [ "${1:-}" = "--rebuild" ]; then rm -rf demos/atlas-*; fi

echo "== Rezepte aus den Blättern =="
python3 scripts/ldraw/gen-atlas.py --strict

echo
echo "== Bauen =="
for recipe in recipes/heritage/*.json; do
  slug=$(basename "$recipe" .json)
  dir="demos/atlas-$slug"
  if [ -f "$dir/tileset/mesh.json" ]; then
    printf '%-16s uebersprungen (schon gebaut)\n' "$slug"
    continue
  fi
  mkdir -p "$dir"
  printf '%-16s ' "$slug"
  $SPEX build "$recipe" -o "$dir/model.ldr" | grep -oE '[0-9]+ placement\(s\)|zero Illegality' | tr '\n' ' '
  $SPEX mesh-model "$dir/model.ldr" -o "$dir/tileset" >/dev/null
  echo "-> $dir"
done

# Stonehenge is Tier A too and predates the sheet format — it is a recipe of
# its own and belongs in the same gallery.
if [ ! -f demos/atlas-stonehenge/tileset/mesh.json ] && [ -f recipes/stonehenge.json ]; then
  mkdir -p demos/atlas-stonehenge
  printf '%-16s ' stonehenge
  $SPEX build recipes/stonehenge.json -o demos/atlas-stonehenge/model.ldr | grep -oE '[0-9]+ placement\(s\)' | tr '\n' ' '
  $SPEX mesh-model demos/atlas-stonehenge/model.ldr -o demos/atlas-stonehenge/tileset >/dev/null
  echo "-> demos/atlas-stonehenge"
fi

echo
$SPEX demos demos | head -40
echo
echo "== Galerie =="
exec $SPEX gallery demos
