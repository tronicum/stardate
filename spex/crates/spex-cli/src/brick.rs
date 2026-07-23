//! `spex brick-part`/`brick-model`/`brick-assembly` — real Klemmbaustein/
//! LEGO-compatible rendering via `spex-ldraw`, replacing what used to be
//! prototyped in `unibrick/`'s Python scripts. See `BRICKs.md` for the
//! domain glossary and licensing background.
use anyhow::Result;
use spex_ldraw::{load_colors, place, resolve_part, sample_surface, to_point_cloud, LdrawCache, ModelSource, Scene, Triangle};
use std::collections::HashMap;
use std::path::Path;

/// A handful of known real LDraw part numbers, so `spex brick-part` has
/// something to point at without requiring the caller to already know an
/// LDraw part number — same spirit as `molecule.rs`'s `KNOWN_MOLECULES`.
pub const KNOWN_PARTS: &[(&str, &str)] = &[
    ("1x1-brick", "3005.dat"),
    ("1x4-brick", "3010.dat"),
    ("1x4-plate", "3710.dat"),
];

/// A handful of known real official LDraw sample models (fetched live from
/// `library/official/models/` — see `scene.rs`), so `spex brick-model`/
/// `brick-assembly` have something to point at by name.
pub const KNOWN_MODELS: &[&str] = &["car", "pyramid"];

/// Decides whether a `spex brick-model`/`brick-assembly` argument refers
/// to a local file already on disk (e.g. `ldraw-scenes/monolith.ldr`) or a
/// named real official model to fetch (e.g. "car").
pub fn resolve_model_source(model: &str) -> ResolvedModelSource {
    let path = Path::new(model);
    if path.exists() {
        ResolvedModelSource::Local(path.to_path_buf())
    } else {
        let name = model.strip_suffix(".ldr").unwrap_or(model).to_string();
        ResolvedModelSource::Named(name)
    }
}

pub enum ResolvedModelSource {
    Local(std::path::PathBuf),
    Named(String),
}

impl ResolvedModelSource {
    pub fn as_model_source(&self) -> ModelSource<'_> {
        match self {
            ResolvedModelSource::Local(path) => ModelSource::LocalFile(path),
            ResolvedModelSource::Named(name) => ModelSource::Named(name),
        }
    }
}

/// Resolves every real *distinct* `(part, color)` pair a scene references
/// exactly once (mirrors `unibrick/gen_model_demo.py`'s own resolve-once
/// reuse, e.g. car.ldr's 61 real placements are only 26 distinct real
/// parts), places each real occurrence at its own real translation/matrix,
/// and samples the merged result into a real point cloud.
pub fn render_scene_to_points(cache: &LdrawCache, scene: &Scene, point_count: usize, seed: u64) -> Result<Vec<spex_core::Point>> {
    let colors = load_colors(cache)?;
    let mut resolved: HashMap<(String, u32), Vec<Triangle>> = HashMap::new();
    let mut all_triangles = Vec::new();
    for placement in &scene.placements {
        let key = (placement.part_file.clone(), placement.color_code);
        if !resolved.contains_key(&key) {
            let triangles = resolve_part(cache, &placement.part_file, placement.color_code)?;
            resolved.insert(key.clone(), triangles);
        }
        let triangles = &resolved[&key];
        let placed = place(triangles, placement.translation, placement.matrix, placement.color_code, None);
        all_triangles.extend(placed);
    }
    let samples = sample_surface(&all_triangles, &colors, point_count, seed);
    Ok(to_point_cloud(&samples))
}

pub fn resolve_part_alias(name: &str) -> &str {
    KNOWN_PARTS
        .iter()
        .find(|(alias, _)| *alias == name)
        .map(|(_, file)| *file)
        .unwrap_or(name)
}

/// Resolves and samples one real part into a real point cloud (still in
/// spex's standard mm/Y-up frame, via `to_point_cloud`) — the shared core
/// both `spex brick-part` and (later) `spex brick-model`'s per-placement
/// resolution build on.
pub fn render_part_to_points(
    cache: &LdrawCache,
    part_file: &str,
    color_code: u32,
    point_count: usize,
    seed: u64,
) -> Result<Vec<spex_core::Point>> {
    let triangles = resolve_part(cache, part_file, color_code)?;
    let colors = load_colors(cache)?;
    let samples = sample_surface(&triangles, &colors, point_count, seed);
    Ok(to_point_cloud(&samples))
}
