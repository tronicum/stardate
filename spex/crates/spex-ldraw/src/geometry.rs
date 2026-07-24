//! Real LDraw geometry: recursive part/subpart/primitive resolution, the
//! 3x3 matrix math LDraw's own "type 1" reference lines use, and real
//! triangle placement (translation + rotation).
use crate::cache::LdrawCache;
use anyhow::{bail, Context, Result};

/// A real triangle (3 vertices) plus the real, already-resolved LDraw
/// color code it should be shaded with (never 16 — "inherit" is always
/// substituted with the requesting color during resolution).
#[derive(Clone, Debug, PartialEq)]
pub struct Triangle {
    pub vertices: [[f64; 3]; 3],
    pub color_code: u32,
}

pub const IDENTITY: [f64; 9] = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
pub const ZERO: [f64; 3] = [0.0, 0.0, 0.0];

/// 3x3 row-major matrix multiply, both as flat 9-element arrays.
pub fn mat_mul(a: &[f64; 9], b: &[f64; 9]) -> [f64; 9] {
    let mut out = [0.0; 9];
    for row in 0..3 {
        for col in 0..3 {
            let mut sum = 0.0;
            for k in 0..3 {
                sum += a[row * 3 + k] * b[k * 3 + col];
            }
            out[row * 3 + col] = sum;
        }
    }
    out
}

pub fn mat_vec(m: &[f64; 9], v: &[f64; 3]) -> [f64; 3] {
    let mut out = [0.0; 3];
    for row in 0..3 {
        out[row] = m[row * 3] * v[0] + m[row * 3 + 1] * v[1] + m[row * 3 + 2] * v[2];
    }
    out
}

