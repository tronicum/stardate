//! M64 — the generated choreography, and the one PRNG both sides can agree on.
//!
//! `spex brick-assembly` has been settling nine bricks out of a scattered
//! start since long before the Fugen Engine existed, and A1-S04 is that shot.
//! The milestone's job is to stop *baking* it into frames and evaluate it at
//! runtime instead — which means the browser has to produce the identical
//! starting layout the Rust demo does.
//!
//! # Why `rand::StdRng` had to go
//!
//! `brick.rs` seeded `StdRng::seed_from_u64` per placement. `StdRng` is
//! ChaCha12: a cryptographic stream, reimplementable in TypeScript only at
//! real cost, and — this is the part that settles it — **`rand` explicitly
//! does not promise its output is stable across versions.** A piece whose
//! visual choreography changes when a transitive dependency bumps a minor
//! version is not reproducible, and "the 2027 edition looked different after
//! `cargo update`" is not a sentence anyone wants to write.
//!
//! So the generator moved to **splitmix64** — twelve lines, a fixed
//! specification, bit-identical in any language with a u64. The old code's
//! comment already said "a splitmix-style constant"; it used that constant to
//! seed ChaCha. Now it is splitmix all the way through.
//!
//! **This changes the existing demo's scatter.** The parts start from
//! different directions than they used to. That is cosmetic — the layout was
//! always arbitrary-but-deterministic, and it is still arbitrary-but-
//! deterministic — and it buys the property the whole milestone rests on:
//! baked and runtime agree *by construction* rather than by coincidence.
//!
//! # Coordinates
//!
//! Everything here is in **LDraw units, Y-down**, because that is the frame
//! the constants were measured in and the frame `brick.rs` works in. The
//! viewer converts once, at the same boundary everything else does.

/// How far "up" (LDraw −Y) each part starts before settling. Ported, not
/// re-chosen: the existing demo's look depends on it.
pub const FLOAT_HEIGHT_LDU: f64 = 420.0;
/// Deterministic sideways scatter, so parts converge from different
/// directions rather than all dropping in a vertical line.
pub const SCATTER_RADIUS_LDU: f64 = 260.0;
/// The golden-ratio odd constant splitmix64 is defined with.
pub const SPLITMIX_GAMMA: u64 = 0x9E37_79B9_7F4A_7C15;

/// One step of splitmix64. Advances `state` and returns the mixed output.
#[inline]
pub fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(SPLITMIX_GAMMA);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// A double in [0,1), from the top 53 bits — the same construction
/// `Math.random()` is specified to produce, so the JavaScript port is exact
/// rather than approximately exact.
#[inline]
pub fn next_f64(state: &mut u64) -> f64 {
    (splitmix64(state) >> 11) as f64 / (1u64 << 53) as f64
}

/// The seed for one placement. Index-derived, so a scene's layout does not
/// change when a *different* scene is built, and edition-seeded, so the
/// endless edition's cycle advance actually reaches the choreography.
#[inline]
pub fn placement_seed(index: usize, edition_seed: u64) -> u64 {
    SPLITMIX_GAMMA
        .wrapping_mul(index as u64 + 1)
        .wrapping_add(edition_seed.wrapping_mul(0x2545_F491_4F6C_DD1D))
}

/// Where a placement starts, as an offset from its own final position, in
/// LDraw units (Y-down: the negative Y is *upward*).
pub fn start_offset_ldu(index: usize, edition_seed: u64) -> [f64; 3] {
    let mut state = placement_seed(index, edition_seed);
    let angle = next_f64(&mut state) * std::f64::consts::TAU;
    let radius = SCATTER_RADIUS_LDU * (0.4 + 0.6 * next_f64(&mut state));
    [radius * angle.cos(), -FLOAT_HEIGHT_LDU, radius * angle.sin()]
}

/// The curve the assembly settles on, and the reference implementation the
/// viewer's `easing.ts` is a port of.
///
/// It lived in `brick.rs` as `ease_in_out_cubic`. It is here now for the same
/// reason the scatter is: two implementations of one curve agree to about
/// three decimals, and this milestone's whole claim is that the baked demo
/// and the runtime shot produce the *same* positions.
#[inline]
pub fn cubic_in_out(t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
    }
}

