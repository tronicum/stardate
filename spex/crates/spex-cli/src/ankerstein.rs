//! `spex ankerstein-part` — real Ankerstein (Richter's Anchor Stone
//! Building Set, Rudolstadt 1880) rendering via `spex-ankerstein`. Reuses
//! `spex-ldraw`'s real face-area-weighted surface sampling (unit-agnostic,
//! works on any flat triangle list) rather than duplicating it — but
//! deliberately NOT its `to_point_cloud`, which bakes in an LDraw-specific
//! LDU-to-mm conversion and a Y-axis flip that don't apply here:
//! Ankerstein shapes are generated directly in real millimeters, Y-up,
//! from the start (see `spex_ankerstein::geometry`), unlike LDraw's native
//! Y-down LDU frame. See `docs/ANKERSTEIN-ENGINE.md` for the full spec.
use anyhow::{Context, Result};
use spex_ankerstein::{generate_shape, AnkersteinShape, Scene};
use spex_ldraw::geometry::Triangle;
use spex_ldraw::{place, rotation_y, sample_surface, ColorTable};
use std::collections::HashMap;

/// A fixed, small color-code table this module invents itself — Ankerstein
/// has no `LDConfig.ldr` equivalent to parse, unlike `spex-ldraw`. Maps
/// each of `spex_ankerstein`'s three named colors to an explicit, stable
/// `u32` code (never derived from `HashMap` iteration order, which isn't
/// guaranteed stable) plus its real RGB, in the exact `ColorTable` shape
/// `spex_ldraw::sample_surface` expects, so that function can be reused
/// unchanged.
pub fn color_table() -> ColorTable {
    let named = spex_ankerstein::load_colors();
    let mut table = ColorTable::new();
    for (code, key) in [(1u32, "brick-red"), (2, "cement-yellow"), (3, "slate-blue-grey")] {
        if let Some(rgb) = named.get(key) {
            table.insert(code, (key.to_string(), *rgb));
        }
    }
    table
}

/// Maps a color name (as typed on the CLI) to the stable code `color_table`
/// assigns it — kept as its own function rather than inlined so a caller
/// can validate a `--color` argument before doing any real geometry work.
pub fn color_code_for(name: &str) -> Result<u32> {
    match name {
        "brick-red" => Ok(1),
        "cement-yellow" => Ok(2),
        "slate-blue-grey" => Ok(3),
        other => anyhow::bail!("unknown Ankerstein color {other:?} — expected one of: brick-red, cement-yellow, slate-blue-grey"),
    }
}

/// Looks up a real catalog shape by id from the seed catalog (see
/// `docs/ANKERSTEIN-ENGINE.md` §2) — not yet reading `data/ankerstein-shapes.json`
/// from disk (M98's seed set is small enough to keep in memory; switch to
/// `spex_ankerstein::load_catalog` once the catalog grows past what's
/// convenient to hardcode).
pub fn find_shape(shape_id: &str) -> Result<AnkersteinShape> {
    spex_ankerstein::catalog::seed_shapes()
        .into_iter()
        .find(|s| s.id == shape_id)
        .with_context(|| format!("unknown Ankerstein shape id {shape_id:?} — run `spex ankerstein-part` with no argument to list known ids"))
}

/// Generates a real catalog shape's geometry and samples it into a real
/// point cloud — already in spex's standard mm/Y-up frame (see this
/// module's own doc comment for why no unit conversion or axis flip is
/// needed here, unlike the LDraw path).
pub fn render_shape_to_points(shape: &AnkersteinShape, color_name: &str, point_count: usize, seed: u64) -> Result<Vec<spex_core::Point>> {
    let color_code = color_code_for(color_name)?;
    let triangles = generate_shape(shape, color_code)?;
    let colors = color_table();
    let samples = sample_surface(&triangles, &colors, point_count, seed);
    Ok(samples.iter().map(|s| spex_core::Point { position: s.position, color: s.color }).collect())
}

