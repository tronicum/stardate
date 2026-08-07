//! M65 — a companion point cloud for every part, so the show can cross
//! between its own two render modes on screen.
//!
//! Act I opens on a point that becomes a brick; Act IV ends with everything
//! becoming points again. Both need the *same object* in both
//! representations, which means the points have to be sampled from the real
//! resolved surface rather than approximated from a bounding box — and, since
//! the crossfade is watched at close range, sampled evenly rather than
//! clumped.
//!
//! # Colour-neutral, exactly like the mesh
//!
//! The spec asked for `positions + baked colour`, reusing the octree's
//! 15-bytes-per-point layout so as not to introduce a format. **That cannot
//! work here.** A part's geometry is deliberately colour-neutral — M51 leaves
//! LDraw code 16 unresolved precisely so one mesh serves every colour it is
//! placed in — and baking colour into a part's point cloud would mean one
//! cloud per (part, colour). That is the entire instancing argument thrown
//! away at the one moment the scene is most expensive: Act IV's *Inkpour* is
//! every brick on screen at once.
//!
//! So a point carries **position and normal**, and its colour comes from the
//! instance's material at draw time, the same way the mesh's does. The normal
//! is not optional decoration either: the spec's own "points lerping outward
//! along their own normals" needs it, and the octree layout has nowhere to
//! put it.
//!
//! # Deterministic without a PRNG
//!
//! Sampling is by **golden-ratio stratification**: an additive recurrence
//! picks the triangle against the cumulative-area table, and a 2-D Halton
//! sequence places the point inside it. No seed, no generator, no
//! cross-version stability question of the kind M64 had to solve — and a
//! visibly more even cloud than uniform random, which clumps. For a swarm
//! that stands in for a solid object, clumping reads as holes.

use crate::weld::WeldedMesh;

/// Points per square millimetre of real surface.
///
/// A 1x1 brick is about 570 mm² of surface, so this gives it ~1 100 points —
/// close to the ~3 000 the screenplay asks for A1-S02's swarm once the
/// underside and tube are counted, and small enough that a 5 000-brick site
/// is not a hundred million vertices.
pub const POINT_DENSITY_PER_MM2: f64 = 2.0;
/// Floor and ceiling. The floor keeps a 2-triangle tile from becoming four
/// points; the ceiling keeps a windscreen from dominating a scene's budget.
pub const MIN_POINTS: usize = 128;
pub const MAX_POINTS: usize = 4096;

/// One sampled surface point, already in the **output** frame: millimetres,
/// +Y up, mirrored — because it is sampled from the welded output mesh, which
/// has been through `to_output_position` already.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SurfacePoint {
    pub position: [f32; 3],
    pub normal: [f32; 3],
}

/// The van der Corput sequence in a given base — one dimension of Halton.
fn van_der_corput(mut i: usize, base: usize) -> f64 {
    let mut f = 1.0;
    let mut r = 0.0;
    while i > 0 {
        f /= base as f64;
        r += f * (i % base) as f64;
        i /= base;
    }
    r
}

/// How many points a part of this surface area gets.
pub fn point_count_for_area(area_mm2: f64) -> usize {
    ((area_mm2 * POINT_DENSITY_PER_MM2).round() as usize).clamp(MIN_POINTS, MAX_POINTS)
}

