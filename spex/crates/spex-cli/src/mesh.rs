//! `spex mesh-part` / `spex mesh-model` — real LDraw geometry out as a mesh
//! bundle instead of a point cloud.
//!
//! The point-cloud verbs (`brick-part`, `brick-model`, `brick-assembly`,
//! `brick-cinematic`) are untouched and keep working exactly as before. These
//! are a second, additive output path: same resolution, same scenes, a
//! different thing on the other end.
use anyhow::{Context, Result};
use spex_ldraw::{load_colors, parse_scene, resolve_part_full, LdrawCache, Scene};
use spex_mesh::{MeshBundleBuilder, MeshBundleStats, DEFAULT_CREASE_DEGREES};
use std::collections::HashSet;
use std::path::Path;

/// Builds a bundle containing exactly one real part, placed once at the
/// origin — the smallest useful case, and the object M57's edge screenshots
/// are taken of.
pub fn build_part_bundle(
    cache: &LdrawCache,
    part_file: &str,
    color_code: u32,
    crease_degrees: f64,
    out_dir: &Path,
) -> Result<MeshBundleStats> {
    let geometry = resolve_part_full(cache, part_file)?;
    let colors = load_colors(cache)?;
    let mut builder = MeshBundleBuilder::new(crease_degrees);
    let part = builder.add_part(part_file, &geometry);
    let material = builder.add_material(&colors, color_code);
    builder.add_instance(
        part,
        material,
        spex_ldraw::ZERO,
        &spex_ldraw::IDENTITY,
        format!("{part_file}/0"),
    )?;
    builder.write(out_dir)
}

/// Builds a bundle for a whole real scene.
///
/// Each *distinct* part is resolved exactly once no matter how many times the
/// scene places it — the same discipline `brick::render_scene_to_points`
/// already applies, and here it matters more: the resolved geometry carries no
/// colour (M51 keeps LDraw code 16 unresolved), so a part placed in six
/// colours is still one mesh. That is the whole basis of instancing, and it is
/// why parts key on the filename alone.
pub fn build_scene_bundle(
    cache: &LdrawCache,
    scene: &Scene,
    crease_degrees: f64,
    out_dir: &Path,
) -> Result<MeshBundleStats> {
    let colors = load_colors(cache)?;
    let mut builder = MeshBundleBuilder::new(crease_degrees);
    for (i, placement) in scene.placements.iter().enumerate() {
        let part = match builder.part_index(&placement.part_file) {
            Some(p) => p,
            None => {
                let g = resolve_part_full(cache, &placement.part_file)
                    .with_context(|| format!("resolving {:?}", placement.part_file))?;
                builder.add_part(&placement.part_file, &g)
            }
        };
        let material = builder.add_material(&colors, placement.color_code);
        builder.add_instance(
            part,
            material,
            placement.translation,
            &placement.matrix,
            format!("{}/{i}", placement.part_file.trim_end_matches(".dat")),
        )?;
    }
    builder.write(out_dir)
}

/// How many real *distinct* parts a scene references — printed so the
/// resolve-once claim is visible rather than asserted.
pub fn distinct_parts(scene: &Scene) -> usize {
    scene
        .placements
        .iter()
        .map(|p| p.part_file.as_str())
        .collect::<HashSet<_>>()
        .len()
}

pub fn parse_scene_arg(cache: &LdrawCache, model: &str) -> Result<Scene> {
    let source = crate::brick::resolve_model_source(model);
    parse_scene(cache, source.as_model_source())
}

pub fn default_crease() -> f64 {
    DEFAULT_CREASE_DEGREES
}
