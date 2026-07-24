//! `spex brick-part`/`brick-model`/`brick-assembly`/`brick-cinematic` —
//! real Klemmbaustein/LEGO-compatible rendering via `spex-ldraw`,
//! replacing what used to be prototyped in `unibrick/`'s Python scripts.
//! See `BRICKs.md` for the domain glossary and licensing background.
use anyhow::Result;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use spex_core::Aabb;
use spex_ldraw::geometry::mat_vec;
use spex_ldraw::{
    load_colors, place, resolve_part, rotation_y, sample_point_in_triangle, sample_surface, shade_color, to_point_cloud,
    triangle_area, triangle_normal, ColorTable, LdrawCache, ModelSource, Scene, Triangle, IDENTITY, LDU_TO_MM, ZERO,
};
use std::collections::HashMap;
use std::f64::consts::TAU;
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

/// One real sampled surface point, in a placement's own local
/// (untransformed) frame — deliberately storing the *unshaded* normal and
/// base color rather than a pre-baked one, so a caller can reshade it
/// correctly under whatever real rotation that frame applies (see
/// `Transform`/`render_frame`). The local point/normal themselves never
/// need resampling across frames — only the transform applied to them
/// changes — which is what keeps animated points moving smoothly instead
/// of shimmering.
struct PlacementSample {
    placement_idx: usize,
    local_point: [f64; 3],
    local_normal: [f64; 3],
    base_rgb: [u8; 3],
}

/// A real per-placement, per-frame transform: `world_point =
/// rotate(rotation, local_point) * scale + translation`, `world_normal =
/// rotate(rotation, local_normal)` (rotation only — a uniform position
/// scale about the origin doesn't change face orientation, so `scale`
/// deliberately never touches the normal). `rotation` must be a pure
/// rotation (orthonormal, det +1) for the normal transform to be valid —
/// true for `IDENTITY` and for `spex_ldraw::rotation_y`, the only two
/// rotations this module ever constructs.
struct Transform {
    translation: [f64; 3],
    rotation: [f64; 9],
    scale: f64,
}

impl Transform {
    fn translate(translation: [f64; 3]) -> Self {
        Transform { translation, rotation: IDENTITY, scale: 1.0 }
    }
}

/// Real face-area-weighted sampling across several placements' worth of
/// local (untransformed) triangles at once — the core both
/// `sample_scene_once` (multi-placement assembly) and `build_spin_frames`
/// (a single "placement") share. Returns `PlacementSample`s carrying the
/// *unshaded* normal/base color, so a caller reshades per frame under
/// whatever real transform it applies (see `Transform`/`render_frame`).
fn sample_placements_once(triangles_per_placement: &[Vec<Triangle>], colors: &ColorTable, point_count: usize, seed: u64) -> Vec<PlacementSample> {
    let weights: Vec<Vec<f64>> = triangles_per_placement.iter().map(|tris| tris.iter().map(triangle_area).collect()).collect();
    let placement_totals: Vec<f64> = weights.iter().map(|w| w.iter().sum::<f64>().max(f64::MIN_POSITIVE)).collect();
    let grand_total: f64 = placement_totals.iter().sum::<f64>().max(f64::MIN_POSITIVE);

    let mut rng = StdRng::seed_from_u64(seed);
    let mut samples = Vec::with_capacity(point_count);
    for _ in 0..point_count {
        let r: f64 = rng.gen::<f64>() * grand_total;
        let mut acc = 0.0;
        let mut placement_idx = triangles_per_placement.len() - 1;
        for (i, total) in placement_totals.iter().enumerate() {
            acc += total;
            if r <= acc {
                placement_idx = i;
                break;
            }
        }
        let tris = &triangles_per_placement[placement_idx];
        let tri_weights = &weights[placement_idx];
        let r2: f64 = rng.gen::<f64>() * placement_totals[placement_idx];
        let mut acc2 = 0.0;
        let mut tri_idx = tris.len() - 1;
        for (i, w) in tri_weights.iter().enumerate() {
            acc2 += w;
            if r2 <= acc2 {
                tri_idx = i;
                break;
            }
        }
        let tri = &tris[tri_idx];
        let local_point = sample_point_in_triangle(tri, &mut rng);
        let local_normal = triangle_normal(tri);
        let base_rgb = colors.get(&tri.color_code).map(|(_, rgb)| *rgb).unwrap_or([200, 200, 200]);
        samples.push(PlacementSample { placement_idx, local_point, local_normal, base_rgb });
    }
    samples
}

/// Resolves every real distinct `(part, color)` pair in a scene exactly
/// once, then samples across all its placements at once.
fn sample_scene_once(cache: &LdrawCache, scene: &Scene, point_count: usize, seed: u64) -> Result<(Vec<PlacementSample>, ColorTable)> {
    let colors = load_colors(cache)?;
    let mut resolved: HashMap<(String, u32), Vec<Triangle>> = HashMap::new();
    for placement in &scene.placements {
        let key = (placement.part_file.clone(), placement.color_code);
        if !resolved.contains_key(&key) {
            let triangles = resolve_part(cache, &placement.part_file, placement.color_code)?;
            resolved.insert(key.clone(), triangles);
        }
    }
    let triangles_per_placement: Vec<Vec<Triangle>> = scene
        .placements
        .iter()
        .map(|placement| resolved[&(placement.part_file.clone(), placement.color_code)].clone())
        .collect();
    let samples = sample_placements_once(&triangles_per_placement, &colors, point_count, seed);
    Ok((samples, colors))
}

