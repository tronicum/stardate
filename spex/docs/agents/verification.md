# Verification

`cargo test` proves the code is internally consistent. It does **not** prove
a browser feature actually renders, that a CLI command's output looks
sensible to a human, or that a real external file/service really behaves
the way a unit test's synthetic fixture assumes. This project's standard
practice — used before essentially every commit — is a ladder that gets
more expensive (and more real) as it goes:

1. `cargo build --release` — must compile clean.
2. `cargo test --workspace` — must be fully green. If a *pre-existing* test
   fails and you didn't touch that code, investigate before assuming it's
   unrelated — check `git log -- <file>` to see if it's actually new/edited
   nearby, and whether the failure is deterministic (rerun it) or genuinely
   flaky.
3. **A real functional CLI check** — actually run the built binary against
   real input and read the output: `spex graph-print`, `spex ascii`, `spex
   info`, whatever the change touches. This is the cheapest real check and
   catches a surprising number of things unit tests don't (e.g. a
   projection that compiles and passes narrow unit tests but produces a
   blank or nonsensical render on real data).
4. **For anything the graph pipeline touches**: run it through
   `graph-layout` and confirm the tileset builds, then `spex ascii` on the
   result for a zero-browser-needed visual sanity check.
5. **For anything viewer-visible** (a new checkbox, a color change, an
   animation): a real headless-Chromium/Playwright session — see
   "Delegate this to a fork" below. Screenshot it; look at the screenshot.
   A blank frame is a failure to launch, not a passing check.
6. `./scripts/walkthrough.sh` — regenerate every demo from scratch and
   confirm nothing broke across the whole set, not just the one thing you
   changed.
7. `cargo test --workspace` again, as a final gate before committing.

## The viewer-rebuild-order gotcha

`viewer/` (TypeScript/three.js) is embedded into the `spex` binary *at Rust
compile time* via `rust-embed`, not read at runtime. After any
`viewer/src/*.ts` change:

```sh
cd viewer && npm run build   # produces viewer/dist
cd ..       && cargo build --release   # re-embeds viewer/dist into the binary
```

Skipping the second step is a real, repeated source of "I changed the
viewer but nothing's different" confusion — the running binary is still
serving the *old* embedded bundle.

## Delegate real browser/long-running verification to a fork

A headless-browser session (screenshots, console-error checks, DOM queries)
or a long real-data fetch produces a lot of tool output that's only useful
once — reading it back into a coordinator's context wastes budget for no
benefit once you have the verdict. Spawn a fork (or the
`.claude/agents/spex-verifier.md` subagent) with a concrete, scoped
verification task and specific pass/fail criteria to check, and let it
report back a short summary. Good verification-fork prompts in this
project's history have included exact things to check ("confirm the debug
panel's hop count denominator matches N", "sample the tooltip at two points
~1s apart and confirm the content differs") rather than an open-ended "does
it look right?" — a concrete check is both cheaper to verify and easier to
trust the summary of.

## Stale server processes ruin a verification round-trip

If a `spex serve`/`spex gallery` background process was started before your
latest `cargo build --release` or before `demos/` was regenerated, it's
serving stale content — any verification against it is meaningless. Check
before trusting it:

```sh
ps aux | grep "target/release/spex" | grep -v grep
ps -p <pid> -o pid,lstart          # compare against: ls -la target/release/spex demos/
```

`pkill -f <pattern>` has failed to actually kill the old process more than
once in this project's history — if a restart doesn't seem to take effect,
verify the PID is actually gone (`kill -9 <pid>` then re-check) before
re-running the check against what might still be the old process.
