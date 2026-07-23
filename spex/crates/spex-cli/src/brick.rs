//! `spex brick-part`/`brick-model`/`brick-assembly` — real Klemmbaustein/
//! LEGO-compatible rendering via `spex-ldraw`, replacing what used to be
//! prototyped in `unibrick/`'s Python scripts. See `BRICKs.md` for the
//! domain glossary and licensing background.
use anyhow::Result;
use spex_ldraw::{load_colors, resolve_part, sample_surface, to_point_cloud, LdrawCache};

/// A handful of known real LDraw part numbers, so `spex brick-part` has
/// something to point at without requiring the caller to already know an
/// LDraw part number — same spirit as `molecule.rs`'s `KNOWN_MOLECULES`.
pub const KNOWN_PARTS: &[(&str, &str)] = &[
    ("1x1-brick", "3005.dat"),
    ("1x4-brick", "3010.dat"),
    ("1x4-plate", "3710.dat"),
];

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