/// Applies each sample's placement's real `Transform` and reshades it
/// fresh from the *rotated* normal — correct for both the translate-only
/// assembly case (`rotation: IDENTITY` is a no-op on the normal, so this
/// is bit-for-bit what baking the color once used to produce) and the
/// rotate-in-place spin case (where the normal genuinely does change
/// frame to frame under a fixed world-space light).
fn render_frame(samples: &[PlacementSample], transforms: &[Transform]) -> Vec<spex_core::Point> {
    samples
        .iter()
        .map(|s| {
            let t = &transforms[s.placement_idx];
            let rotated_point = mat_vec(&t.rotation, &s.local_point);
            let world_point = [
                rotated_point[0] * t.scale + t.translation[0],
                rotated_point[1] * t.scale + t.translation[1],
                rotated_point[2] * t.scale + t.translation[2],
            ];
            let world_normal = mat_vec(&t.rotation, &s.local_normal);
            let color = shade_color(s.base_rgb, world_normal);
            spex_core::Point {
                position: [world_point[0] * LDU_TO_MM, -world_point[1] * LDU_TO_MM, world_point[2] * LDU_TO_MM],
                color,
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
    let final_translations: Vec<[f64; 3]> = scene.placements.iter().map(|p| p.translation).collect();
    let start = start_translations(&final_translations);
    let (samples, _colors) = sample_scene_once(cache, scene, point_count, seed)?;

    let mut frames = Vec::with_capacity(frame_count);
    for f in 0..frame_count {
        let t = if frame_count > 1 { f as f64 / (frame_count - 1) as f64 } else { 1.0 };
        let eased = ease_in_out_cubic(t);
        let transforms: Vec<Transform> = (0..final_translations.len())
            .map(|i| {
                let s = start[i];
                let fi = final_translations[i];
                Transform::translate([s[0] + (fi[0] - s[0]) * eased, s[1] + (fi[1] - s[1]) * eased, s[2] + (fi[2] - s[2]) * eased])
            })
            .collect();
        frames.push(render_frame(&samples, &transforms));
    }
    Ok(frames)
}

// --- brick-cinematic: a real hero part spinning solo, then cutting into a
// real scene's own assembly animation ---

/// Builds `frame_count` real frames of a single real part spinning in
/// place at the scene origin — real constant angular velocity (no easing;
/// a hero spin should read as continuous motion, not settle to a stop,
/// since it cuts directly into another animation with no hold), one full
/// real rotation about LDraw's own Y axis per `revolutions`. `hero_scale`
/// uniformly scales the part's own *sampled positions* (never its
/// normals — see `Transform`) around its own center, a deliberate
/// cinematic choice so a small hero part reads as prominent even though
/// the viewer computes one shared camera framing across every frame in
/// the whole sequence (see `compute_hero_scale`).
pub fn build_spin_frames(
    cache: &LdrawCache,
    part_file: &str,
    color_code: u32,
    point_count: usize,
    frame_count: usize,
    revolutions: f64,
    hero_scale: f64,
    seed: u64,
) -> Result<Vec<Vec<spex_core::Point>>> {
    let triangles = resolve_part(cache, part_file, color_code)?;
    let colors = load_colors(cache)?;
    let samples = sample_placements_once(&[triangles], &colors, point_count, seed);

    let mut frames = Vec::with_capacity(frame_count);
    for f in 0..frame_count {
        // Exclusive framing (divide by frame_count, not frame_count - 1):
        // frame 0 and a hypothetical "next full loop" frame aren't a
        // visually duplicated pair, which matters here since this phase
        // cuts directly into another one with no hold on the last frame.
        let theta = revolutions * TAU * f as f64 / frame_count as f64;
        let transforms = vec![Transform { translation: ZERO, rotation: rotation_y(theta), scale: hero_scale }];
        frames.push(render_frame(&samples, &transforms));
    }
    Ok(frames)
}

/// A real, computed scale ratio (not a fabricated constant) for the hero
/// spin: the viewer frames its camera once across every frame in a whole
/// sequence, so a small solo part sharing a sequence with a much larger
/// multi-part scene would otherwise render as a barely-visible speck
/// during its own hero shot. Scales the hero part up to roughly a real
/// fraction of the scene's own real spatial extent (`scene_diag`, from
/// the scene's own placement translations — cheap, no full resolve
/// needed) relative to the part's own real resolved size (`part_diag`).
/// The fraction is a deliberate cinematic choice (prominent, not
/// literally to-scale against the context shot that follows), tuned
/// visually against a real headless-Chromium screenshot the same way
/// this project has always tuned constants like M38's animated-ASCII
/// width or M42's auto-rotate speed — an initial `0.5` measured as only
/// ~30x40px in an 800px-tall real screenshot (too modest to read as a
/// hero shot); `2.5` (roughly a 5x jump) was the real, re-verified fix.
pub fn compute_hero_scale(hero_triangles: &[Triangle], scene: &Scene) -> f64 {
    const HERO_PROMINENCE_FRACTION: f64 = 2.5;
    let part_diag = Aabb::from_points(hero_triangles.iter().flat_map(|t| t.vertices)).diagonal();
    let scene_diag = Aabb::from_points(scene.placements.iter().map(|p| p.translation)).diagonal();
    if part_diag <= 0.0 || scene_diag <= 0.0 {
        return 1.0;
    }
    (scene_diag / part_diag) * HERO_PROMINENCE_FRACTION
}
