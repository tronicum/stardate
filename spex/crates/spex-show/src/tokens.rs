//! A4-S03's token flow — the Rust twin of `viewer/src/show/tokens.ts`.
//!
//! # Why this exists at all, when nothing in Rust renders it
//!
//! Nothing does, yet. `docs/fugen/wasm-duplikate.md` counts what this project
//! implements twice and names this generator as the one piece that is
//! TypeScript *only* — which sounds like the safe side of the ledger and is
//! the opposite. Phase 6 (M87) moves the evaluator into Rust and compiles it
//! to wasm; a generator with no Rust side is a generator M87 has to *port*
//! under deadline rather than *delete*, and porting a walk is exactly the
//! operation where an off-by-one in the hop index is invisible.
//!
//! So the twin is written now, while the TypeScript is fresh and while there
//! is time to pin it. Both are pinned to
//! `docs/fugen/fixtures/token-flow.json` — generated from the TypeScript
//! itself, bundled and run — rather than to each other, the same arrangement
//! `choreography.rs` and `choreography.ts` already have.
//!
//! # The three decisions this file inherits, and must not soften
//!
//! - **The walk is reflected at the boundary, not wrapped.** A token that
//!   wraps crosses the whole frame in one frame and reads as a glitch.
//! - **`position_at` walks the hops from the start on every call.** O(hops),
//!   fourteen here. A cached incremental walk would be faster and would drift
//!   on a seek, which is the bug M66 spent a shot's worth of frames finding in
//!   the camera.
//! - **The arc is a half sine over the hop**, so it is zero at both ends: a
//!   token is absorbed at the node's own level, not above it.

use crate::choreography::{next_f64, placement_seed};

/// The four cardinal steps on the grid, in the order a seeded pick indexes.
const STEPS: [(f64, f64); 4] = [(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)];

/// What a `seed` cue with `"generator": "tokens"` declares.
#[derive(Debug, Clone, PartialEq)]
pub struct TokenFlowSpec {
    /// Node (0,0) of the lattice, in millimetres.
    pub origin_mm: [f64; 3],
    /// Distance between adjacent nodes, in millimetres.
    pub pitch_mm: f64,
    /// Nodes per side. The walk is reflected at 0 and `nodes - 1`.
    pub nodes: u32,
    /// Hops the whole shot is worth. One hop is one edge crossed.
    pub hops: u32,
    /// Peak height of the travelling arc above the lattice, in millimetres.
    pub arc_mm: f64,
    pub edition_seed: u64,
}

/// Reflects a grid coordinate back inside `[0, n - 1]`.
fn reflect(v: f64, n: f64) -> f64 {
    if v < 0.0 {
        -v
    } else if v > n - 1.0 {
        2.0 * (n - 1.0) - v
    } else {
        v
    }
}