/// Renders a real assembled scene (see `spex_ankerstein::Scene`/`Placement`)
/// into a real point cloud — the "spex ankerstein-model" counterpart to
/// `render_shape_to_points`'s single-shape case. Resolves each *distinct*
/// real catalog shape id exactly once (mirrors `brick::render_scene_to_points`'s
/// resolve-once pattern for LDraw parts), reusing `spex-ldraw`'s own
/// `rotation_y`/`place` for the per-placement transform — both are pure
/// matrix math with no LDraw-specific unit assumptions baked in, unlike
/// `to_point_cloud` (see this module's own doc comment), so they're safe
/// to reuse unchanged here.
pub fn render_scene_to_points(scene: &Scene, color_name: &str, point_count: usize, seed: u64) -> Result<Vec<spex_core::Point>> {
    let color_code = color_code_for(color_name)?;
    let colors = color_table();
    let mut resolved: HashMap<String, Vec<Triangle>> = HashMap::new();
    let mut all_triangles = Vec::new();
    for placement in &scene.placements {
        if !resolved.contains_key(&placement.shape_id) {
            let shape = find_shape(&placement.shape_id)?;
            let triangles = generate_shape(&shape, color_code)?;
            resolved.insert(placement.shape_id.clone(), triangles);
        }
        let triangles = &resolved[&placement.shape_id];
        let matrix = rotation_y(placement.rotation_y_degrees.to_radians());
        let placed = place(triangles, placement.translation_mm, matrix, color_code, None);
        all_triangles.extend(placed);
    }
    let samples = sample_surface(&all_triangles, &colors, point_count, seed);
    Ok(samples.iter().map(|s| spex_core::Point { position: s.position, color: s.color }).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_the_seed_gk_cube_at_correct_real_bounds() {
        // The seed catalog cites this shape as a real 25x25x25mm GK cube
        // (docs/ANKERSTEIN-ENGINE.md §2) - the rendered point cloud's own
        // bounds should match that, the same "real bounds match real cited
        // dimensions" check spex-ldraw's M40 used for its 1x1 LDraw brick.
        let shape = find_shape("gk-cube-full").unwrap();
        let points = render_shape_to_points(&shape, "brick-red", 2000, 42).unwrap();
        assert_eq!(points.len(), 2000);
        let bounds = spex_core::Aabb::from_points(points.iter().map(|p| p.position));
        let size = [bounds.max[0] - bounds.min[0], bounds.max[1] - bounds.min[1], bounds.max[2] - bounds.min[2]];
        for (axis, span) in size.iter().enumerate() {
            assert!((span - 25.0).abs() < 1.0, "axis {axis}: expected a ~25mm span, got {span}");
        }
    }

    #[test]
    fn renders_the_half_height_block_with_a_flattened_y_axis() {
        // Real cited dimensions: 25 x 12.5 x 25mm (width x height x depth) -
        // the middle (height/Y) axis should be roughly half the other two.
        let shape = find_shape("gk-half-height").unwrap();
        let points = render_shape_to_points(&shape, "slate-blue-grey", 2000, 7).unwrap();
        let bounds = spex_core::Aabb::from_points(points.iter().map(|p| p.position));
        let size = [bounds.max[0] - bounds.min[0], bounds.max[1] - bounds.min[1], bounds.max[2] - bounds.min[2]];
        assert!((size[0] - 25.0).abs() < 1.0, "width: {}", size[0]);
        assert!((size[1] - 12.5).abs() < 1.0, "height: {}", size[1]);
        assert!((size[2] - 25.0).abs() < 1.0, "depth: {}", size[2]);
    }

    #[test]
    fn rejects_an_unknown_color_name() {
        let shape = find_shape("gk-cube-full").unwrap();
        assert!(render_shape_to_points(&shape, "not-a-real-color", 100, 1).is_err());
    }

    #[test]
    fn rejects_an_unknown_shape_id() {
        assert!(find_shape("does-not-exist").is_err());
    }

    #[test]
    fn render_scene_to_points_places_multiple_shapes_at_real_offsets() {
        // Two adjacent GK cubes (25mm apart, center-to-center, each cube
        // itself 25mm wide) should produce a real combined bounding box
        // 50mm wide on that axis (25mm center-to-center span plus a
        // 12.5mm half-width beyond each outer center), not overlapping
        // or collapsed into one shape's worth of points.
        let scene = Scene {
            title: Some("test fixture, not a real historical assembly".to_string()),
            placements: vec![
                spex_ankerstein::Placement { shape_id: "gk-cube-full".to_string(), translation_mm: [-12.5, 0.0, 0.0], rotation_y_degrees: 0.0 },
                spex_ankerstein::Placement { shape_id: "gk-cube-full".to_string(), translation_mm: [12.5, 0.0, 0.0], rotation_y_degrees: 0.0 },
            ],
        };
        let points = render_scene_to_points(&scene, "brick-red", 4000, 99).unwrap();
        assert_eq!(points.len(), 4000);
        let bounds = spex_core::Aabb::from_points(points.iter().map(|p| p.position));
        let width = bounds.max[0] - bounds.min[0];
        assert!((width - 50.0).abs() < 1.0, "expected a ~50mm combined width (two 25mm cubes, centers 25mm apart), got {width}");
    }

    #[test]
    fn render_scene_to_points_rejects_a_placement_with_an_unknown_shape_id() {
        let scene = Scene {
            title: None,
            placements: vec![spex_ankerstein::Placement { shape_id: "does-not-exist".to_string(), translation_mm: [0.0, 0.0, 0.0], rotation_y_degrees: 0.0 }],
        };
        assert!(render_scene_to_points(&scene, "brick-red", 100, 1).is_err());
    }

    #[test]
    fn color_table_has_exactly_the_three_stable_codes() {
        let table = color_table();
        assert_eq!(table.len(), 3);
        assert_eq!(table.get(&1).map(|(name, _)| name.as_str()), Some("brick-red"));
        assert_eq!(table.get(&2).map(|(name, _)| name.as_str()), Some("cement-yellow"));
        assert_eq!(table.get(&3).map(|(name, _)| name.as_str()), Some("slate-blue-grey"));
    }
}
