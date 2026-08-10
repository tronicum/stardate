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
    // A top-level name is almost always a part, so `parts/` is tried first
    // and costs one cache hit. But it is not *only* ever a part: a real
    // LDraw resolver searches p/, parts/ and parts/s/ for any reference,
    // and a hand-authored scene is entitled to name a primitive directly —
    // which is what a scene composed of `box5.dat` and `stug-2x2.dat` does.
    // Before this fell back, such a scene failed with a bare 404 on
    // `official/parts/box5.dat`, which reads as "the library is missing a
    // part" rather than "the resolver looked in one folder".
    let text = if depth == 0 {
        match cache.fetch(&format!("parts/{part_file}")) {
            Ok(text) => text,
            Err(_) => resolve_ref_path(cache, part_file)?.1,
        }
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
    // Same fallback as `resolve_into`: a scene may legally name a primitive,
    // and a primitive has a description line like everything else.
    let text = match cache.fetch(&format!("parts/{part_file}")) {
        Ok(text) => text,
        Err(_) => resolve_ref_path(cache, part_file)?.1,
    };
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

// ---------------------------------------------------------------------------
// M51 — full resolution: BFC-correct winding, real edges, real provenance.
//
// `resolve_part` above is deliberately untouched. Every existing caller (the
// point pipeline, `brick.rs`, every shipped demo) keeps producing byte-
// identical output; this is a second, additive entry point.
//
// Three things it does that `resolve_part` does not:
//   1. composes BFC winding (see `bfc.rs`) so normals come out pointing
//      outward without a per-caller correction;
//   2. keeps type-2 and type-5 lines instead of discarding them;
//   3. records, per triangle and per edge, which real LDraw file it came
//      from — which is what lets a LOD pass drop studs and tubes by
//      reference path rather than by guessing.
//
// It also does NOT bake colour. LDraw code 16 ("inherit") is preserved as
// `None`, so one resolved part can be instanced in any colour without being
// resolved again. Baking it here would defeat instancing at Atlas scale.
//
// Coordinates stay in LDraw's own frame: LDU, Y-down. The conversion to
// spex's mm/Y-up frame is a *mirror* and must reverse triangle winding to
// stay consistent with the normals established here — that belongs at the
// bundle boundary (M52), in exactly one place. See `bfc.rs`'s module doc.
// ---------------------------------------------------------------------------

use crate::bfc::BfcState;
use crate::edges::{Edge, EdgeKind};

/// A real triangle with its colour left unresolved and its origin recorded.
#[derive(Clone, Debug, PartialEq)]
pub struct FullTriangle {
    pub vertices: [[f64; 3]; 3],
    /// `None` = LDraw colour 16, "inherit" — resolved per instance.
    pub color_code: Option<u32>,
    /// Index into `PartGeometry::sources`.
    pub source: u16,
}

/// Everything M52 needs from one real part: geometry, lines, provenance.
#[derive(Clone, Debug, Default)]
pub struct PartGeometry {
    pub triangles: Vec<FullTriangle>,
    pub edges: Vec<Edge>,
    /// Every real LDraw **reference path** that contributed, in first-seen
    /// order. A triangle's or edge's `source` indexes into this.
    ///
    /// The *path*, not the file: `parts/3010.dat > p/stug-1x4.dat > p/stud.dat`
    /// rather than `p/stud.dat`. That distinction is what M59's LOD1 stands
    /// on. `p/4-4cyli.dat` is a quarter-cylinder primitive, and the same file
    /// is used for a stud, for an underside tube, and for a hole in a
    /// technic beam — the leaf name cannot tell them apart. The chain can:
    /// a cylinder *reached through* a stud is a stud. Review 01, finding B5,
    /// which is exactly the "gate on the reference path, never on a
    /// heuristic about geometry" rule.
    pub sources: Vec<String>,
    /// The part's own title — LDraw's convention is that a file's first line
    /// is `0 <description>`.
    pub description: Option<String>,
    /// `0 !LICENSE ...`, carried through so a mesh bundle can state its own
    /// terms rather than the build asserting them.
    pub license: Option<String>,
    /// `0 Author: ...`
    pub author: Option<String>,
    /// Files that never declared `BFC CERTIFY`. Real official parts are
    /// certified essentially without exception, so a non-empty list here is
    /// worth looking at rather than assuming.
    pub uncertified: Vec<String>,
}

impl PartGeometry {
    pub fn conditional_edge_count(&self) -> usize {
        self.edges.iter().filter(|e| e.is_conditional()).count()
    }
    pub fn hard_edge_count(&self) -> usize {
        self.edges.len() - self.conditional_edge_count()
    }
}

struct FullCtx<'a> {
    cache: &'a LdrawCache,
    geo: PartGeometry,
    source_index: std::collections::HashMap<String, u16>,
}

