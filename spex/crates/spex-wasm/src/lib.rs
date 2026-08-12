//! M86 — the boundary. One implementation of the resolver, two targets.
//!
//! `docs/fugen/phase6-wasm.md` describes the phase; `docs/fugen/wasm-duplikate.md`
//! counts what it is for. This crate is the first of its milestones and
//! deliberately the smallest one that can be true: **two exports, and the
//! pipeline proved before it is widened.**
//!
//! # What this is not
//!
//! It is not a second implementation of anything. `resolve_show` below calls
//! `spex_show::resolve` — the same function `spex show-build` calls, compiled
//! a second time for a second target. The pleasing consequence, once the
//! viewer bundles the module and `spex-server` embeds the viewer, is that one
//! executable contains the same `resolve()` twice: once as x86-64 and once as
//! wasm32.
//!
//! # Why the crate builds on the host too
//!
//! `wasm-bindgen` is a `cfg(target_arch = "wasm32")` dependency and the
//! `#[wasm_bindgen]` attributes are gated with it, so `cargo test` on the host
//! compiles this crate as an ordinary rlib and runs the equivalence assertion
//! below as an ordinary unit test. A browser is not needed to find out that
//! the resolver disagrees with itself, and a check that needs a browser is a
//! check that gets run once.
//!
//! # JSON in and JSON out, on purpose
//!
//! This is a cold path — once per page load — so legibility beats
//! serialisation cost, and the string boundary means the acceptance criterion
//! ("byte-identical to what the CLI writes") is a string comparison rather
//! than a structural one. M87's hot path is the opposite and shares nothing
//! with this: it writes f32 straight into the buffer three.js uploads.

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

/// The crate version, so a page can say which build it is running.
///
/// Trivial, and the first thing to call: if this returns, the module loaded,
/// the memory initialised and the string boundary works. Everything after is
/// detail.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Resolves a show document at a target duration — the SAME
/// `spex_show::resolve` the CLI calls.
///
/// Returns the resolved document as JSON, or the error text. The error is
/// returned rather than panicked because `resolve` errors are *authoring*
/// errors with a number in them ("the clamps make this target unreachable, by
/// 3.4 s"), and a panic in wasm loses the message behind an unhelpful trap.
pub fn resolve_show_impl(
    show_json: &str,
    target_sec: f64,
    seed: u64,
    endless: bool,
) -> Result<String, String> {
    let show = spex_show::from_str(show_json).map_err(|e| e.to_string())?;
    let resolved = spex_show::resolve(
        &show,
        &spex_show::ResolveOptions { target_sec, seed, endless },
    )
    .map_err(|e| e.to_string())?;
    serde_json::to_string_pretty(&resolved).map_err(|e| e.to_string())
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn resolve_show(
    show_json: &str,
    target_sec: f64,
    seed: u64,
    endless: bool,
) -> Result<String, JsValue> {
    resolve_show_impl(show_json, target_sec, seed, endless).map_err(|e| JsValue::from_str(&e))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo_root() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    /// M86's acceptance criterion 2, as a unit test rather than an errand.
    ///
    /// The wasm export must produce what `spex show-build` produces. Since the
    /// export is a thin wrapper over the same function, the honest thing to
    /// assert is that the wrapper does not *change* anything — same document,
    /// same seed, same duration, same bytes — and to assert it against the
    /// real screenplay rather than a fixture, because the real screenplay is
    /// the document that has 23 shots and a water-filling pass.
    #[test]
    fn the_wasm_entry_point_resolves_exactly_what_the_library_does() {
        let path = repo_root().join("shows/die-geschichtliche-matrix.show.json");
        let text = std::fs::read_to_string(&path).expect("reading the screenplay");
        let show = spex_show::from_str(&text).unwrap();

        for (target, endless) in [(240.0, false), (120.0, false), (240.0, true)] {
            let direct = spex_show::resolve(
                &show,
                &spex_show::ResolveOptions { target_sec: target, seed: show.seed, endless },
            )
            .unwrap();
            let direct_json = serde_json::to_string_pretty(&direct).unwrap();
            let through_wasm =
                resolve_show_impl(&text, target, show.seed, endless).expect("the wasm entry point");
            assert_eq!(
                direct_json, through_wasm,
                "the wasm boundary changed the resolved document at {target} s (endless {endless})"
            );
        }
    }

    /// An authoring error has a number in it, and the number has to survive
    /// the boundary — this is the whole reason the signature returns a
    /// `Result<String, String>` instead of unwrapping.
    #[test]
    fn an_unreachable_target_comes_back_as_a_message_and_not_a_trap() {
        let path = repo_root().join("shows/die-geschichtliche-matrix.show.json");
        let text = std::fs::read_to_string(&path).unwrap();
        // Far below the sum of every fixed shot's own duration.
        let err = resolve_show_impl(&text, 12.0, 1, false).expect_err("12 s cannot be reachable");
        assert!(
            err.chars().any(|c| c.is_ascii_digit()),
            "the shortfall message lost its number: {err}"
        );
    }

    #[test]
    fn a_document_that_is_not_a_show_says_so() {
        let err = resolve_show_impl("{\"nope\": 1}", 240.0, 1, false).expect_err("not a show");
        assert!(!err.is_empty());
    }
}
