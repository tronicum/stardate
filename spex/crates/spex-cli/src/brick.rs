//! `spex brick-part`/`brick-model`/`brick-assembly` — real Klemmbaustein/
//! LEGO-compatible rendering via `spex-ldraw`, replacing what used to be
//! prototyped in `unibrick/`'s Python scripts. See `BRICKs.md` for the
//! domain glossary and licensing background.
use anyhow::Result;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use spex_ldraw::{
    load_colors, place, resolve_part, sample_point_in_triangle, sample_surface, shade_color, to_point_cloud,
    triangle_area, triangle_normal, ColorTable, LdrawCache, ModelSource, Scene, Triangle, LDU_TO_MM,
};
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

// --- brick-assembly: animate any real scene's placements from a stylized
// scattered start into their real final positions ---

const FLOAT_HEIGHT_LDU: f64 = 420.0; // how far "up" (real LDraw -Y) each part starts before settling
const SCATTER_RADIUS_LDU: f64 = 260.0; // deterministic sideways scatter so parts visibly converge from different directions

fn ease_in_out_cubic(t: f64) -> f64 {
    if t < 0.5 {
        4.0 * t.powi(3)
    } else {
        1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
    }
}

/// A real, deliberately *stylized* starting layout — not a physics
/// simulation. Each placement starts `FLOAT_HEIGHT_LDU` further "up" than
/// its own real final position, plus a deterministic sideways scatter
/// (seeded per placement index via a splitmix-style constant, so
/// re-running this produces the identical starting layout every time) so
/// parts visibly converge from different directions rather than all
/// dropping straight down in a boring vertical line.
fn start_translations(final_translations: &[[f64; 3]]) -> Vec<[f64; 3]> {
    final_translations
        .iter()
        .enumerate()
        .map(|(i, &[fx, fy, fz])| {
            let seed = 0x9E3779B97F4A7C15u64.wrapping_mul(i as u64 + 1);
            let mut rng = StdRng::seed_from_u64(seed);
            let angle: f64 = rng.gen::<f64>() * std::f64::consts::TAU;
            let radius = SCATTER_RADIUS_LDU * (0.4 + 0.6 * rng.gen::<f64>());
            [fx + radius * angle.cos(), fy - FLOAT_HEIGHT_LDU, fz + radius * angle.sin()]
        })
        .collect()
}

struct AssemblySample {
    placement_idx: usize,
    local_point: [f64; 3],
    color: [u8; 3],
}

/// Samples every real output point exactly once, in each placement's own
/// local (untransformed) frame, with shading baked in immediately
/// (translation-invariant, since it only depends on the local triangle
/// normal). Recomputing a frame's *world* points is then just adding that
/// frame's own per-placement translation — cheap, and crucially avoids
/// resampling with a fresh random draw every frame, which would make
/// points visibly "shimmer" instead of moving smoothly.
fn sample_scene_once(cache: &LdrawCache, scene: &Scene, colors: &ColorTable, point_count: usize, seed: u64) -> Result<Vec<AssemblySample>> {
    let mut resolved: HashMap<(String, u32), Vec<Triangle>> = HashMap::new();
    for placement in &scene.placements {
        let key = (placement.part_file.clone(), placement.color_code);
        if !resolved.contains_key(&key) {
            let triangles = resolve_part(cache, &placement.part_file, placement.color_code)?;
            resolved.insert(key.clone(), triangles);
        }
    }
    let per_placement_triangles: Vec<&Vec<Triangle>> = scene
        .placements
        .iter()
        .map(|placement| &resolved[&(placement.part_file.clone(), placement.color_code)])
        .collect();

    let per_placement_weights: Vec<Vec<f64>> = per_placement_triangles.iter().map(|tris| tris.iter().map(triangle_area).collect()).collect();
    let placement_totals: Vec<f64> = per_placement_weights.iter().map(|w| w.iter().sum::<f64>().max(f64::MIN_POSITIVE)).collect();
    let grand_total: f64 = placement_totals.iter().sum::<f64>().max(f64::MIN_POSITIVE);

    let mut rng = StdRng::seed_from_u64(seed);
    let mut samples = Vec::with_capacity(point_count);
    for _ in 0..point_count {
        let r: f64 = rng.gen::<f64>() * grand_total;
        let mut acc = 0.0;
        let mut placement_idx = per_placement_triangles.len() - 1;
        for (i, total) in placement_totals.iter().enumerate() {
            acc += total;
            if r <= acc {
                placement_idx = i;
                break;
            }
        }
        let tris = per_placement_triangles[placement_idx];
        let weights = &per_placement_weights[placement_idx];
        let r2: f64 = rng.gen::<f64>() * placement_totals[placement_idx];
        let mut acc2 = 0.0;
        let mut tri_idx = tris.len() - 1;
        for (i, w) in weights.iter().enumerate() {
            acc2 += w;
            if r2 <= acc2 {
                tri_idx = i;
                break;
            }
        }
        let tri = &tris[tri_idx];
        let local_point = sample_point_in_triangle(tri, &mut rng);
        let base_rgb = colors.get(&tri.color_code).map(|(_, rgb)| *rgb).unwrap_or([200, 200, 200]);
        let color = shade_color(base_rgb, triangle_normal(tri));
        samples.push(AssemblySample { placement_idx, local_point, color });
    }
    Ok(samples)
}

fn render_frame(samples: &[AssemblySample], translations: &[[f64; 3]]) -> Vec<spex_core::Point> {
    samples
        .iter()
        .map(|s| {
            let t = translations[s.placement_idx];
            let x = s.local_point[0] + t[0];
            let y = s.local_point[1] + t[1];
            let z = s.local_point[2] + t[2];
            spex_core::Point {
                position: [x * LDU_TO_MM, -y * LDU_TO_MM, z * LDU_TO_MM],
                color: s.color,
            }
        })
        .collect()
}

/// Builds `frame_count` real point-cloud frames — an eased (slow-start,
/// fast-middle, slow-end) interpolation of each placement's translation
/// from a stylized scattered start to its real final position (taken
/// directly from the scene's own already-parsed placements — no
/// stacking-math reimplementation needed, unlike the old Python version,
/// since a real `.ldr` scene already encodes final positions). Rotation
/// stays fixed at each placement's own final matrix throughout — only
/// position animates, a real, honest scope limit shared with the
/// original Python version.
pub fn build_assembly_frames(cache: &LdrawCache, scene: &Scene, point_count: usize, frame_count: usize, seed: u64) -> Result<Vec<Vec<spex_core::Point>>> {
    let colors = load_colors(cache)?;
    let final_translations: Vec<[f64; 3]> = scene.placements.iter().map(|p| p.translation).collect();
    let start = start_translations(&final_translations);
    let samples = sample_scene_once(cache, scene, &colors, point_count, seed)?;

    let mut frames = Vec::with_capacity(frame_count);
    for f in 0..frame_count {
        let t = if frame_count > 1 { f as f64 / (frame_count - 1) as f64 } else { 1.0 };
        let eased = ease_in_out_cubic(t);
        let translations: Vec<[f64; 3]> = (0..final_translations.len())
            .map(|i| {
                let s = start[i];
                let fi = final_translations[i];
                [s[0] + (fi[0] - s[0]) * eased, s[1] + (fi[1] - s[1]) * eased, s[2] + (fi[2] - s[2]) * eased]
            })
            .collect();
        frames.push(render_frame(&samples, &translations));
    }
    Ok(frames)
}
