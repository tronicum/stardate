//! Vertex welding and crease-angle normal smoothing.
//!
//! LDraw geometry arrives as a flat triangle soup: three fresh vertices per
//! face, every shared corner duplicated as many times as it is touched. A 1x1
//! brick is 76 triangles and therefore 228 unwelded vertices, for an object
//! that has far fewer distinct corners than that.
//!
//! Welding does two jobs at once. It shrinks the buffer, and — more
//! importantly — it is what makes a stud look round. A stud's cylinder is 16
//! flat facets; unwelded, each facet is flat-shaded and the cylinder reads as
//! a 16-sided prism. Averaging normals across facets whose dihedral angle is
//! small turns it back into a cylinder, while a brick's 90-degree corners stay
//! sharp because they are nowhere near the crease threshold.

/// Default crease angle in degrees. LDView uses 30, and the number sits in a
/// wide safe gap: LDraw's 16-segment primitives meet at 22.5 degrees and its
/// 48-segment ones at 7.5, so both smooth; a brick's corners are at 90, so
/// they stay sharp. Anything between about 25 and 60 behaves identically on
/// real parts — this is not a knife-edge value.
pub const DEFAULT_CREASE_DEGREES: f64 = 30.0;

/// Positions are quantised to this fraction of an LDU before being compared,
/// so that vertices meant to be the same point are welded despite the last
/// bit of floating-point drift accumulated through a deep reference chain.
/// One thousandth of an LDU is 0.4 micrometres — far below any real
/// modelling tolerance, and far above the drift.
const WELD_QUANTUM: f64 = 0.001;

#[derive(Clone, Debug, Default)]
pub struct WeldedMesh {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
    /// Per *triangle* (so `indices.len() / 3` entries): the unresolved LDraw
    /// colour, `None` meaning code 16 / inherit.
    pub triangle_colors: Vec<Option<u32>>,
    /// Per triangle: index into the source part's `sources` table.
    pub triangle_sources: Vec<u16>,
}

impl WeldedMesh {
    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }
    pub fn vertex_count(&self) -> usize {
        self.positions.len()
    }
}

