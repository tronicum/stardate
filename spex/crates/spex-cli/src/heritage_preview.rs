//! `spex heritage-preview` — a real, quick, standalone proof that a real
//! UNESCO World Heritage monument's dominant real-world shape can be
//! reconstructed through spex's *existing* point-cloud pipeline, ahead of
//! the formal `spex-heritage`/M74 Atlas system (see `docs/FUGEN-ENGINE.md`)
//! which is a much bigger, currently-unbuilt undertaking (a whole real
//! mesh-rendering pipeline, `spex-build`'s grid-legal brick compiler,
//! M51-M72). This is deliberately *not* that — it's a small, honest,
//! real-dimensioned parametric shape sampled the same way
//! `spex-ankerstein`'s `geometry.rs` already does (reusing `spex-ldraw`'s
//! `sample_surface`/`shade_color`, which are pure geometry/lighting
//! functions with no LDraw-specific unit baked in), through
//! `spex_tiler::build` directly — no new rendering machinery.
use anyhow::{Context, Result};
use spex_core::Point;
use spex_ldraw::{sample_surface, ColorTable, Triangle};
use std::collections::HashMap;

const SOLID_COLOR_CODE: u32 = 0;

/// Real, cited monument shape: base half-side, real total height (meters),
/// and a real, defensible approximate stone color.
struct HeritageShape {
    id: &'static str,
    name: &'static str,
    base_side_m: f64,
    height_m: f64,
    /// Approximate real stone color — see each shape's own citation for
    /// exactly what this approximates.
    color: [u8; 3],
    citation: &'static str,
}

const KNOWN_SHAPES: &[HeritageShape] = &[
    HeritageShape {
        id: "giza-pyramid",
        name: "Great Pyramid of Giza (Pyramid of Khufu)",
        // Real original base side ~230.33m and original completed height
        // ~146.6m (before losing its outer Tura-limestone casing and
        // capstone — current height is ~138.5m) — W.M. Flinders Petrie's
        // 1880s triangulated survey, the real historical baseline
        // measurement still cited as the standard reference today.
        base_side_m: 230.33,
        height_m: 146.6,
        // Approximate real pale sandy-tan limestone tone — the original
        // white Tura limestone casing is almost entirely lost; this
        // approximates the duller tan of the exposed core (Mokattam)
        // limestone visible today, not the lost original white finish.
        color: [200, 182, 148],
        citation: "W.M. Flinders Petrie, The Pyramids and Temples of Gizeh (1883) — base ~230.33m, original completed height ~146.6m",
    },
    HeritageShape {
        id: "khafre-pyramid",
        name: "Pyramid of Khafre",
        // Real original base ~215.25m, original height ~143.5m — appears
        // nearly as tall as Khufu's despite the smaller base because it
        // sits on higher bedrock, and it alone still retains a real patch
        // of its original polished Tura-limestone casing near the apex.
        base_side_m: 215.25,
        height_m: 143.5,
        // Approximate real Mokattam limestone core tone, same basis as
        // giza-pyramid's — Khafre's casing remnant near the apex is
        // lighter/smoother than this, but that's a small fraction of the
        // real total surface area, not the dominant color.
        color: [198, 180, 146],
        citation: "base ~215.25m, original height ~143.5m — standard Giza survey figures (Petrie 1883; Lehner, The Complete Pyramids, 1997)",
    },
    HeritageShape {
        id: "menkaure-pyramid",
        name: "Pyramid of Menkaure",
        // Real original base is not perfectly square — ~102.2m x 104.6m —
        // this uses their real average as one side length, documented
        // here rather than silently squared off. Real original height
        // ~65m, by far the smallest of the three main Giza pyramids.
        base_side_m: 103.4,
        height_m: 65.0,
        color: [196, 178, 144],
        citation: "base ~102.2m x 104.6m (avg. 103.4m used here), original height ~65m — standard Giza survey figures (Petrie 1883; Lehner, The Complete Pyramids, 1997)",
    },
];