/// A placement's own progress through an assembly, given the shot's progress.
///
/// `stagger` is the fraction of the shot spent handing over from the first
/// part to the last: at 0 everything lands together, at 1 the last part does
/// not begin until the first has finished. `order` is the placement's rank —
/// its index, or its real build step where the scene has `0 STEP` lines,
/// which is the whole reason those were parsed.
///
/// Returns raw 0..1 progress; the caller eases it. Keeping the easing out
/// means the same stagger works for a curve authored per shot.
pub fn staggered_progress(t01: f64, order: usize, count: usize, stagger: f64) -> f64 {
    if count <= 1 || stagger <= 0.0 {
        return t01.clamp(0.0, 1.0);
    }
    let s = stagger.clamp(0.0, 1.0);
    let span = 1.0 - s;
    // Each placement's window is `span` long and starts `s * rank/(n-1)` in.
    let start = s * (order.min(count - 1) as f64) / ((count - 1) as f64);
    if span <= f64::EPSILON {
        return if t01 >= start { 1.0 } else { 0.0 };
    }
    ((t01 - start) / span).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// splitmix64's first outputs from state 0 are published reference values.
    /// If this drifts, every edition of the piece drifts with it.
    #[test]
    fn splitmix64_matches_its_published_reference_stream() {
        let mut s = 0u64;
        assert_eq!(splitmix64(&mut s), 0xE220_A839_7B1D_CDAF);
        assert_eq!(splitmix64(&mut s), 0x6E78_9E6A_A1B9_65F4);
        assert_eq!(splitmix64(&mut s), 0x06C4_5D18_8009_454F);
    }

    #[test]
    fn a_double_from_splitmix_is_in_range_and_reproducible() {
        let mut a = 12345u64;
        let mut b = 12345u64;
        for _ in 0..1000 {
            let x = next_f64(&mut a);
            assert!((0.0..1.0).contains(&x), "{x} outside [0,1)");
            assert_eq!(x, next_f64(&mut b));
        }
    }

    /// The scatter is a *ring*, not a disc, and both bounds matter: the inner
    /// one keeps a part from starting on top of its own destination, the
    /// outer one keeps it in frame.
    #[test]
    fn every_start_offset_lands_in_the_authored_ring_and_at_the_authored_height() {
        for i in 0..500 {
            let [x, y, z] = start_offset_ldu(i, 0);
            let r = (x * x + z * z).sqrt();
            assert!(
                r >= SCATTER_RADIUS_LDU * 0.4 - 1e-9 && r <= SCATTER_RADIUS_LDU + 1e-9,
                "placement {i}: radius {r}"
            );
            assert_eq!(y, -FLOAT_HEIGHT_LDU);
        }
    }

    /// The edition seed has to actually reach the choreography, or an
    /// "edition" is the same film with a different number printed on it.
    #[test]
    fn a_different_edition_seed_produces_a_different_layout() {
        let a: Vec<[f64; 3]> = (0..9).map(|i| start_offset_ldu(i, 0)).collect();
        let b: Vec<[f64; 3]> = (0..9).map(|i| start_offset_ldu(i, 263865)).collect();
        assert_ne!(a, b);
        // ...but still deterministic within one edition.
        let a2: Vec<[f64; 3]> = (0..9).map(|i| start_offset_ldu(i, 0)).collect();
        assert_eq!(a, a2);
    }

    #[test]
    fn stagger_hands_over_from_the_first_placement_to_the_last() {
        // With no stagger everything moves together.
        assert_eq!(staggered_progress(0.5, 0, 9, 0.0), 0.5);
        assert_eq!(staggered_progress(0.5, 8, 9, 0.0), 0.5);

        // With stagger the last placement has not started when the first is
        // already half done.
        let first = staggered_progress(0.3, 0, 9, 0.6);
        let last = staggered_progress(0.3, 8, 9, 0.6);
        assert!(first > last, "{first} should lead {last}");
        assert_eq!(last, 0.0, "the last placement waits");

        // Everything is finished at the end, and nothing before the start.
        for order in 0..9 {
            assert_eq!(staggered_progress(0.0, order, 9, 0.6), if order == 0 { 0.0 } else { 0.0 });
            assert_eq!(staggered_progress(1.0, order, 9, 0.6), 1.0);
        }
    }

    /// The fixture both languages are pinned to. If this test fails, the Rust
    /// generator changed; if the browser check fails against the same file,
    /// the TypeScript one did. Either way the two have stopped agreeing, and
    /// that is the thing M64 exists to prevent.
    #[test]
    fn the_shared_fixture_still_describes_this_generator() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/fugen/fixtures/assembly-scatter.json");
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
        let fixture: serde_json::Value = serde_json::from_str(&text).unwrap();

        for case in fixture["cases"].as_array().unwrap() {
            let seed = case["editionSeed"].as_u64().unwrap();
            let expected = case["offsetsLdu"].as_array().unwrap();
            for (i, want) in expected.iter().enumerate() {
                let got = start_offset_ldu(i, seed);
                for k in 0..3 {
                    let w = want[k].as_f64().unwrap();
                    assert!(
                        (got[k] - w).abs() < 1e-9,
                        "seed {seed}, placement {i}, component {k}: {} vs fixture {w}",
                        got[k]
                    );
                }
            }
        }
    }
}