impl FullCtx<'_> {
    fn intern(&mut self, name: &str) -> u16 {
        if let Some(i) = self.source_index.get(name) {
            return *i;
        }
        let i = u16::try_from(self.geo.sources.len()).unwrap_or(u16::MAX);
        self.geo.sources.push(name.to_string());
        self.source_index.insert(name.to_string(), i);
        i
    }
}

/// Effective colour for a face/line: an explicit code wins, code 16 inherits
/// from whatever the referencing chain carried (which may itself be `None`,
/// meaning the instance decides).
fn effective_color(code: u32, inherited: Option<u32>) -> Option<u32> {
    if code == 16 {
        inherited
    } else {
        Some(code)
    }
}

#[allow(clippy::too_many_arguments)]
fn resolve_full_into(
    ctx: &mut FullCtx,
    part_file: &str,
    matrix: &[f64; 9],
    translation: &[f64; 3],
    inherited_color: Option<u32>,
    depth: u32,
    inherited_reversed: bool,
    top_level: bool,
    parent_chain: &str,
) -> Result<()> {
    if depth > 8 {
        bail!("LDraw reference recursion too deep at {part_file:?} - likely a real cycle or bug");
    }
    // `parts/` first, then the same search any real LDraw resolver does —
    // see `resolve_into`. A hand-authored scene may name a primitive
    // directly, and before this fell back it failed with a bare 404 on
    // `official/parts/box5.dat`, which reads as a missing part rather than
    // as a resolver that looked in one folder.
    let (path, text) = if top_level {
        let p = format!("parts/{part_file}");
        match ctx.cache.fetch(&p) {
            Ok(t) => (p, t),
            Err(_) => resolve_ref_path(ctx.cache, part_file)?,
        }
    } else {
        resolve_ref_path(ctx.cache, part_file)?
    };
    // The reference *chain*, not just this file: see `PartGeometry::sources`.
    let chain = if parent_chain.is_empty() { path.clone() } else { format!("{parent_chain} > {path}") };
    let source = ctx.intern(&chain);
    let mut bfc = BfcState::new(inherited_reversed);
    let mut first_comment_seen = false;

    for line in text.lines() {
        let tokens: Vec<&str> = line.split_whitespace().collect();
        let Some(&line_type) = tokens.first() else {
            continue;
        };
        match line_type {
            "0" => {
                bfc.apply_meta(&tokens);
                if top_level {
                    let rest = line.trim_start().trim_start_matches('0').trim();
                    if let Some(v) = rest.strip_prefix("Author:") {
                        ctx.geo.author = Some(v.trim().to_string());
                    } else if let Some(v) = rest.strip_prefix("!LICENSE") {
                        ctx.geo.license = Some(v.trim().to_string());
                    } else if !first_comment_seen
                        && !rest.is_empty()
                        && !rest.starts_with('!')
                        && !rest.starts_with("//")
                        && !rest.starts_with("Name:")
                        && !rest.starts_with("BFC")
                    {
                        ctx.geo.description = Some(rest.to_string());
                        first_comment_seen = true;
                    }
                }
            }
            "1" => {
                if tokens.len() < 15 {
                    continue;
                }
                let sub_color: u32 = tokens[1].parse().unwrap_or(16);
                let Ok(nums) = tokens[2..14].iter().map(|t| t.parse::<f64>()).collect::<Result<Vec<f64>, _>>() else {
                    continue;
                };
                let sub_translation = [nums[0], nums[1], nums[2]];
                let sub_matrix: [f64; 9] = nums[3..12].try_into().unwrap();
                let child_reversed = bfc.winding_for_reference(&sub_matrix);
                let new_matrix = mat_mul(matrix, &sub_matrix);
                let new_translation = vec_add(&mat_vec(matrix, &sub_translation), translation);
                let sub_file = tokens[14..].join(" ");
                resolve_full_into(
                    ctx,
                    &sub_file,
                    &new_matrix,
                    &new_translation,
                    effective_color(sub_color, inherited_color),
                    depth + 1,
                    child_reversed,
                    false,
                    &chain,
                )?;
            }
            "3" | "4" => {
                if tokens.len() < 2 {
                    continue;
                }
                let face_code: u32 = tokens[1].parse().unwrap_or(16);
                let Ok(nums) = tokens[2..].iter().map(|t| t.parse::<f64>()).collect::<Result<Vec<f64>, _>>() else {
                    continue;
                };
                let verts: Vec<[f64; 3]> = nums
                    .chunks(3)
                    .filter(|c| c.len() == 3)
                    .map(|c| vec_add(&mat_vec(matrix, &[c[0], c[1], c[2]]), translation))
                    .collect();
                let needed = if line_type == "3" { 3 } else { 4 };
                if verts.len() < needed {
                    continue;
                }
                let color_code = effective_color(face_code, inherited_color);
                let reversed = bfc.face_reversed();
                let mut push = |a: usize, b: usize, c: usize| {
                    let v = if reversed {
                        [verts[c], verts[b], verts[a]]
                    } else {
                        [verts[a], verts[b], verts[c]]
                    };
                    ctx.geo.triangles.push(FullTriangle { vertices: v, color_code, source });
                };
                push(0, 1, 2);
                if line_type == "4" {
                    push(0, 2, 3);
                }
            }
            "2" => {
                if tokens.len() < 8 {
                    continue;
                }
                let code: u32 = tokens[1].parse().unwrap_or(16);
                let Ok(nums) = tokens[2..8].iter().map(|t| t.parse::<f64>()).collect::<Result<Vec<f64>, _>>() else {
                    continue;
                };
                let a = vec_add(&mat_vec(matrix, &[nums[0], nums[1], nums[2]]), translation);
                let b = vec_add(&mat_vec(matrix, &[nums[3], nums[4], nums[5]]), translation);
                ctx.geo.edges.push(Edge {
                    vertices: [a, b],
                    color_code: effective_color(code, inherited_color),
                    kind: EdgeKind::Hard,
                    source,
                });
            }
            "5" => {
                if tokens.len() < 14 {
                    continue;
                }
                let code: u32 = tokens[1].parse().unwrap_or(16);
                let Ok(nums) = tokens[2..14].iter().map(|t| t.parse::<f64>()).collect::<Result<Vec<f64>, _>>() else {
                    continue;
                };
                let p: Vec<[f64; 3]> = (0..4)
                    .map(|i| {
                        vec_add(
                            &mat_vec(matrix, &[nums[i * 3], nums[i * 3 + 1], nums[i * 3 + 2]]),
                            translation,
                        )
                    })
                    .collect();
                // The control points are transformed by the same composed
                // matrix as the endpoints; a conditional edge whose controls
                // were left untransformed would test against the wrong
                // geometry and flicker.
                ctx.geo.edges.push(Edge {
                    vertices: [p[0], p[1]],
                    color_code: effective_color(code, inherited_color),
                    kind: EdgeKind::Conditional { control: [p[2], p[3]] },
                    source,
                });
            }
            _ => {}
        }
    }

    if !bfc.certified {
        ctx.geo.uncertified.push(path);
    }
    Ok(())
}