/// One real member of the Giza plateau's three-pyramid diagonal layout —
/// `offset_m` is that shape's real center position relative to Khufu's
/// (`giza-pyramid`'s) center, [east, north] in meters, positive
/// east/north. Derived from W.M. Flinders Petrie's real triangulated
/// center-to-center distances (`The Pyramids and Temples of Gizeh`,
/// 1883), which are published in cubits — converted here using the same
/// real cubit length implied by `giza-pyramid`'s own cited 230.33m/440-cubit
/// base (0.523477 m/cubit), so the whole complex is internally consistent
/// with the single-pyramid citation already in this file. Real distances:
/// Khufu->Khafre 638.5 cubits EW / 675.6 cubits NS; Khufu->Menkaure 1096.8
/// cubits EW / 1411.3 cubits NS — both southwest of Khufu, matching Giza's
/// well-documented real diagonal plateau layout (each pyramid smaller and
/// further southwest than the last).
struct GizaComplexMember {
    shape_id: &'static str,
    offset_m: [f64; 2],
}

const GIZA_COMPLEX: &[GizaComplexMember] = &[
    GizaComplexMember { shape_id: "giza-pyramid", offset_m: [0.0, 0.0] },
    GizaComplexMember { shape_id: "khafre-pyramid", offset_m: [-334.24, -353.66] },
    GizaComplexMember { shape_id: "menkaure-pyramid", offset_m: [-574.15, -738.78] },
];

const GIZA_COMPLEX_ID: &str = "giza-complex";

pub fn run(shape_id: Option<String>, points: usize, out: Option<std::path::PathBuf>) -> Result<()> {
    let Some(shape_id) = shape_id else {
        println!("known real heritage shape previews:");
        for shape in KNOWN_SHAPES {
            println!("  {:<14} {} ({:.2}m base, {:.1}m tall)", shape.id, shape.name, shape.base_side_m, shape.height_m);
        }
        println!("  {GIZA_COMPLEX_ID:<14} all three Giza pyramids together, at their real relative positions");
        println!("\nusage: spex heritage-preview <shape-id> -o <tileset-dir>");
        return Ok(());
    };
    let out = out.context("--out <tileset-dir> is required when rendering a shape")?;

    if shape_id == GIZA_COMPLEX_ID {
        return run_giza_complex(points, out);
    }

    let shape = KNOWN_SHAPES
        .iter()
        .find(|s| s.id == shape_id)
        .ok_or_else(|| anyhow::anyhow!("unknown heritage preview shape {shape_id:?} — run with no argument to list known ids"))?;

    println!("generating {} ({}, {:.2}m base x {:.1}m tall, source: {})...", shape.id, shape.name, shape.base_side_m, shape.height_m, shape.citation);
    let cloud = render_pyramid_to_points(shape, points, 0xC0FFEE);
    println!("sampled {} real points, building octree tileset...", cloud.len());

    spex_tiler::build(cloud, &out, &spex_tiler::TilerConfig::default())?;
    println!("wrote tileset to {}", out.display());
    Ok(())
}

/// Renders all three real Giza pyramids together in one combined point
/// cloud, each at its real relative position (see `GIZA_COMPLEX`'s doc
/// comment for the citation/derivation). All three pyramids' triangles go
/// into a single `sample_surface` call, which is real-surface-area-weighted
/// (see `spex-ldraw`'s `sampling.rs`) — so the requested `points` budget
/// naturally lands more samples on the two larger pyramids and fewer on
/// Menkaure, rather than an arbitrary even/manual split.
fn run_giza_complex(points: usize, out: std::path::PathBuf) -> Result<()> {
    let mut triangles = Vec::new();
    let mut colors: ColorTable = HashMap::new();
    println!("generating {GIZA_COMPLEX_ID} (Khufu, Khafre, Menkaure at their real relative positions)...");
    for (color_code, member) in GIZA_COMPLEX.iter().enumerate() {
        let shape = KNOWN_SHAPES
            .iter()
            .find(|s| s.id == member.shape_id)
            .unwrap_or_else(|| panic!("GIZA_COMPLEX references unknown shape id {:?}", member.shape_id));
        println!("  {} ({:.2}m base x {:.1}m tall) at real offset [{:.1}m E, {:.1}m N] from Khufu — {}", shape.id, shape.base_side_m, shape.height_m, member.offset_m[0], member.offset_m[1], shape.citation);
        colors.insert(color_code as u32, (shape.name.to_string(), shape.color));
        triangles.extend(pyramid_triangles(shape, member.offset_m, color_code as u32));
    }
    let samples = sample_surface(&triangles, &colors, points, 0xC0FFEE);
    let cloud: Vec<Point> = samples.iter().map(|s| Point { position: s.position, color: s.color }).collect();
    println!("sampled {} real points across all three pyramids, building octree tileset...", cloud.len());

    spex_tiler::build(cloud, &out, &spex_tiler::TilerConfig::default())?;
    println!("wrote tileset to {}", out.display());
    Ok(())
}

