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
use spex_ankerstein::{generate_shape, to_part_geometry, AnkersteinShape, Scene};
use spex_ldraw::geometry::Triangle;
use spex_ldraw::{place, rotation_y, sample_surface, ColorTable, Finish, LdrawColor};
use spex_mesh::{FullColorTable, MeshBundleBuilder, MeshBundleStats};
use std::collections::HashMap;
use std::path::Path;

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

/// The real, full `spex_mesh::FullColorTable` (finish/edge/alpha/luminance,
/// not just name+RGB) `MeshBundleBuilder::add_material` needs — the mesh
/// counterpart of `color_table()` above. LDraw has `LDConfig.ldr` to parse
/// for this; Ankerstein doesn't (see `color_table`'s own doc comment), so
/// this is a small, static, self-authored table keyed on the exact same
/// stable codes `color_code_for` assigns. `edge` is a real dark neutral
/// grey rather than each colour's own darkened shade — Ankerstein's real
/// mortar/edge lines read as a consistent dark seam regardless of the
/// stone's own colour, unlike LDraw's per-colour `EDGE` value (which
/// mimics injection-molded plastic's own material boundary, not mortar).
/// `finish: Solid` is the closest existing `Finish` to real baked
/// quartz-sand/chalk/linseed-oil stone — matte, not glossy like ABS
/// plastic, but there's no stone-specific PBR entry in `material.rs` yet;
/// a real follow-up, not pretended away here.
pub fn full_color_table() -> FullColorTable {
    let named = spex_ankerstein::load_colors();
    let mut table = FullColorTable::new();
    for (code, key) in [(1u32, "brick-red"), (2, "cement-yellow"), (3, "slate-blue-grey")] {
        if let Some(rgb) = named.get(key) {
            table.insert(
                code,
                LdrawColor {
                    code,
                    name: key.to_string(),
                    value: *rgb,
                    edge: [0x2a, 0x28, 0x24],
                    alpha: 255,
                    luminance: 0,
                    finish: Finish::Solid,
                    // Not from LDConfig.ldr at all — these are Ankerstein's
                    // own stone colours, and they belong to no LDraw section.
                    section: String::new(),
                },
            );
        }
    }
    table
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

/// Builds a real MESH bundle for a whole Ankerstein scene — the mesh
/// counterpart of `render_scene_to_points` above, and structurally
/// identical to `mesh::build_scene_bundle`'s LDraw scene loop: each
/// *distinct* real shape id is resolved into a `PartGeometry` exactly once
/// (`spex_ankerstein::to_part_geometry`, cached via `builder.part_index`)
/// no matter how many times the scene places it, then every real placement
/// becomes one real instance — real triangles, real crease-smoothed
/// normals, and real crisp edge outlines (`to_part_geometry`'s analytic
/// `EdgeKind::Hard` edges), not sampled points.
///
/// One global `color_code` for the whole scene, matching
/// `render_scene_to_points`'s own single `--color` — `Scene`/`Placement`
/// (`spex_ankerstein::scene`) has no per-placement colour field, so there
/// is nothing more specific to honour yet.
pub fn build_scene_mesh_bundle(scene: &Scene, color_code: u32, crease_degrees: f64, out_dir: &Path) -> Result<MeshBundleStats> {
    let colors = full_color_table();
    let mut builder = MeshBundleBuilder::new(crease_degrees);
    let material = builder.add_material(&colors, color_code);
    for (i, placement) in scene.placements.iter().enumerate() {
        let part = match builder.part_index(&placement.shape_id) {
            Some(p) => p,
            None => {
                let shape = find_shape(&placement.shape_id)?;
                let geometry = to_part_geometry(&shape).with_context(|| format!("building mesh geometry for shape {:?}", placement.shape_id))?;
                builder.add_part(&placement.shape_id, &geometry)
            }
        };
        // Ankerstein placements rotate only about the vertical (Y) axis
        // (`spex_ankerstein::scene::Placement::rotation_y_degrees`) — and a
        // pure Y-axis rotation is exactly invariant under
        // `spex_mesh::bundle::to_output_matrix`'s Y-flip conjugation (F M F
        // with F = diag(1,-1,1) leaves any matrix untouched wherever every
        // nonzero entry sits at F-index pairs of equal sign, which every
        // entry of a Y-only rotation does). So, unlike the translation
        // (which does need `to_bundle_frame_position`, see
        // `spex_ankerstein::geometry::to_bundle_frame_position`'s doc
        // comment), the real rotation matrix is passed straight through
        // with no pre-conversion — verified directly by this module's own
        // `y_rotation_is_invariant_under_the_bundle_frame_conversion` test.
        let matrix = rotation_y(placement.rotation_y_degrees.to_radians());
        let translation = spex_ankerstein::to_bundle_frame_position(placement.translation_mm);
        builder.add_instance(part, material, translation, &matrix, format!("{}/{i}", placement.shape_id))?;
    }
    builder.write(out_dir)
}

/// How many real *distinct* shape ids a scene references — the mesh
/// counterpart of `mesh::distinct_parts`.
pub fn distinct_shapes(scene: &Scene) -> usize {
    scene.placements.iter().map(|p| p.shape_id.as_str()).collect::<std::collections::HashSet<_>>().len()
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

    #[test]
    fn full_color_table_has_the_same_three_stable_codes_with_a_finish() {
        let table = full_color_table();
        assert_eq!(table.len(), 3);
        let red = table.get(&1).unwrap();
        assert_eq!(red.name, "brick-red");
        assert_eq!(red.finish, Finish::Solid);
        assert_eq!(red.alpha, 255);
    }

    /// The claim `build_scene_mesh_bundle`'s own comment makes: a pure
    /// Y-axis rotation matrix is left completely unchanged by
    /// `spex_mesh::bundle::to_output_matrix`'s Y-flip conjugation, so
    /// passing `rotation_y(...)` straight through to `add_instance` (no
    /// `to_bundle_frame_position`-style pre-conversion) is correct — not
    /// assumed, checked directly against the real function.
    #[test]
    fn y_rotation_is_invariant_under_the_bundle_frame_conversion() {
        for degrees in [0.0, 17.0, 90.0, 180.0, 271.0] {
            let m = rotation_y((degrees as f64).to_radians());
            let converted = spex_mesh::to_output_matrix(&m);
            for i in 0..9 {
                assert!((converted[i] - m[i]).abs() < 1e-9, "degrees {degrees}: index {i}: {converted:?} != {m:?}");
            }
        }
    }

    /// End-to-end proof that `build_scene_mesh_bundle` produces real,
    /// non-stub geometry for a real multi-shape scene: both `generate_box`
    /// (the two GK cubes) and `generate_prism` (the sloped roof) are
    /// exercised together, every distinct shape id is resolved exactly
    /// once, and every placement becomes one real instance.
    #[test]
    fn build_scene_mesh_bundle_resolves_distinct_shapes_once_and_every_placement_becomes_an_instance() {
        let scene = Scene {
            title: Some("test fixture: two cubes and a roof".to_string()),
            placements: vec![
                spex_ankerstein::Placement { shape_id: "gk-cube-full".to_string(), translation_mm: [-12.5, 0.0, 0.0], rotation_y_degrees: 0.0 },
                spex_ankerstein::Placement { shape_id: "gk-cube-full".to_string(), translation_mm: [12.5, 0.0, 0.0], rotation_y_degrees: 0.0 },
                spex_ankerstein::Placement { shape_id: "gk-prism-45".to_string(), translation_mm: [0.0, 37.5, 0.0], rotation_y_degrees: 0.0 },
            ],
        };
        assert_eq!(distinct_shapes(&scene), 2);

        let dir = std::env::temp_dir().join(format!("spex-ankerstein-mesh-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let color_code = color_code_for("brick-red").unwrap();
        let stats = build_scene_mesh_bundle(&scene, color_code, spex_mesh::DEFAULT_CREASE_DEGREES, &dir).unwrap();

        assert_eq!(stats.part_count, 2, "gk-cube-full and gk-prism-45, resolved once each");
        assert_eq!(stats.instance_count, 3, "every real placement becomes one real instance");
        assert_eq!(stats.material_count, 1, "one global colour for the whole scene");
        // Per distinct PART, not per instance — the box shape is instanced
        // twice but its 12 triangles/12 edges are only counted once, which
        // is the whole point of resolving each shape id exactly once.
        assert_eq!(stats.total_triangles, 12 + 8, "one box part (12 tri) + one prism part (8 tri)");
        assert_eq!(stats.total_hard_edges, 12 + 9, "one box part (12 edges) + one prism part (9 edges)");
        assert_eq!(stats.total_conditional_edges, 0, "no curved surfaces in Ankerstein geometry");
        assert!(dir.join("mesh.json").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn build_scene_mesh_bundle_rejects_a_placement_with_an_unknown_shape_id() {
        let scene = Scene {
            title: None,
            placements: vec![spex_ankerstein::Placement { shape_id: "does-not-exist".to_string(), translation_mm: [0.0, 0.0, 0.0], rotation_y_degrees: 0.0 }],
        };
        let dir = std::env::temp_dir().join(format!("spex-ankerstein-mesh-test-err-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        assert!(build_scene_mesh_bundle(&scene, 1, spex_mesh::DEFAULT_CREASE_DEGREES, &dir).is_err());
    }
}
