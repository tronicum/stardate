#!/usr/bin/env bash
# A guided tour of spex: generates a handful of real demos so you can see
# what the tool actually does without having to invent your own first target.
# Safe to re-run any time — everything it writes lives under demos/, which is
# gitignored and fully regenerable.
set -uo pipefail

cd "$(dirname "$0")/.."
BIN=./target/release/spex

say() { printf '\n\033[1m%s\033[0m\n' "$1"; }
note() { printf '  %s\n' "$1"; }

say "spex walkthrough"
note "This builds a few example trees and turns each into a tiny 3D point-cloud"
note "scene you can look at in a browser. Nothing here touches real files"
note "outside this repo except to *read* them (du/ps/brew/traceroute); the one"
note "exception is downloading the real Chinook sample database (~1MB, once)"
note "under demos/ for the sql-schema example."

if [ ! -x "$BIN" ]; then
  say "1. Building spex (cargo build --release)..."
  cargo build --release || { echo "build failed — fix that first."; exit 1; }
else
  say "1. spex is already built ($BIN)"
fi

mkdir -p demos

capture() {
  local name="$1"; shift
  say "-> $name"
  mkdir -p "demos/$name"
  if "$@" -o "demos/$name/graph.json"; then
    "$BIN" graph-layout "demos/$name/graph.json" -o "demos/$name/tileset" >/dev/null
    note "ready: spex serve demos/$name/tileset"
  else
    note "skipped (capture failed — see message above)"
  fi
}

say "2. Always-available examples (no external tools required)"

note "A fabricated minimal-boot process tree — just to see the shape of things."
capture pstree "$BIN" pstree-demo

note "A tiny simulated packet journey Berlin -> Tegernsee -> Neuss (real"
note "haversine distances; latency is illustrative, not measured)."
mkdir -p demos/berlin-tegernsee-neuss
cat > demos/berlin-tegernsee-neuss/graph.json <<'JSON'
{
  "title": "simulated packet journey: Berlin -> Tegernsee -> Neuss",
  "metric_label": "simulated one-way latency (ms) - illustrative, not measured",
  "nodes": [
    {
      "id": "berlin",
      "label": "Berlin",
      "parent": null,
      "metric": null,
      "metadata": { "lat": 52.52, "lon": 13.405, "note": "start of the journey" }
    },
    {
      "id": "tegernsee",
      "label": "Tegernsee",
      "parent": "berlin",
      "metric": 8.2,
      "metadata": { "lat": 47.7167, "lon": 11.75, "distanceKm": 546.9, "note": "real haversine distance from Berlin; latency is illustrative, not measured" }
    },
    {
      "id": "neuss",
      "label": "Neuss",
      "parent": "tegernsee",
      "metric": 9.6,
      "metadata": { "lat": 51.1985, "lon": 6.6956, "distanceKm": 532.1, "note": "real haversine distance from Tegernsee; latency is illustrative, not measured" }
    }
  ]
}
JSON
"$BIN" graph-layout demos/berlin-tegernsee-neuss/graph.json -o demos/berlin-tegernsee-neuss/tileset >/dev/null
note "ready: spex serve demos/berlin-tegernsee-neuss/tileset"

note "Real disk usage of this repo's demos/ folder — a genuine 'what's taking"
note "up space' tree, using the real du on this machine."
capture disk-usage "$BIN" disk-usage demos

say "3. Real examples, if the underlying tool is on this machine"

if command -v traceroute >/dev/null 2>&1; then
  note "A real traceroute — pick any host you like; using www.de-cix.net as a"
  note "reasonable default (a real internet exchange operator's website)."
  capture decix-trace "$BIN" trace www.de-cix.net
else
  note "no traceroute found — skipping the network-path example"
fi

if command -v ps >/dev/null 2>&1; then
  note "This script's own real process tree (small and concrete, instead of"
  note "the whole system's 1000+ processes — try \`--root <pid>\` yourself"
  note "with any pid, e.g. \`echo \$\$\` for your interactive shell)."
  capture my-shell "$BIN" ps-tree --root "$$"
else
  note "no ps found — skipping the process-tree example"
fi