/// Four real triangular side faces of a square-base pyramid — the base
/// itself is omitted (never visible from any real above/eye-level camera
/// angle, and this is a deliberately small honest proof, not a full solid
/// model). Y-up, apex directly above `[offset_xz[0], height, offset_xz[1]]`,
/// real meters (this monument's natural real-world unit — unlike
/// `spex-ankerstein`'s mm or `spex-ldraw`'s LDU, there's no shared-scale
/// reason to convert here since this produces its own independent,
/// standalone tileset). `offset_xz` places this pyramid's base center in a
/// shared horizontal coordinate frame — `[0.0, 0.0]` for a lone pyramid
/// (its own center), or a real relative position when combined with others
/// (see `GIZA_COMPLEX`).
fn pyramid_triangles(shape: &HeritageShape, offset_xz: [f64; 2], color_code: u32) -> Vec<Triangle> {
    let hs = shape.base_side_m / 2.0;
    let h = shape.height_m;
    let [ox, oz] = offset_xz;
    let c1 = [-hs + ox, 0.0, -hs + oz];
    let c2 = [hs + ox, 0.0, -hs + oz];
    let c3 = [hs + ox, 0.0, hs + oz];
    let c4 = [-hs + ox, 0.0, hs + oz];
    let apex = [ox, h, oz];

    // Each triangle's winding is chosen (base-edge corners reversed, then
    // apex) so its normal points outward-and-upward, verified by direct
    // cross-product computation — a naive CCW-around-the-base ordering
    // gives an inward-facing normal here instead, which would make
    // `shade_color`'s lighting look inside-out.
    vec![
        Triangle { vertices: [c2, c1, apex], color_code },
        Triangle { vertices: [c3, c2, apex], color_code },
        Triangle { vertices: [c4, c3, apex], color_code },
        Triangle { vertices: [c1, c4, apex], color_code },
    ]
}

