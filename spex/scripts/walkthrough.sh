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
note "exception is a tiny SQLite fixture database it creates under demos/ for"
note "the sql-schema example."

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

if command -v sqlite3 >/dev/null 2>&1; then
  note "A tiny SQLite database (customers/products/orders/order_items, a"
  note "few real rows each) introspected via the real sqlite3 CLI — table"
  note "row counts and foreign keys, not fabricated JSON."
  mkdir -p demos/sql-schema
  sqlite3 demos/sql-schema/shop.db <<'SQL'
CREATE TABLE IF NOT EXISTS customers (id INTEGER PRIMARY KEY, name TEXT, email TEXT);
CREATE TABLE IF NOT EXISTS products (id INTEGER PRIMARY KEY, name TEXT, price REAL);
CREATE TABLE IF NOT EXISTS orders (id INTEGER PRIMARY KEY, customer_id INTEGER, order_date TEXT,
  FOREIGN KEY(customer_id) REFERENCES customers(id));
CREATE TABLE IF NOT EXISTS order_items (id INTEGER PRIMARY KEY, order_id INTEGER, product_id INTEGER, quantity INTEGER,
  FOREIGN KEY(order_id) REFERENCES orders(id),
  FOREIGN KEY(product_id) REFERENCES products(id));
INSERT OR IGNORE INTO customers VALUES (1,'Alice','alice@example.com'),(2,'Bob','bob@example.com'),(3,'Carol','carol@example.com');
INSERT OR IGNORE INTO products VALUES (1,'Widget',9.99),(2,'Gadget',19.99),(3,'Gizmo',29.99),(4,'Thingamajig',4.99);
INSERT OR IGNORE INTO orders VALUES (1,1,'2026-01-02'),(2,2,'2026-01-05'),(3,1,'2026-02-10'),(4,3,'2026-03-01');
INSERT OR IGNORE INTO order_items VALUES
  (1,1,1,2),(2,1,2,1),(3,2,3,1),(4,3,1,5),(5,3,2,2),(6,4,4,3),(7,4,1,1),(8,4,2,2),(9,4,3,1);
SQL
  capture sql-schema "$BIN" sql-schema demos/sql-schema/shop.db
else
  note "no sqlite3 found — skipping the SQL schema example"
fi

say "4. What you've got"
"$BIN" demos

say "Next: pick one and run \`spex serve <tileset dir>\` — drag to orbit, scroll"
note "to zoom, hover a blob for its label. \`spex graph-print <graph.json>\`"
note "gives you the same tree as plain text in the terminal, no browser needed."
