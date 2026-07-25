# Phase 7 - shipping (M91-M97)

*Determinism, compatibility, accessibility, deployment, the single-file edition, the archive record.*

**Part of [`docs/fugen/`](README.md) — the Fugen Engine implementation spec, rev 3.**
Read [`README.md`](README.md) first: it carries the working rules, the scope decision and the milestone index.
Review record and the reasoning behind every rev-2 change: [`../FUGEN-ENGINE-REVIEW-01.md`](../FUGEN-ENGINE-REVIEW-01.md).

---


| M | Deliverable |
|---|---|
| **M91** | **Determinism harness.** Headless capture of 20 sampled frames + an audio render hash per (seed, duration); committed as a regression fixture. Any commit that changes a frame hash must say why in its message. |
| **M92** | **Performance and compatibility matrix.** Chrome/Firefox/Safari × macOS/Windows/Linux × High/Medium/Low. Record real fps and real load times. Safari is the one that will hurt; find out in week 25, not on launch day. |
| **M93** | **Accessibility and fallbacks.** `prefers-reduced-motion` (the Kick becomes a 2 s fade, the wave stills, the orbit slows), keyboard control (space, arrows, `m`, `f`), captions for every chronicle card, a WebGL-unavailable message that is a *statement* rather than an error, and a genuinely watchable Low tier. |
| **M94** | **Static export + deployment** to `research.iunctura.org/matrix/` (or `fugen.iunctura.de`), via the existing `export-static` discipline: relative paths throughout, works from a domain root or a subpath, no backend. |
| **M95** | **The single-file edition.** One self-contained HTML file with the wasm, the bundles and the score inlined (base64), for the on-chain edition and for the USB stick that goes to Billund with the Postilla. Size budget: ≤ 12 MB. Seed injected from the token hash at mint time — one documented entry point, `window.__SPEX_SEED__`. |
| **M96** | **Documentation sync.** `CLAUDE.md`, `AGENTS.md`, `BRICKs.md`, `README.md`, `spec/README.md`, `TODOs.md` (M51–M97 entries in the established style), `docs/LICENSING.md`, `docs/agents/wasm.md`. A reader arriving cold must be able to build the piece from the repo alone. |
| **M97** | **The archive record.** `quellenregister.json` extended with every source this phase used; IPFS pin; the Ethereum anchor; the run that produced the canonical edition recorded with its exact seed, commit hash, and frame hashes. The work is not finished until it is *provable*. |

---