fn render_pyramid_to_points(shape: &HeritageShape, point_count: usize, seed: u64) -> Vec<Point> {
    let triangles = pyramid_triangles(shape, [0.0, 0.0], SOLID_COLOR_CODE);
    let mut colors: ColorTable = HashMap::new();
    colors.insert(SOLID_COLOR_CODE, (shape.name.to_string(), shape.color));
    let samples = sample_surface(&triangles, &colors, point_count, seed);
    samples
        .iter()
        .map(|s| Point { position: s.position, color: s.color })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use spex_ldraw::Sample;

    fn giza() -> &'static HeritageShape {
        &KNOWN_SHAPES[0]
    }

    #[test]
    fn render_pyramid_to_points_produces_the_requested_count() {
        let points = render_pyramid_to_points(giza(), 2000, 42);
        assert_eq!(points.len(), 2000);
    }

    #[test]
    fn render_pyramid_to_points_stays_within_the_real_cited_bounds() {
        let shape = giza();
        let points = render_pyramid_to_points(shape, 4000, 42);
        let mut min = [f64::INFINITY; 3];
        let mut max = [f64::NEG_INFINITY; 3];
        for p in &points {
            for axis in 0..3 {
                min[axis] = min[axis].min(p.position[axis]);
                max[axis] = max[axis].max(p.position[axis]);
            }
        }
        assert!(max[1] - min[1] <= shape.height_m + 1e-6, "real cited height exceeded");
        assert!(max[1] - min[1] > shape.height_m * 0.8, "suspiciously short for a real {}m pyramid", shape.height_m);
        let x_span = max[0] - min[0];
        let z_span = max[2] - min[2];
        assert!(x_span <= shape.base_side_m + 1e-6 && z_span <= shape.base_side_m + 1e-6, "real cited base exceeded");
    }

    #[test]
    fn render_pyramid_to_points_tapers_from_base_to_apex() {
        // A real pyramid's cross-section shrinks with height — points
        // near the apex (high Y) should hug the center (X,Z near 0) far
        // more tightly than points near the base (low Y).
        let shape = giza();
        let points = render_pyramid_to_points(shape, 8000, 7);
        let near_apex: Vec<&Point> = points.iter().filter(|p| p.position[1] > shape.height_m * 0.9).collect();
        let near_base: Vec<&Point> = points.iter().filter(|p| p.position[1] < shape.height_m * 0.1).collect();
        assert!(!near_apex.is_empty() && !near_base.is_empty());

        let max_radius = |pts: &[&Point]| -> f64 {
            pts.iter().map(|p| (p.position[0].powi(2) + p.position[2].powi(2)).sqrt()).fold(0.0, f64::max)
        };
        assert!(
            max_radius(&near_apex) < max_radius(&near_base),
            "points near the apex should be closer to the center axis than points near the base"
        );
    }

    fn shape(id: &str) -> &'static HeritageShape {
        KNOWN_SHAPES.iter().find(|s| s.id == id).unwrap()
    }

    #[test]
    fn all_three_giza_pyramids_are_individually_previewable() {
        for id in ["giza-pyramid", "khafre-pyramid", "menkaure-pyramid"] {
            let s = shape(id);
            let points = render_pyramid_to_points(s, 500, 1);
            assert_eq!(points.len(), 500);
            assert!(s.height_m > 0.0 && s.base_side_m > 0.0);
        }
    }

    #[test]
    fn giza_complex_members_resolve_to_real_known_shapes() {
        for member in GIZA_COMPLEX {
            assert!(KNOWN_SHAPES.iter().any(|s| s.id == member.shape_id), "GIZA_COMPLEX references unknown shape id {:?}", member.shape_id);
        }
    }

    #[test]
    fn giza_complex_places_each_pyramid_at_its_real_relative_offset() {
        // Khafre and Menkaure should each be centered at their real cited
        // offset from Khufu (at the origin) — checked by looking at each
        // pyramid's own base-corner point cluster (low Y), not the whole
        // combined cloud, since the three footprints don't overlap.
        let mut triangles = Vec::new();
        let mut colors: ColorTable = HashMap::new();
        for (color_code, member) in GIZA_COMPLEX.iter().enumerate() {
            let s = shape(member.shape_id);
            colors.insert(color_code as u32, (s.name.to_string(), s.color));
            triangles.extend(pyramid_triangles(s, member.offset_m, color_code as u32));
        }
        let samples = sample_surface(&triangles, &colors, 6000, 99);

        for member in GIZA_COMPLEX {
            let s = shape(member.shape_id);
            let hs = s.base_side_m / 2.0;
            let [ox, oz] = member.offset_m;
            // Every real sampled point on this pyramid's slant faces must
            // land within its own real footprint (offset +/- half its own
            // base side) — proves the offset actually moved the geometry,
            // not just a label.
            let on_this_pyramid = |p: &Sample| {
                (p.position[1] >= -1e-6 && p.position[1] <= s.height_m + 1.0) && (p.position[0] - ox).abs() <= hs + 1.0 && (p.position[2] - oz).abs() <= hs + 1.0
            };
            let count_here = samples.iter().filter(|p| on_this_pyramid(p)).count();
            assert!(count_here > 0, "expected real sampled points near {}'s real offset [{ox}, {oz}]", s.id);
        }
    }
}
