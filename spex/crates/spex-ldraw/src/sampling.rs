//! Real face-area-weighted surface sampling + baked-in Lambertian/specular
//! shading — pure geometry/color functions with no LDraw-fetch-specific
//! code. Takes any flat triangle list (a single resolved part, or several
//! placed parts merged into one assembly) and turns it into a colored
//! point cloud. Also the natural seam where a future true mesh/vector
//! renderer would diverge from the point-cloud pipeline (see `BRICKs.md`).
use crate::colors::ColorTable;
use crate::geometry::{triangle_area, triangle_normal, Triangle};
use crate::LDU_TO_MM;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

/// A fixed "headlight" direction near the viewer's own default camera
/// angle (spex's default camera sits at center + diagonal*0.6 on every
/// axis), so the baked-in highlight actually reads as light coming from
/// roughly where you're already looking from by default.
const LIGHT_DIR: [f64; 3] = [0.5774, 0.5774, 0.5774]; // normalize((0.6, 0.6, 0.6))
const AMBIENT_FLOOR: f64 = 0.35; // unlit faces stay dimly visible, not pure black
const SPECULAR_POWER: f64 = 28.0; // higher = tighter, glassier-looking highlight
const SPECULAR_STRENGTH: f64 = 0.55;

/// One sampled surface point, still in LDraw's native LDU coordinate frame
/// (Y-down) — real, already-shaded color, position not yet converted to
/// millimeters or flipped to spex's Y-up convention (see `to_point_cloud`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sample {
    pub position: [f64; 3],
    pub color: [u8; 3],
}

/// Bakes real Lambertian shading + a tight specular-style highlight
/// directly into a point's stored color, computed once from the real
/// triangle normal it was sampled from — neither spex's WebGL viewer nor
/// its ASCII renderer compute lighting at render time, so this is the
/// only way to get a "shiny" look out of either.
pub fn shade_color(base_rgb: [u8; 3], normal: [f64; 3]) -> [u8; 3] {
    let diffuse = (normal[0] * LIGHT_DIR[0] + normal[1] * LIGHT_DIR[1] + normal[2] * LIGHT_DIR[2]).max(0.0);
    let intensity = AMBIENT_FLOOR + (1.0 - AMBIENT_FLOOR) * diffuse;
    let specular = diffuse.powf(SPECULAR_POWER);
    let mut out = [0u8; 3];
    for i in 0..3 {
        let channel = base_rgb[i] as f64 * intensity + 255.0 * specular * SPECULAR_STRENGTH;
        out[i] = channel.round().clamp(0.0, 255.0) as u8;
    }
    out
}

/// Picks a real uniform-random point on a triangle's surface (barycentric
/// sampling) — exposed as a reusable primitive for callers (like an
/// assembly-animation choreographer) that need to sample a scene's real
/// surface points *once* and reuse them across many frames, rather than
/// going through the full `sample_surface` convenience wrapper per frame.
pub fn sample_point_in_triangle(tri: &Triangle, rng: &mut StdRng) -> [f64; 3] {
    let [v0, v1, v2] = tri.vertices;
    let mut u: f64 = rng.gen();
    let mut v: f64 = rng.gen();
    if u + v > 1.0 {
        u = 1.0 - u;
        v = 1.0 - v;
    }
    [
        v0[0] + u * (v1[0] - v0[0]) + v * (v2[0] - v0[0]),
        v0[1] + u * (v1[1] - v0[1]) + v * (v2[1] - v0[1]),
        v0[2] + u * (v1[2] - v0[2]) + v * (v2[2] - v0[2]),
    ]
}