/// Where token `i` is at `t01` of the shot, in millimetres.
///
/// Deliberately allocation-free and stateless, for the reason in the header.
pub fn position_at(spec: &TokenFlowSpec, i: usize, t01: f64) -> [f64; 3] {
    let nodes = spec.nodes as f64;
    let t = t01.clamp(0.0, 1.0);

    // Where this token starts. Two draws from two seeds, exactly as the
    // TypeScript does it — one seed advanced twice would be a different walk
    // and would look just as plausible.
    let mut rng = placement_seed(i, spec.edition_seed);
    let mut gx = (next_f64(&mut rng) * nodes).floor();
    let mut rng = placement_seed(i + 1013, spec.edition_seed);
    let mut gz = (next_f64(&mut rng) * nodes).floor();

    let hops = spec.hops.max(1);
    let total = t * hops as f64;
    let hop = (total.floor() as u32).min(hops - 1);
    let u = total - hop as f64;

    // Walk the completed hops, then take the one in progress.
    let mut sx = 0.0;
    let mut sz = 0.0;
    for h in 0..=hop {
        let mut pick = placement_seed(i * 7919 + h as usize, spec.edition_seed);
        let idx = ((next_f64(&mut pick) * 4.0).floor() as usize).min(3);
        let (dx, dz) = STEPS[idx];
        sx = dx;
        sz = dz;
        if h < hop {
            gx = reflect(gx + dx, nodes);
            gz = reflect(gz + dz, nodes);
        }
    }
    let nx = reflect(gx + sx, nodes);
    let nz = reflect(gz + sz, nodes);

    let fx = gx + (nx - gx) * u;
    let fz = gz + (nz - gz) * u;

    [
        spec.origin_mm[0] + fx * spec.pitch_mm,
        spec.origin_mm[1] + (std::f64::consts::PI * u).sin() * spec.arc_mm,
        spec.origin_mm[2] + fz * spec.pitch_mm,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a4_s03() -> TokenFlowSpec {
        TokenFlowSpec {
            origin_mm: [10804.0, 12.0, -252.0],
            pitch_mm: 32.0,
            nodes: 17,
            hops: 14,
            arc_mm: 26.0,
            edition_seed: 263865,
        }
    }

    /// The one test that matters: the same numbers the browser produces.
    #[test]
    fn the_shared_fixture_still_describes_this_walk() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/fugen/fixtures/token-flow.json");
        let text = std::fs::read_to_string(&path).expect("reading the token-flow fixture");
        let doc: serde_json::Value = serde_json::from_str(&text).unwrap();
        let spec = a4_s03();

        for sample in doc["samples"].as_array().unwrap() {
            let t01 = sample["t01"].as_f64().unwrap();
            let instances = sample["instances"].as_array().unwrap();
            let positions = sample["positionsMm"].as_array().unwrap();
            for (n, inst) in instances.iter().enumerate() {
                let i = inst.as_u64().unwrap() as usize;
                let got = position_at(&spec, i, t01);
                let want = positions[n].as_array().unwrap();
                for k in 0..3 {
                    let w = want[k].as_f64().unwrap();
                    assert!(
                        (got[k] - w).abs() < 1e-9,
                        "instance {i} at t01={t01}, component {k}: {} vs {w}",
                        got[k]
                    );
                }
            }
        }
    }

    /// Reflected, not wrapped — the decision the header calls a reading rather
    /// than an implementation detail, and the one a port would silently undo.
    #[test]
    fn the_walk_reflects_at_the_boundary_and_never_wraps() {
        let spec = a4_s03();
        let lo = spec.origin_mm[0];
        let hi = spec.origin_mm[0] + (spec.nodes as f64 - 1.0) * spec.pitch_mm;
        let mut worst_step: f64 = 0.0;
        for i in 0..48 {
            let mut prev = position_at(&spec, i, 0.0);
            for k in 1..=560 {
                let p = position_at(&spec, i, k as f64 / 560.0);
                assert!(p[0] >= lo - 1e-9 && p[0] <= hi + 1e-9, "token {i} left the lattice in x");
                assert!(
                    p[2] >= spec.origin_mm[2] - 1e-9
                        && p[2] <= spec.origin_mm[2] + (spec.nodes as f64 - 1.0) * spec.pitch_mm + 1e-9,
                    "token {i} left the lattice in z"
                );
                // A wrap would show up as a single step of the whole lattice.
                let step = ((p[0] - prev[0]).powi(2) + (p[2] - prev[2]).powi(2)).sqrt();
                worst_step = worst_step.max(step);
                prev = p;
            }
        }
        assert!(
            worst_step < spec.pitch_mm,
            "largest step {worst_step} mm is a whole pitch or more — that is a wrap"
        );
    }

    /// The arc is zero at both ends of every hop, because a token is absorbed
    /// at the node's own level.
    #[test]
    fn the_arc_returns_to_the_plane_at_every_node() {
        let spec = a4_s03();
        for i in [0usize, 5, 31, 47] {
            for hop in 0..spec.hops {
                let t = hop as f64 / spec.hops as f64;
                let p = position_at(&spec, i, t);
                assert!(
                    (p[1] - spec.origin_mm[1]).abs() < 1e-9,
                    "token {i} is {} mm off the plane at hop {hop}",
                    p[1] - spec.origin_mm[1]
                );
            }
            let mid = position_at(&spec, i, 0.5 / spec.hops as f64);
            assert!(
                (mid[1] - spec.origin_mm[1] - spec.arc_mm).abs() < 1e-9,
                "the arc should peak at exactly arcMm mid-hop"
            );
        }
    }
}