if command -v brew >/dev/null 2>&1; then
  note "A real package's dependency tree via Homebrew."
  capture neovim-deps "$BIN" brew-deps neovim
else
  note "no Homebrew found — skipping the package-dependency example"
fi

if command -v python3 >/dev/null 2>&1; then
  note "A simulated packet journey Neuss -> Hamburg -> ... -> Tegernsee, with"
  note "synthetic router hops between each city pair (real haversine"
  note "distances; latency is illustrative, not measured)."
  say "-> traveling-salesman"
  mkdir -p demos/traveling-salesman
  if python3 scripts/gen_traveling_salesman.py demos/traveling-salesman/graph.json; then
    "$BIN" graph-layout demos/traveling-salesman/graph.json -o demos/traveling-salesman/tileset >/dev/null
    note "ready: spex serve demos/traveling-salesman/tileset"
  else
    note "skipped (generation failed — see message above)"
  fi
else
  note "no python3 found — skipping the traveling-salesman example"
fi

if command -v python3 >/dev/null 2>&1; then
  note "The real Big Mac Index for the United States (2000-present) — real"
  note "prices twice a year, published by The Economist"
  note "(github.com/TheEconomist/big-mac-data), one node per publication date."
  say "-> bigmac"
  mkdir -p demos/bigmac
  if python3 scripts/gen_bigmac_demo.py demos/bigmac/graph.json "United States"; then
    "$BIN" graph-layout demos/bigmac/graph.json -o demos/bigmac/tileset >/dev/null
    note "ready: spex serve demos/bigmac/tileset"
  else
    note "skipped (download/generation failed — see message above, needs internet on first run)"
  fi
else
  note "no python3 found — skipping the Big Mac Index example"
fi

if command -v python3 >/dev/null 2>&1; then
  note "Real weekly stock closes (Tesla) from a committed data snapshot —"
  note "see scripts/stock-data/ and scripts/gen_stock_demo.py."
  say "-> stock-tesla"
  mkdir -p demos/stock-tesla
  if python3 scripts/gen_stock_demo.py demos/stock-tesla/graph.json tsla; then
    "$BIN" graph-layout demos/stock-tesla/graph.json -o demos/stock-tesla/tileset >/dev/null
    note "ready: spex serve demos/stock-tesla/tileset"
  else
    note "skipped (generation failed — see message above)"
  fi
else
  note "no python3 found — skipping the stock-price example"
fi

CHINOOK_URL="https://github.com/lerocha/chinook-database/raw/master/ChinookDatabase/DataSources/Chinook_Sqlite.sqlite"
if command -v sqlite3 >/dev/null 2>&1 && command -v curl >/dev/null 2>&1; then
  mkdir -p demos/sql-schema
  if [ ! -f demos/sql-schema/chinook.sqlite ]; then
    note "Downloading the real Chinook sample database (a digital media"
    note "store: artists/albums/tracks/customers/invoices, ~1MB, MIT-licensed"
    note "(c) 2008-2024 Luis Rocha, https://github.com/lerocha/chinook-database)."
    curl -sL --max-time 30 -o demos/sql-schema/chinook.sqlite.tmp "$CHINOOK_URL" \
      && mv demos/sql-schema/chinook.sqlite.tmp demos/sql-schema/chinook.sqlite
  fi
  if [ -f demos/sql-schema/chinook.sqlite ]; then
    note "Introspected via the real sqlite3 CLI — real tables, real row"
    note "counts, real foreign keys (including a self-referential one,"
    note "Employee.ReportsTo, correctly excluded as a tree parent)."
    capture sql-schema "$BIN" sql-schema demos/sql-schema/chinook.sqlite
  else
    rm -f demos/sql-schema/chinook.sqlite.tmp
    note "couldn't download the Chinook database (offline?) — skipping the SQL schema example"
  fi
else
  note "no sqlite3/curl found — skipping the SQL schema example"
fi

say "4. What you've got"
"$BIN" demos

say "Next: pick one and run \`spex serve <tileset dir>\` — drag to orbit, scroll"
note "to zoom, hover a blob for its label. \`spex graph-print <graph.json>\`"
note "gives you the same tree as plain text in the terminal, no browser needed."