/// Area-weighted, evenly stratified samples over a welded mesh's triangles.
///
/// Returns an empty vector for an empty mesh rather than failing: a part with
/// no triangles is a real thing in the LDraw library (a few files are pure
/// metadata), and it should produce a bundle with no points, not no bundle.
pub fn sample_surface(welded: &WeldedMesh) -> Vec<SurfacePoint> {
    let tri_count = welded.indices.len() / 3;
    if tri_count == 0 {
        return Vec::new();
    }

    // Cumulative area, so a triangle's chance of being picked is its share of
    // the surface. Sampling triangles uniformly would put as many points on a
    // stud's tiny facet as on a brick's whole wall.
    let mut cumulative = Vec::with_capacity(tri_count);
    let mut total = 0.0f64;
    for t in 0..tri_count {
        let [a, b, c] = triangle(welded, t);
        total += area(a, b, c);
        cumulative.push(total);
    }
    if total <= 0.0 {
        return Vec::new();
    }

    let n = point_count_for_area(total);
    let mut out = Vec::with_capacity(n);
    // The golden ratio's fractional part: the additive recurrence with the
    // lowest possible discrepancy, which is why it is the one to use when the
    // point of the exercise is *not* clumping.
    const PHI_INV: f64 = 0.618_033_988_749_894_9;
    let mut cursor = 0.5f64;

    for i in 0..n {
        cursor = (cursor + PHI_INV).fract();
        let target = cursor * total;
        // The triangle whose cumulative area first exceeds the target.
        let t = match cumulative.binary_search_by(|probe| probe.partial_cmp(&target).unwrap()) {
            Ok(k) => k,
            Err(k) => k.min(tri_count - 1),
        };

        // Barycentric, from a 2-D Halton pair folded into the triangle.
        let mut u = van_der_corput(i + 1, 2);
        let mut v = van_der_corput(i + 1, 3);
        if u + v > 1.0 {
            u = 1.0 - u;
            v = 1.0 - v;
        }
        let w = 1.0 - u - v;

        let [ia, ib, ic] = indices(welded, t);
        let pa = welded.positions[ia];
        let pb = welded.positions[ib];
        let pc = welded.positions[ic];
        let na = welded.normals[ia];
        let nb = welded.normals[ib];
        let nc = welded.normals[ic];

        let mut normal = [
            (na[0] as f64 * w + nb[0] as f64 * u + nc[0] as f64 * v) as f32,
            (na[1] as f64 * w + nb[1] as f64 * u + nc[1] as f64 * v) as f32,
            (na[2] as f64 * w + nb[2] as f64 * u + nc[2] as f64 * v) as f32,
        ];
        let len = (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
        if len > 1e-9 {
            normal = [normal[0] / len, normal[1] / len, normal[2] / len];
        } else {
            // Three welded normals that cancel: geometrically possible on a
            // degenerate sliver, and a zero-length normal would send the
            // point nowhere during the inkpour rather than outward.
            normal = [0.0, 1.0, 0.0];
        }

        out.push(SurfacePoint {
            position: [
                (pa[0] as f64 * w + pb[0] as f64 * u + pc[0] as f64 * v) as f32,
                (pa[1] as f64 * w + pb[1] as f64 * u + pc[1] as f64 * v) as f32,
                (pa[2] as f64 * w + pb[2] as f64 * u + pc[2] as f64 * v) as f32,
            ],
            normal,
        });
    }
    out
}

/// Positions + normals, interleaved, little-endian f32 — 24 bytes per point.
pub fn to_bytes(points: &[SurfacePoint]) -> Vec<u8> {
    let mut out = Vec::with_capacity(points.len() * 24);
    for p in points {
        for c in p.position {
            out.extend_from_slice(&c.to_le_bytes());
        }
        for c in p.normal {
            out.extend_from_slice(&c.to_le_bytes());
        }
    }
    out
}

fn indices(w: &WeldedMesh, t: usize) -> [usize; 3] {
    [
        w.indices[t * 3] as usize,
        w.indices[t * 3 + 1] as usize,
        w.indices[t * 3 + 2] as usize,
    ]
}

fn triangle(w: &WeldedMesh, t: usize) -> [[f32; 3]; 3] {
    let [a, b, c] = indices(w, t);
    [w.positions[a], w.positions[b], w.positions[c]]
}

fn area(a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> f64 {
    let u = [
        (b[0] - a[0]) as f64,
        (b[1] - a[1]) as f64,
        (b[2] - a[2]) as f64,
    ];
    let v = [
        (c[0] - a[0]) as f64,
        (c[1] - a[1]) as f64,
        (c[2] - a[2]) as f64,
    ];
    let cross = [
        u[1] * v[2] - u[2] * v[1],
        u[2] * v[0] - u[0] * v[2],
        u[0] * v[1] - u[1] * v[0],
    ];
    0.5 * (cross[0] * cross[0] + cross[1] * cross[1] + cross[2] * cross[2]).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A unit square in the XZ plane, two triangles, normals +Y.
    fn square(size: f32) -> WeldedMesh {
        WeldedMesh {
            positions: vec![
                [0.0, 0.0, 0.0],
                [size, 0.0, 0.0],
                [size, 0.0, size],
                [0.0, 0.0, size],
            ],
            normals: vec![[0.0, 1.0, 0.0]; 4],
            indices: vec![0, 1, 2, 0, 2, 3],
            triangle_colors: vec![None, None],
            triangle_sources: vec![0, 0],
        }
    }

    #[test]
    fn every_sample_lands_on_the_surface_it_was_sampled_from() {
        let mesh = square(10.0);
        let pts = sample_surface(&mesh);
        assert!(!pts.is_empty());
        for p in &pts {
            assert!(p.position[1].abs() < 1e-4, "off the plane: {:?}", p.position);
            assert!((0.0..=10.0).contains(&p.position[0]));
            assert!((0.0..=10.0).contains(&p.position[2]));
            assert!((p.normal[1] - 1.0).abs() < 1e-5);
        }
    }

    /// The whole reason for stratifying rather than randomising: an even
    /// cloud. Split the square into a 4x4 grid and check no cell is empty and
    /// none holds more than twice its fair share — a uniform PRNG fails the
    /// first of those regularly at this sample count.
    #[test]
    fn the_cloud_is_even_rather_than_clumped() {
        let mesh = square(10.0);
        let pts = sample_surface(&mesh);
        let mut cells = [0usize; 16];
        for p in &pts {
            let cx = ((p.position[0] / 10.0) * 4.0).floor().clamp(0.0, 3.0) as usize;
            let cz = ((p.position[2] / 10.0) * 4.0).floor().clamp(0.0, 3.0) as usize;
            cells[cz * 4 + cx] += 1;
        }
        let fair = pts.len() as f64 / 16.0;
        for (i, &c) in cells.iter().enumerate() {
            assert!(c > 0, "cell {i} is empty — the cloud has a hole in it");
            assert!(
                (c as f64) < fair * 2.0,
                "cell {i} holds {c}, more than twice its fair share of {fair:.1}"
            );
        }
    }

    /// No seed, no generator: two runs are the same run.
    #[test]
    fn sampling_is_deterministic() {
        let mesh = square(10.0);
        assert_eq!(sample_surface(&mesh), sample_surface(&mesh));
    }

    #[test]
    fn point_count_follows_area_between_the_floor_and_the_ceiling() {
        assert_eq!(point_count_for_area(1.0), MIN_POINTS, "a tiny part still gets a usable cloud");
        assert_eq!(point_count_for_area(1e9), MAX_POINTS, "a huge part is capped");
        assert_eq!(point_count_for_area(500.0), 1000);
    }

    #[test]
    fn a_mesh_with_no_triangles_yields_no_points_rather_than_failing() {
        let empty = WeldedMesh {
            positions: vec![],
            normals: vec![],
            indices: vec![],
            triangle_colors: vec![],
            triangle_sources: vec![],
        };
        assert!(sample_surface(&empty).is_empty());
    }

    #[test]
    fn the_binary_layout_is_twenty_four_bytes_a_point() {
        let pts = sample_surface(&square(10.0));
        assert_eq!(to_bytes(&pts).len(), pts.len() * 24);
    }
}