/// Samples `point_count` points across `triangles`, real face-area
/// weighted so density stays even regardless of triangle size, and bakes
/// each point's real shading in immediately. `seed` makes a given call
/// reproducible (same principle as the fixed-seed German-cities-TSP demo
/// elsewhere in this project).
pub fn sample_surface(triangles: &[Triangle], colors: &ColorTable, point_count: usize, seed: u64) -> Vec<Sample> {
    if triangles.is_empty() {
        return Vec::new();
    }
    let weights: Vec<f64> = triangles.iter().map(triangle_area).collect();
    let normals: Vec<[f64; 3]> = triangles.iter().map(triangle_normal).collect();
    let total: f64 = weights.iter().sum::<f64>().max(f64::MIN_POSITIVE);

    let mut rng = StdRng::seed_from_u64(seed);
    let mut samples = Vec::with_capacity(point_count);
    for _ in 0..point_count {
        let r: f64 = rng.gen::<f64>() * total;
        let mut acc = 0.0;
        let mut idx = triangles.len() - 1;
        for (i, w) in weights.iter().enumerate() {
            acc += w;
            if r <= acc {
                idx = i;
                break;
            }
        }
        let tri = &triangles[idx];
        let base_rgb = colors.get(&tri.color_code).map(|(_, rgb)| *rgb).unwrap_or([200, 200, 200]);
        let position = sample_point_in_triangle(tri, &mut rng);
        let color = shade_color(base_rgb, normals[idx]);
        samples.push(Sample { position, color });
    }
    samples
}

/// Converts real LDraw-native samples (LDU units, Y-down) into spex's
/// standard `Point` (millimeters, Y-up) — LDraw's own coordinate
/// convention has +Y pointing down (a stud's tip is at negative Y), so Y
/// is negated only here, at the very end; shading is computed beforehand
/// in LDraw's native frame and needs no change.
pub fn to_point_cloud(samples: &[Sample]) -> Vec<spex_core::Point> {
    samples
        .iter()
        .map(|s| spex_core::Point {
            position: [s.position[0] * LDU_TO_MM, -s.position[1] * LDU_TO_MM, s.position[2] * LDU_TO_MM],
            color: s.color,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn xy_triangle(color_code: u32) -> Triangle {
        Triangle {
            vertices: [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            color_code,
        }
    }

    #[test]
    fn shade_color_never_goes_fully_black_due_to_ambient_floor() {
        // A normal facing directly away from the light should still clear
        // the real ambient floor, not clamp to (0,0,0).
        let shaded = shade_color([100, 100, 100], [-1.0, 0.0, 0.0]);
        let floor = (100.0 * AMBIENT_FLOOR).round() as u8;
        assert_eq!(shaded, [floor, floor, floor]);
    }

    #[test]
    fn shade_color_brightens_a_directly_lit_face() {
        let unlit = shade_color([100, 100, 100], [-1.0, 0.0, 0.0]);
        let lit = shade_color([100, 100, 100], LIGHT_DIR);
        assert!(lit[0] > unlit[0], "a face pointed straight at the light should be brighter than one facing away");
    }

    #[test]
    fn sample_surface_is_reproducible_given_the_same_seed() {
        let mut colors = HashMap::new();
        colors.insert(4u32, ("Red".to_string(), [201u8, 26, 9]));
        let triangles = vec![xy_triangle(4)];
        let a = sample_surface(&triangles, &colors, 50, 1337);
        let b = sample_surface(&triangles, &colors, 50, 1337);
        assert_eq!(a, b, "same seed must produce the same sampled points");
    }

    #[test]
    fn sample_surface_uses_the_real_color_table_and_falls_back_when_missing() {
        let colors = HashMap::new(); // deliberately empty - color 4 isn't in it
        let triangles = vec![xy_triangle(4)];
        let samples = sample_surface(&triangles, &colors, 10, 1);
        assert!(samples.iter().all(|s| s.color != [0, 0, 0]), "should fall back to a real gray, not black, when a color code is unknown");
    }

    #[test]
    fn to_point_cloud_applies_ldu_to_mm_and_flips_y() {
        let samples = vec![Sample { position: [10.0, 20.0, 30.0], color: [1, 2, 3] }];
        let points = to_point_cloud(&samples);
        assert_eq!(points[0].position, [10.0 * LDU_TO_MM, -20.0 * LDU_TO_MM, 30.0 * LDU_TO_MM]);
        assert_eq!(points[0].color, [1, 2, 3]);
    }
}