fn key(p: [f64; 3]) -> [i64; 3] {
    [
        (p[0] / WELD_QUANTUM).round() as i64,
        (p[1] / WELD_QUANTUM).round() as i64,
        (p[2] / WELD_QUANTUM).round() as i64,
    ]
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// Welds coincident positions and averages normals across faces meeting below
/// `crease_degrees`.
///
/// A welded vertex may carry several *distinct* normals — a brick's top face
/// and its side wall share a corner position but must not share a normal — so
/// the output splits one position into as many vertices as it has distinct
/// smoothing groups. That is why `positions.len()` is usually a little larger
/// than the number of distinct positions, and still far smaller than the
/// unwelded count.
pub fn weld_and_smooth(
    triangles: &[spex_ldraw::FullTriangle],
    crease_degrees: f64,
) -> WeldedMesh {
    let cos_crease = crease_degrees.to_radians().cos();
    let face_normals: Vec<[f64; 3]> = triangles
        .iter()
        .map(spex_ldraw::full_triangle_normal)
        .collect();

    // position key -> the faces touching it
    let mut touching: std::collections::HashMap<[i64; 3], Vec<usize>> =
        std::collections::HashMap::new();
    for (fi, tri) in triangles.iter().enumerate() {
        for v in tri.vertices {
            touching.entry(key(v)).or_default().push(fi);
        }
    }

    let mut out = WeldedMesh::default();
    // (position key, smoothing-group representative face) -> emitted vertex
    let mut emitted: std::collections::HashMap<([i64; 3], usize), u32> =
        std::collections::HashMap::new();

    for (fi, tri) in triangles.iter().enumerate() {
        out.triangle_colors.push(tri.color_code);
        out.triangle_sources.push(tri.source);
        for v in tri.vertices {
            let k = key(v);
            let faces = &touching[&k];
            // Every face at this position within the crease angle of *this*
            // face forms its smoothing group. The lowest such face index is a
            // stable representative, so two faces in the same group agree on
            // it without any union-find.
            let mut acc = [0.0f64; 3];
            let mut rep = fi;
            for &other in faces {
                if dot(face_normals[fi], face_normals[other]) >= cos_crease {
                    acc[0] += face_normals[other][0];
                    acc[1] += face_normals[other][1];
                    acc[2] += face_normals[other][2];
                    rep = rep.min(other);
                }
            }
            let len = (acc[0] * acc[0] + acc[1] * acc[1] + acc[2] * acc[2]).sqrt();
            let n = if len > 1e-12 {
                [acc[0] / len, acc[1] / len, acc[2] / len]
            } else {
                face_normals[fi]
            };
            let idx = *emitted.entry((k, rep)).or_insert_with(|| {
                out.positions.push([v[0] as f32, v[1] as f32, v[2] as f32]);
                out.normals.push([n[0] as f32, n[1] as f32, n[2] as f32]);
                (out.positions.len() - 1) as u32
            });
            out.indices.push(idx);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use spex_ldraw::FullTriangle;

    fn tri(a: [f64; 3], b: [f64; 3], c: [f64; 3]) -> FullTriangle {
        FullTriangle { vertices: [a, b, c], color_code: None, source: 0 }
    }

    #[test]
    fn a_flat_quad_welds_from_six_vertices_to_four() {
        let quad = vec![
            tri([0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 1.0, 0.0]),
            tri([0.0, 0.0, 0.0], [1.0, 1.0, 0.0], [0.0, 1.0, 0.0]),
        ];
        let w = weld_and_smooth(&quad, DEFAULT_CREASE_DEGREES);
        assert_eq!(w.vertex_count(), 4, "the shared diagonal is welded");
        assert_eq!(w.triangle_count(), 2);
        assert_eq!(w.indices.len(), 6);
    }

    #[test]
    fn a_right_angle_stays_sharp() {
        // Two faces meeting at 90 degrees share an edge. They must NOT share
        // normals, or a brick's corners round off into soap.
        let l = vec![
            tri([0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 1.0, 0.0]),
            tri([0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 1.0]),
        ];
        let w = weld_and_smooth(&l, DEFAULT_CREASE_DEGREES);
        let n0 = w.normals[w.indices[0] as usize];
        let n3 = w.normals[w.indices[3] as usize];
        assert_ne!(n0, n3, "a 90-degree crease must split the vertex");
    }

    #[test]
    fn a_shallow_fan_smooths() {
        // Three facets of a many-sided cylinder, each ~15 degrees apart: well
        // inside the crease angle, so they share an averaged normal.
        let mut tris = Vec::new();
        for i in 0..3 {
            let a = (i as f64) * 15f64.to_radians();
            let b = ((i + 1) as f64) * 15f64.to_radians();
            tris.push(tri(
                [a.cos(), a.sin(), 0.0],
                [b.cos(), b.sin(), 0.0],
                [b.cos(), b.sin(), 1.0],
            ));
        }
        let w = weld_and_smooth(&tris, DEFAULT_CREASE_DEGREES);
        assert!(
            w.vertex_count() < 9,
            "shallow facets must share vertices, got {}",
            w.vertex_count()
        );
    }

    #[test]
    fn welding_reduces_the_buffer_and_keeps_the_triangles() {
        let mut tris = Vec::new();
        for i in 0..12 {
            let a = (i as f64) * 30f64.to_radians();
            let b = ((i + 1) as f64) * 30f64.to_radians();
            tris.push(tri([0.0, 0.0, 0.0], [a.cos(), a.sin(), 0.0], [b.cos(), b.sin(), 0.0]));
        }
        let unwelded = tris.len() * 3;
        let w = weld_and_smooth(&tris, DEFAULT_CREASE_DEGREES);
        assert_eq!(w.triangle_count(), tris.len(), "no triangle is lost");
        assert!(
            w.vertex_count() * 2 <= unwelded,
            "welding a fan should at least halve the vertices: {} -> {}",
            unwelded,
            w.vertex_count()
        );
    }

    #[test]
    fn per_triangle_colour_and_source_survive_welding() {
        let mut tris = vec![
            tri([0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 1.0, 0.0]),
            tri([0.0, 0.0, 0.0], [1.0, 1.0, 0.0], [0.0, 1.0, 0.0]),
        ];
        tris[1].color_code = Some(4);
        tris[1].source = 7;
        let w = weld_and_smooth(&tris, DEFAULT_CREASE_DEGREES);
        assert_eq!(w.triangle_colors, vec![None, Some(4)]);
        assert_eq!(w.triangle_sources, vec![0, 7]);
    }
}