/// Resolves one real top-level LDraw part into triangles **and** edges, with
/// BFC-correct winding, unresolved colour, and per-primitive provenance.
///
/// Output stays in LDraw's native frame (LDU, Y-down). See this section's
/// header for why that matters.
pub fn resolve_part_full(cache: &LdrawCache, part_file: &str) -> Result<PartGeometry> {
    let mut ctx = FullCtx {
        cache,
        geo: PartGeometry::default(),
        source_index: std::collections::HashMap::new(),
    };
    resolve_full_into(&mut ctx, part_file, &IDENTITY, &ZERO, None, 0, false, true, "")
        .with_context(|| format!("fully resolving real LDraw part {part_file:?}"))?;
    Ok(ctx.geo)
}

/// Right-hand-rule normal of a `FullTriangle` — the same rule as
/// `triangle_normal`, which is correct here precisely because
/// `resolve_part_full` already reversed the vertex order wherever BFC said to.
pub fn full_triangle_normal(tri: &FullTriangle) -> [f64; 3] {
    triangle_normal(&Triangle { vertices: tri.vertices, color_code: 0 })
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

    // ---------------------------------------------------------------- M51 ---

    /// Writes a synthetic library and returns a cache over it.
    fn fixture(files: &[(&str, &str)]) -> (tempfile::TempDir, LdrawCache) {
        let dir = tempfile::tempdir().unwrap();
        for (path, body) in files {
            let full = dir.path().join(path);
            std::fs::create_dir_all(full.parent().unwrap()).unwrap();
            std::fs::write(full, body).unwrap();
        }
        let cache = LdrawCache::new(dir.path());
        (dir, cache)
    }

    fn positions(tris: &[FullTriangle]) -> Vec<[u64; 3]> {
        let mut v: Vec<[u64; 3]> = tris
            .iter()
            .flat_map(|t| t.vertices)
            .map(|p| [p[0].to_bits(), p[1].to_bits(), p[2].to_bits()])
            .collect();
        v.sort_unstable();
        v
    }

    #[test]
    fn full_resolution_leaves_colour_16_unresolved_and_keeps_explicit_codes() {
        let (_d, cache) = fixture(&[(
            "parts/c.dat",
            "0 Colour Test\n0 BFC CERTIFY CCW\n\
             3 16 0 0 0 1 0 0 0 1 0\n\
             3 4 0 0 0 1 0 0 0 1 0\n",
        )]);
        let g = resolve_part_full(&cache, "c.dat").unwrap();
        assert_eq!(g.triangles[0].color_code, None, "16 must stay unresolved for per-instance colour");
        assert_eq!(g.triangles[1].color_code, Some(4), "an explicit accent colour is kept");
    }

    #[test]
    fn an_explicit_colour_on_a_reference_is_inherited_by_its_faces() {
        let (_d, cache) = fixture(&[
            ("parts/outer.dat", "0 Outer\n0 BFC CERTIFY CCW\n1 7 0 0 0 1 0 0 0 1 0 0 0 1 inner.dat\n"),
            ("p/inner.dat", "0 Inner\n0 BFC CERTIFY CCW\n3 16 0 0 0 1 0 0 0 1 0\n"),
        ]);
        let g = resolve_part_full(&cache, "outer.dat").unwrap();
        assert_eq!(g.triangles[0].color_code, Some(7));
    }

    #[test]
    fn type_2_and_type_5_lines_are_kept_and_their_control_points_transformed() {
        let (_d, cache) = fixture(&[
            ("parts/e.dat", "0 Edges\n0 BFC CERTIFY CCW\n1 16 10 0 0 1 0 0 0 1 0 0 0 1 lines.dat\n"),
            (
                "p/lines.dat",
                "0 Lines\n\
                 2 24 0 0 0 1 0 0\n\
                 5 24 0 0 0 1 0 0 0 1 0 0 -1 0\n",
            ),
        ]);
        let g = resolve_part_full(&cache, "e.dat").unwrap();
        assert_eq!(g.hard_edge_count(), 1);
        assert_eq!(g.conditional_edge_count(), 1);
        let hard = g.edges.iter().find(|e| !e.is_conditional()).unwrap();
        assert_eq!(hard.vertices[0], [10.0, 0.0, 0.0], "the reference's translation applies");
        let cond = g.edges.iter().find(|e| e.is_conditional()).unwrap();
        match cond.kind {
            EdgeKind::Conditional { control } => {
                assert_eq!(control[0], [10.0, 1.0, 0.0], "control points move with the edge");
                assert_eq!(control[1], [10.0, -1.0, 0.0]);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn invertnext_reverses_exactly_one_reference_subtree() {
        let body = "0 Prim\n0 BFC CERTIFY CCW\n3 16 0 0 0 1 0 0 0 1 0\n";
        let (_d, cache) = fixture(&[
            (
                "parts/inv.dat",
                "0 Invert Test\n0 BFC CERTIFY CCW\n\
                 0 BFC INVERTNEXT\n\
                 1 16 0 0 0 1 0 0 0 1 0 0 0 1 prim.dat\n\
                 1 16 0 0 0 1 0 0 0 1 0 0 0 1 prim.dat\n",
            ),
            ("p/prim.dat", body),
        ]);
        let g = resolve_part_full(&cache, "inv.dat").unwrap();
        assert_eq!(g.triangles.len(), 2);
        let a = full_triangle_normal(&g.triangles[0]);
        let b = full_triangle_normal(&g.triangles[1]);
        assert!((a[2] + b[2]).abs() < 1e-12, "the flagged one faces the other way: {a:?} vs {b:?}");
        assert!(a[2] * b[2] < 0.0, "and only the flagged one");
    }

    /// A mirrored reference must come out with its outward normal still
    /// pointing outward. That is the *whole* job of the determinant check: a
    /// negative-determinant matrix flips the geometry's handedness, so the
    /// stored winding has to flip too, and the two cancel. Without the check,
    /// the mirrored copy would face inward — which is invisible until the
    /// part is lit and culled, and is exactly the class of defect
    /// `scripts/mesh-vs-points-spike/` caught in the coordinate conversion.
    #[test]
    fn a_mirrored_reference_still_faces_outward() {
        let body = "0 Prim\n0 BFC CERTIFY CCW\n3 16 0 0 0 1 0 0 0 1 0\n";
        let (_d, cache) = fixture(&[
            (
                "parts/mir.dat",
                "0 Mirror Test\n0 BFC CERTIFY CCW\n\
                 1 16 0 0 0 1 0 0 0 1 0 0 0 1 prim.dat\n\
                 1 16 0 0 0 -1 0 0 0 1 0 0 0 1 prim.dat\n",
            ),
            ("p/prim.dat", body),
        ]);
        let g = resolve_part_full(&cache, "mir.dat").unwrap();
        let plain = full_triangle_normal(&g.triangles[0]);
        let mirrored = full_triangle_normal(&g.triangles[1]);
        assert!(plain[2] > 0.9, "the unmirrored face points +Z: {plain:?}");
        assert!(
            mirrored[2] > 0.9,
            "so must the mirrored one — the winding flip cancels the handedness flip: {mirrored:?}"
        );
        // And the geometry really was mirrored — the fixture negates X, so
        // the (1,0,0) corner has to come back as (-1,0,0). Without this the
        // normal assertions above would pass even if the matrix were ignored.
        assert_eq!(
            g.triangles[1].vertices.iter().filter(|v| v[0] < 0.0).count(),
            1,
            "got {:?}",
            g.triangles[1].vertices
        );
    }

    #[test]
    fn a_cw_certified_file_has_its_faces_reversed() {
        let (_d, cache) = fixture(&[
            ("parts/cw.dat", "0 CW\n0 BFC CERTIFY CW\n3 16 0 0 0 1 0 0 0 1 0\n"),
            ("parts/ccw.dat", "0 CCW\n0 BFC CERTIFY CCW\n3 16 0 0 0 1 0 0 0 1 0\n"),
        ]);
        let cw = resolve_part_full(&cache, "cw.dat").unwrap();
        let ccw = resolve_part_full(&cache, "ccw.dat").unwrap();
        let a = full_triangle_normal(&cw.triangles[0]);
        let b = full_triangle_normal(&ccw.triangles[0]);
        assert!(a[2] * b[2] < 0.0);
    }

    #[test]
    fn provenance_is_recorded_per_primitive() {
        let (_d, cache) = fixture(&[
            (
                "parts/p.dat",
                "0 Provenance\n0 BFC CERTIFY CCW\n0 Author: A Real Author\n\
                 0 !LICENSE Redistributable under CCAL version 2.0\n\
                 3 16 0 0 0 1 0 0 0 1 0\n\
                 1 16 0 0 0 1 0 0 0 1 0 0 0 1 stud.dat\n",
            ),
            ("p/stud.dat", "0 Stud\n0 BFC CERTIFY CCW\n3 16 0 0 0 1 0 0 0 1 0\n2 24 0 0 0 1 0 0\n"),
        ]);
        let g = resolve_part_full(&cache, "p.dat").unwrap();
        assert_eq!(g.description.as_deref(), Some("Provenance"));
        assert_eq!(g.author.as_deref(), Some("A Real Author"));
        assert_eq!(g.license.as_deref(), Some("Redistributable under CCAL version 2.0"));
        assert_eq!(
            g.sources,
            vec!["parts/p.dat".to_string(), "parts/p.dat > p/stud.dat".to_string()],
            "sources record the reference *chain*, which is what LOD1 gates on"
        );
        assert_eq!(g.triangles[0].source, 0, "the part's own face");
        assert_eq!(g.triangles[1].source, 1, "the stud's face — this is what M59 gates LOD on");
        assert_eq!(g.edges[0].source, 1);
        assert!(g.uncertified.is_empty());
    }

    #[test]
    fn an_uncertified_file_is_reported_rather_than_silently_assumed() {
        let (_d, cache) = fixture(&[("parts/u.dat", "0 No BFC here\n3 16 0 0 0 1 0 0 0 1 0\n")]);
        let g = resolve_part_full(&cache, "u.dat").unwrap();
        assert_eq!(g.uncertified, vec!["parts/u.dat".to_string()]);
    }

    #[test]
    fn full_resolution_and_resolve_part_agree_on_the_geometry_itself() {
        // The point of M51 is that it is ADDITIVE. `resolve_part` keeps
        // producing what it always produced; the two must describe the same
        // surface. Compared as an unordered set of vertex positions, because
        // full resolution deliberately reverses winding where BFC says to —
        // and a reversal is not a rotation of the vertex order.
        let (_d, cache) = fixture(&[
            (
                "parts/same.dat",
                "0 Same\n0 BFC CERTIFY CCW\n\
                 4 16 0 0 0 1 0 0 1 1 0 0 1 0\n\
                 0 BFC INVERTNEXT\n\
                 1 16 5 0 0 1 0 0 0 1 0 0 0 1 prim.dat\n\
                 1 4 0 0 5 1 0 0 0 1 0 0 0 1 prim.dat\n\
                 2 24 0 0 0 1 0 0\n",
            ),
            ("p/prim.dat", "0 Prim\n0 BFC CERTIFY CCW\n3 16 0 0 0 1 0 0 0 1 0\n2 24 0 0 0 0 1 0\n"),
        ]);
        let plain = resolve_part(&cache, "same.dat", 16).unwrap();
        let full = resolve_part_full(&cache, "same.dat").unwrap();
        assert_eq!(plain.len(), full.triangles.len(), "same triangle count");
        let plain_full: Vec<FullTriangle> = plain
            .iter()
            .map(|t| FullTriangle { vertices: t.vertices, color_code: None, source: 0 })
            .collect();
        assert_eq!(positions(&plain_full), positions(&full.triangles), "same surface");
        assert_eq!(full.hard_edge_count(), 3, "and the edges resolve_part throws away");
    }

    #[test]
    #[ignore = "real live network fetch against ldraw.org, not run by default"]
    fn real_1x1_brick_resolves_with_edges_and_no_uncertified_files() {
        let dir = tempfile::tempdir().unwrap();
        let cache = LdrawCache::new(dir.path());
        let g = resolve_part_full(&cache, "3005.dat").unwrap();
        // Numbers measured by scripts/mesh-vs-points-spike/ against the real
        // library; they are a regression guard, not a specification.
        assert_eq!(g.triangles.len(), 76);
        assert_eq!(g.hard_edge_count(), 56);
        assert_eq!(g.conditional_edge_count(), 16);
        assert!(g.uncertified.is_empty(), "real official parts are BFC-certified: {:?}", g.uncertified);
        assert_eq!(g.description.as_deref(), Some("Brick  1 x  1"));
    }
}
