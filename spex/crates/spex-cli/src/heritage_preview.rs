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

const KNOWN_SHAPES: &[HeritageShape] = &[HeritageShape {
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
}];

pub fn run(shape_id: Option<String>, points: usize, out: Option<std::path::PathBuf>) -> Result<()> {
    let Some(shape_id) = shape_id else {
        println!("known real heritage shape previews:");
        for shape in KNOWN_SHAPES {
            println!("  {:<14} {} ({:.2}m base, {:.1}m tall)", shape.id, shape.name, shape.base_side_m, shape.height_m);
        }
        println!("\nusage: spex heritage-preview <shape-id> -o <tileset-dir>");
        return Ok(());
    };
    let out = out.context("--out <tileset-dir> is required when rendering a shape")?;
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

/// Four real triangular side faces of a square-base pyramid — the base
/// itself is omitted (never visible from any real above/eye-level camera
/// angle, and this is a deliberately small honest proof, not a full solid
/// model). Y-up, apex directly above the base center, real meters (this
/// monument's natural real-world unit — unlike `spex-ankerstein`'s mm or
/// `spex-ldraw`'s LDU, there's no shared-scale reason to convert here
/// since this produces its own independent, standalone tileset).
fn render_pyramid_to_points(shape: &HeritageShape, point_count: usize, seed: u64) -> Vec<Point> {
    let hs = shape.base_side_m / 2.0;
    let h = shape.height_m;
    let c1 = [-hs, 0.0, -hs];
    let c2 = [hs, 0.0, -hs];
    let c3 = [hs, 0.0, hs];
    let c4 = [-hs, 0.0, hs];
    let apex = [0.0, h, 0.0];

    // Each triangle's winding is chosen (base-edge corners reversed, then
    // apex) so its normal points outward-and-upward, verified by direct
    // cross-product computation — a naive CCW-around-the-base ordering
    // gives an inward-facing normal here instead, which would make
    // `shade_color`'s lighting look inside-out.
    let triangles = vec![
        Triangle { vertices: [c2, c1, apex], color_code: SOLID_COLOR_CODE },
        Triangle { vertices: [c3, c2, apex], color_code: SOLID_COLOR_CODE },
        Triangle { vertices: [c4, c3, apex], color_code: SOLID_COLOR_CODE },
        Triangle { vertices: [c1, c4, apex], color_code: SOLID_COLOR_CODE },
    ];

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
}