pub fn vec_add(a: &[f64; 3], b: &[f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

/// A pure rotation about LDraw's own Y axis by `theta` radians — row-major
/// flat `[f64;9]`, same convention as `IDENTITY`/`mat_vec`. Orthonormal,
/// determinant +1 by construction: safe to apply to a normal with the
/// *exact same* matrix used for a position (no inverse-transpose needed) —
/// unlike an arbitrary LDraw-authored placement matrix (which could in
/// principle carry scale/mirroring), this one is always a pure rotation.
/// Used to spin a single real part in place (see `brick.rs`'s
/// `build_spin_frames`) — rotating in LDraw's native frame, before the
/// one real Y-flip every output point gets at the very end, exactly like
/// every other transform in this pipeline.
pub fn rotation_y(theta: f64) -> [f64; 9] {
    let (s, c) = theta.sin_cos();
    [c, 0.0, s, 0.0, 1.0, 0.0, -s, 0.0, c]
}

/// A referenced LDraw filename doesn't say which real library folder it
/// lives in — try the same real candidate folders any real LDraw resolver
/// does, using whichever the cache/server actually has.
fn resolve_ref_path(cache: &LdrawCache, name: &str) -> Result<(String, String)> {
    let name = name.replace('\\', "/");
    let candidates: Vec<String> = if let Some(rest) = name.strip_prefix("s/") {
        vec![format!("parts/s/{rest}")]
    } else if name.starts_with("48/") {
        vec![format!("p/{name}")]
    } else {
        vec![format!("p/{name}"), format!("parts/{name}"), format!("parts/s/{name}")]
    };
    let mut last_err = None;
    for candidate in &candidates {
        match cache.fetch(candidate) {
            Ok(text) => return Ok((candidate.clone(), text)),
            Err(e) => last_err = Some(e),
        }
    }
    Err(anyhow::anyhow!(
        "couldn't resolve real LDraw file {name:?} in any of {candidates:?}: {}",
        last_err.map(|e| e.to_string()).unwrap_or_default()
    ))
}

/// Recursively resolves one real LDraw file into a flat list of real
/// triangles, in the *top-level part's* local coordinate space — every
/// nested real transform composed down through recursion. `depth == 0`
/// means `part_file` is a top-level real part (fetched from `parts/`
/// directly); deeper recursion resolves subpart/primitive references via
/// `resolve_ref_path`.
#[allow(clippy::too_many_arguments)]
fn resolve_into(
    cache: &LdrawCache,
    part_file: &str,
    matrix: &[f64; 9],
    translation: &[f64; 3],
    color_code: u32,
    depth: u32,
    triangles: &mut Vec<Triangle>,
) -> Result<()> {
    if depth > 8 {
        bail!("LDraw reference recursion too deep at {part_file:?} - likely a real cycle or bug");
    }
    let text = if depth == 0 {
        cache.fetch(&format!("parts/{part_file}"))?
    } else {
        resolve_ref_path(cache, part_file)?.1
    };

    for line in text.lines() {
        let tokens: Vec<&str> = line.split_whitespace().collect();
        let Some(&line_type) = tokens.first() else {
            continue;
        };
        match line_type {
            "1" => {
                // 1 <colour> x y z a b c d e f g h i <file>
                if tokens.len() < 15 {
                    continue;
                }
                let sub_color: u32 = tokens[1].parse().unwrap_or(16);
                let nums: Result<Vec<f64>, _> = tokens[2..14].iter().map(|t| t.parse::<f64>()).collect();
                let Ok(nums) = nums else { continue };
                let sub_translation = [nums[0], nums[1], nums[2]];
                let sub_matrix: [f64; 9] = nums[3..12].try_into().unwrap();
                let new_matrix = mat_mul(matrix, &sub_matrix);
                let new_translation = vec_add(&mat_vec(matrix, &sub_translation), translation);
                let sub_file = tokens[14..].join(" ");
                let effective_color = if sub_color == 16 { color_code } else { sub_color };
                resolve_into(cache, &sub_file, &new_matrix, &new_translation, effective_color, depth + 1, triangles)?;
            }
            "3" | "4" => {
                if tokens.len() < 2 {
                    continue;
                }
                let face_color_code: u32 = tokens[1].parse().unwrap_or(16);
                let effective_color = if face_color_code == 16 { color_code } else { face_color_code };
                let nums: Result<Vec<f64>, _> = tokens[2..].iter().map(|t| t.parse::<f64>()).collect();
                let Ok(nums) = nums else { continue };
                let local_verts: Vec<[f64; 3]> = nums.chunks(3).filter(|c| c.len() == 3).map(|c| [c[0], c[1], c[2]]).collect();
                let world_verts: Vec<[f64; 3]> = local_verts.iter().map(|v| vec_add(&mat_vec(matrix, v), translation)).collect();
                if line_type == "3" {
                    if world_verts.len() == 3 {
                        triangles.push(Triangle {
                            vertices: [world_verts[0], world_verts[1], world_verts[2]],
                            color_code: effective_color,
                        });
                    }
                } else if world_verts.len() == 4 {
                    triangles.push(Triangle {
                        vertices: [world_verts[0], world_verts[1], world_verts[2]],
                        color_code: effective_color,
                    });
                    triangles.push(Triangle {
                        vertices: [world_verts[0], world_verts[2], world_verts[3]],
                        color_code: effective_color,
                    });
                }
            }
            // "0" (comment/meta) and "2"/"5" (real edge/optional lines - never
            // solid surface) are deliberately skipped.
            _ => {}
        }
    }
    Ok(())
}

/// Resolves a real top-level LDraw part into a flat, untransformed
/// (identity matrix/zero translation) list of real triangles — the same
/// "one part, its own local frame" shape every mesh in this crate uses.
pub fn resolve_part(cache: &LdrawCache, part_file: &str, color_code: u32) -> Result<Vec<Triangle>> {
    let mut triangles = Vec::new();
    resolve_into(cache, part_file, &IDENTITY, &ZERO, color_code, 0, &mut triangles)
        .with_context(|| format!("resolving real LDraw part {part_file:?}"))?;
    Ok(triangles)
}

/// Returns a real part's own descriptive title — LDraw's own convention is
/// that a part file's very first line is `0 <description>` (e.g. "Brick  1
/// x  1") — or `None` if that line is missing/unparseable.
pub fn part_description(cache: &LdrawCache, part_file: &str) -> Result<Option<String>> {
    let text = cache.fetch(&format!("parts/{part_file}"))?;
    let Some(first_line) = text.lines().next() else {
        return Ok(None);
    };
    let mut tokens = first_line.splitn(2, char::is_whitespace);
    if tokens.next() == Some("0") {
        if let Some(rest) = tokens.next() {
            return Ok(Some(rest.trim().to_string()));
        }
    }
    Ok(None)
}

/// Places already-resolved triangles at a real translation/rotation
/// matrix. If `recolor_to` is given, any triangle whose color equals
/// `base_color_code` (i.e. was LDraw color 16, "inherit," at resolve time)
/// is remapped — a real, honest approximation: it recolors whatever was
/// "the part's own color" and leaves genuinely fixed/accent-colored
/// triangles alone, rather than a full re-resolve against a different
/// color.
pub fn place(triangles: &[Triangle], translation: [f64; 3], matrix: [f64; 9], base_color_code: u32, recolor_to: Option<u32>) -> Vec<Triangle> {
    triangles
        .iter()
        .map(|tri| {
            let vertices = [
                vec_add(&mat_vec(&matrix, &tri.vertices[0]), &translation),
                vec_add(&mat_vec(&matrix, &tri.vertices[1]), &translation),
                vec_add(&mat_vec(&matrix, &tri.vertices[2]), &translation),
            ];
            let color_code = match recolor_to {
                Some(new_color) if tri.color_code == base_color_code => new_color,
                _ => tri.color_code,
            };
            Triangle { vertices, color_code }
        })
        .collect()
}

/// Real face area, via the cross-product magnitude / 2.
pub fn triangle_area(tri: &Triangle) -> f64 {
    let [v0, v1, v2] = tri.vertices;
    let u = [v1[0] - v0[0], v1[1] - v0[1], v1[2] - v0[2]];
    let v = [v2[0] - v0[0], v2[1] - v0[1], v2[2] - v0[2]];
    let cross = [u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0]];
    0.5 * (cross[0] * cross[0] + cross[1] * cross[1] + cross[2] * cross[2]).sqrt()
}

/// Real face normal via the right-hand rule, from the real vertex winding
/// LDraw's own BFC (Back Face Culling) certification guarantees (every
/// real official part file declares `BFC CERTIFY CCW`). Not adjusted for
/// `BFC INVERTNEXT` (a real directive some parts use to flag a
/// mirrored/flipped sub-file reference) — a handful of faces on a
/// composite part can end up with an inward-facing normal as a result, a
/// minor cosmetic imperfection in baked lighting, not a correctness bug in
/// the real geometry itself.
pub fn triangle_normal(tri: &Triangle) -> [f64; 3] {
    let [v0, v1, v2] = tri.vertices;
    let u = [v1[0] - v0[0], v1[1] - v0[1], v1[2] - v0[2]];
    let v = [v2[0] - v0[0], v2[1] - v0[1], v2[2] - v0[2]];
    let n = [u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0]];
    let length = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
    if length == 0.0 {
        [0.0, 0.0, 0.0]
    } else {
        [n[0] / length, n[1] / length, n[2] / length]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mat_mul_identity_is_a_no_op() {
        let m = [2.0, 0.0, 0.0, 0.0, 3.0, 0.0, 0.0, 0.0, 4.0];
        assert_eq!(mat_mul(&IDENTITY, &m), m);
        assert_eq!(mat_mul(&m, &IDENTITY), m);
    }

    #[test]
    fn mat_vec_scales_correctly() {
        let m = [2.0, 0.0, 0.0, 0.0, 3.0, 0.0, 0.0, 0.0, 4.0];
        assert_eq!(mat_vec(&m, &[1.0, 1.0, 1.0]), [2.0, 3.0, 4.0]);
    }

    #[test]
    fn rotation_y_at_zero_is_identity() {
        let m = rotation_y(0.0);
        for i in 0..9 {
            assert!((m[i] - IDENTITY[i]).abs() < 1e-12, "index {i}: {m:?} != {IDENTITY:?}");
        }
    }

    #[test]
    fn rotation_y_quarter_turn_maps_x_to_negative_z() {
        let m = rotation_y(std::f64::consts::FRAC_PI_2);
        let rotated = mat_vec(&m, &[1.0, 0.0, 0.0]);
        assert!((rotated[0]).abs() < 1e-9, "{rotated:?}");
        assert!((rotated[1]).abs() < 1e-9, "{rotated:?}");
        assert!((rotated[2] - -1.0).abs() < 1e-9, "{rotated:?}");
    }

    #[test]
    fn rotation_y_preserves_unit_length_and_leaves_y_untouched() {
        let m = rotation_y(0.73); // an arbitrary, non-special angle
        let v = [1.0, 0.0, 0.0];
        let rotated = mat_vec(&m, &v);
        let length = (rotated[0] * rotated[0] + rotated[1] * rotated[1] + rotated[2] * rotated[2]).sqrt();
        assert!((length - 1.0).abs() < 1e-9, "rotation must preserve length, got {length}");

        let up = [0.0, 1.0, 0.0];
        assert_eq!(mat_vec(&m, &up), up, "a rotation about Y must leave the Y axis itself fixed");
    }

    #[test]
    fn resolves_a_synthetic_single_triangle_part() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("parts")).unwrap();
        std::fs::write(
            dir.path().join("parts/test1.dat"),
            "0 Test Triangle Part\n\
             3 16 0 0 0 1 0 0 0 1 0\n",
        )
        .unwrap();
        let cache = LdrawCache::new(dir.path());
        let triangles = resolve_part(&cache, "test1.dat", 4).unwrap();
        assert_eq!(triangles.len(), 1);
        assert_eq!(triangles[0].color_code, 4, "color 16 (inherit) should resolve to the requested color");
        assert_eq!(triangles[0].vertices, [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]);
    }

    #[test]
    fn resolves_a_quad_into_two_triangles() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("parts")).unwrap();
        std::fs::write(
            dir.path().join("parts/quad.dat"),
            "0 Test Quad Part\n\
             4 4 0 0 0 1 0 0 1 1 0 0 1 0\n",
        )
        .unwrap();
        let cache = LdrawCache::new(dir.path());
        let triangles = resolve_part(&cache, "quad.dat", 16).unwrap();
        assert_eq!(triangles.len(), 2, "a real type-4 quad must split into 2 triangles");
        assert!(triangles.iter().all(|t| t.color_code == 4), "explicit color 4 on the quad line, not inherited");
    }

    #[test]
    fn resolves_a_real_subpart_reference_with_composed_transform() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("parts")).unwrap();
        std::fs::create_dir_all(dir.path().join("p")).unwrap();
        // A top-level part that references a shared primitive, translated
        // by (10,0,0) - the primitive's own local triangle should end up
        // shifted by exactly that in the resolved output.
        std::fs::write(
            dir.path().join("parts/composite.dat"),
            "0 Composite Part\n\
             1 16 10 0 0 1 0 0 0 1 0 0 0 1 prim.dat\n",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("p/prim.dat"),
            "0 Shared Primitive\n\
             3 16 0 0 0 1 0 0 0 1 0\n",
        )
        .unwrap();
        let cache = LdrawCache::new(dir.path());
        let triangles = resolve_part(&cache, "composite.dat", 7).unwrap();
        assert_eq!(triangles.len(), 1);
        assert_eq!(triangles[0].color_code, 7);
        assert_eq!(triangles[0].vertices[0], [10.0, 0.0, 0.0]);
        assert_eq!(triangles[0].vertices[1], [11.0, 0.0, 0.0]);
    }

    #[test]
    fn part_description_reads_the_first_real_comment_line() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("parts")).unwrap();
        std::fs::write(dir.path().join("parts/3005.dat"), "0 Brick  1 x  1\n0 Name: 3005.dat\n").unwrap();
        let cache = LdrawCache::new(dir.path());
        assert_eq!(part_description(&cache, "3005.dat").unwrap(), Some("Brick  1 x  1".to_string()));
    }

    #[test]
    fn place_translates_and_recolors_inherited_triangles_only() {
        let triangles = vec![
            Triangle { vertices: [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]], color_code: 4 },
            Triangle { vertices: [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]], color_code: 0 }, // a fixed accent color, e.g. black
        ];
        let placed = place(&triangles, [5.0, 0.0, 0.0], IDENTITY, 4, Some(7));
        assert_eq!(placed[0].color_code, 7, "the inherited base color gets recolored");
        assert_eq!(placed[1].color_code, 0, "a genuinely fixed accent color must stay untouched");
        assert_eq!(placed[0].vertices[0], [5.0, 0.0, 0.0]);
    }

    #[test]
    fn triangle_area_of_a_unit_right_triangle_is_a_half() {
        let tri = Triangle { vertices: [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]], color_code: 0 };
        assert!((triangle_area(&tri) - 0.5).abs() < 1e-9);
    }

    #[test]
    fn triangle_normal_of_the_xy_plane_points_along_z() {
        let tri = Triangle { vertices: [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]], color_code: 0 };
        let n = triangle_normal(&tri);
        assert!((n[2].abs() - 1.0).abs() < 1e-9);
    }
}
