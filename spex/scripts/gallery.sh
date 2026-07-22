#!/usr/bin/env bash
# Regenerate the demos (via walkthrough.sh) and (re)launch the gallery server
# on one port. The gallery's demo list is fixed at process startup — if
# demos/ changes after `spex gallery` is already running (a re-run of
# walkthrough.sh, a new adapter, ...), the running server won't see the
# update. This kills any previous gallery on the same port first so you
# always get a fresh one.
set -uo pipefail

cd "$(dirname "$0")/.."
BIN=./target/release/spex
PORT="${1:-8080}"

say() { printf '\n\033[1m%s\033[0m\n' "$1"; }

if [ ! -x "$BIN" ]; then
  say "Building spex (cargo build --release)..."
  cargo build --release || { echo "build failed — fix that first."; exit 1; }
fi

say "Regenerating demos..."
./scripts/walkthrough.sh >/dev/null

existing_pid="$(lsof -nP -iTCP:"$PORT" -sTCP:LISTEN -t 2>/dev/null || true)"
if [ -n "$existing_pid" ]; then
  say "Stopping stale gallery server on port $PORT (pid $existing_pid)..."
  kill "$existing_pid" 2>/dev/null
  sleep 0.5
fi

say "Starting gallery on http://127.0.0.1:$PORT/"
exec "$BIN" gallery demos --port "$PORT"
