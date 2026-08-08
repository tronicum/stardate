# spex-wasm PoC demo

A standalone, self-contained demo proving `spex-graph`'s real radial layout
algorithm runs client-side, compiled to WebAssembly — no `spex serve`, no
Rust process, no backend at all. Not wired into the real viewer (`viewer/`);
this is a proof-of-concept that the layout core itself is WASM-portable, not
a finished feature.

## Requirements

This project's default Rust toolchain (Homebrew's) doesn't ship the
`wasm32-unknown-unknown` target. You'll need a `rustup`-managed toolchain
for the build step (the rest of `spex` doesn't need this — it's local to
`spex-wasm`):

```sh
brew install rustup-init   # or `rustup`, depending on the current formula name
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.126   # match the wasm-bindgen crate version in Cargo.lock
```

## Build

From `spex/`:

```sh
cargo build -p spex-wasm --release --target wasm32-unknown-unknown
wasm-bindgen --target web \
  --out-dir crates/spex-wasm/poc-demo/pkg \
  target/wasm32-unknown-unknown/release/spex_wasm.wasm
```

## View

```sh
cd crates/spex-wasm/poc-demo
python3 -m http.server 8000
```

Open `http://localhost:8000`, paste a real `graph.json`, click "Layout
in-browser" — the layout runs entirely in the WASM module, in-page, no
network calls after the initial page/module load.
