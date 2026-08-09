//! Parametric solid generation for Ankerstein shapes — the real difference
//! from `spex-ldraw::geometry`, which resolves an existing LDraw mesh
//! instead. No equivalent open mesh library exists for Ankerstein (see
//! `docs/ANKERSTEIN-ENGINE.md` §1), so a shape's geometry is generated
//! directly from its `AnkersteinShape::dimensions_mm`, emitting the same
//! `spex_ldraw::geometry::Triangle` shape so `spex-ldraw`'s own
//! `sample_surface`/`shade_color`/`to_point_cloud` can be reused unchanged
//! (see this crate's `lib.rs` doc comment).
use crate::catalog::{AnkersteinShape, ShapeType};
use spex_ldraw::geometry::Triangle;

/// Generates a real axis-aligned rectangular box (a `ShapeType::Block`),
/// centered at the origin, with the exact `dimensions_mm` given —
/// straightforward analytic geometry (12 triangles, 2 per face), unlike
/// `spex-ldraw`'s recursive mesh resolution, since there's no file to
/// resolve.
pub fn generate_box(dimensions_mm: [f64; 3], color_code: u32) -> Vec<Triangle> {
    let [w, h, d] = dimensions_mm.map(|v| v / 2.0);
    // 8 corners of the box.
    let corners = [
        [-w, -h, -d],
        [w, -h, -d],
        [w, h, -d],
        [-w, h, -d],
        [-w, -h, d],
        [w, -h, d],
        [w, h, d],
        [-w, h, d],
    ];
    // 6 faces, each as 2 triangles, CCW winding when viewed from outside.
    const FACES: [[usize; 4]; 6] = [
        [0, 1, 2, 3], // -Z
        [5, 4, 7, 6], // +Z
        [4, 0, 3, 7], // -X
        [1, 5, 6, 2], // +X
        [4, 5, 1, 0], // -Y
        [3, 2, 6, 7], // +Y
    ];
    let mut triangles = Vec::with_capacity(12);
    for face in FACES {
        let [a, b, c, e] = face.map(|i| corners[i]);
        triangles.push(Triangle { vertices: [a, b, c], color_code });
        triangles.push(Triangle { vertices: [a, c, e], color_code });
    }
    triangles
}

/// Generates a real right-triangular prism (`ShapeType::Prism`) — the
/// simplest of the real historical "prism (sloped or triangular) shaped
/// blocks" from the fifth historical set (see this module's `Prism` match
/// arm in `generate_shape` for the real citation). Cross-section is a
/// right *isosceles* triangle (both legs equal to `leg_mm`, a 45° slope),
/// extruded `depth_mm` along the third axis, centered at the origin the
/// same way `generate_box` centers a block — **a real, deliberate scope
/// limit**: the historical record cites "six different roof slopes" in
/// total (see `docs/ANKERSTEIN-ENGINE.md` M99's own note), of which this
/// is only the one whose geometry is directly and simply citable (two
/// such prisms combine into a real cube); the other five angles are real
/// but not yet reduced to cited numbers, and would need their own
/// generation function (a different, non-isosceles cross-section) rather
/// than a parameter on this one.
///
/// Local layout before centering (right angle at the origin, legs along
/// +X/+Y, extrusion along +Z): cross-section vertices `(0,0)`, `(leg,0)`,
/// `(0,leg)`; two end caps plus three side faces (the two leg faces and
/// the sloped hypotenuse face) — 8 triangles total, each wound so its
/// face normal points outward (verified by `prism_faces_point_outward`
/// below, the same kind of check `spex-ldraw::geometry`'s BFC-CCW
/// assumption relies on for real LDraw parts).
pub fn generate_prism(leg_mm: f64, depth_mm: f64, color_code: u32) -> Vec<Triangle> {
    let l = leg_mm;
    let d = depth_mm;
    // Cross-section corners at z=0 and z=d, right angle at (0,0).
    let a0 = [0.0, 0.0, 0.0];
    let b0 = [l, 0.0, 0.0];
    let c0 = [0.0, l, 0.0];
    let a1 = [0.0, 0.0, d];
    let b1 = [l, 0.0, d];
    let c1 = [0.0, l, d];

    let mut triangles = vec![
        // End caps.
        Triangle { vertices: [a0, c0, b0], color_code }, // z=0 cap, outward -Z
        Triangle { vertices: [a1, b1, c1], color_code }, // z=d cap, outward +Z
        // Leg face at y=0 (outward -Y).
        Triangle { vertices: [a0, b0, b1], color_code },
        Triangle { vertices: [a0, b1, a1], color_code },
        // Leg face at x=0 (outward -X).
        Triangle { vertices: [a0, c1, c0], color_code },
        Triangle { vertices: [a0, a1, c1], color_code },
        // Sloped hypotenuse face (outward, away from the right-angle edge).
        Triangle { vertices: [b0, c0, c1], color_code },
        Triangle { vertices: [b0, c1, b1], color_code },
    ];

    // Center at the origin, matching generate_box's convention (helps
    // later placement/rotation logic treat every shape's local frame the
    // same way).
    let offset = [-l / 2.0, -l / 2.0, -d / 2.0];
    for tri in &mut triangles {
        for v in &mut tri.vertices {
            v[0] += offset[0];
            v[1] += offset[1];
            v[2] += offset[2];
        }
    }
    triangles
}

/// Dispatches on a catalog shape's `shape_type`. `Block` (M98) and `Prism`
/// (M99) are implemented; `ArchVoussoir` (M101) is a real, planned next
/// step, not a silently unsupported placeholder.
pub fn generate_shape(shape: &AnkersteinShape, color_code: u32) -> anyhow::Result<Vec<Triangle>> {
    match shape.shape_type {
        ShapeType::Block => Ok(generate_box(shape.dimensions_mm, color_code)),
        ShapeType::Prism => {
            let [leg_x, leg_y, depth] = shape.dimensions_mm;
            if (leg_x - leg_y).abs() > 1e-9 {
                anyhow::bail!(
                    "shape {:?}: generate_prism only supports the real 45° isosceles case (equal X/Y legs) — \
                     got dimensions_mm {:?}; a non-isosceles roof slope needs its own generation function, \
                     see docs/ANKERSTEIN-ENGINE.md M99",
                    shape.id,
                    shape.dimensions_mm
                );
            }
            Ok(generate_prism(leg_x, depth, color_code))
        }
        ShapeType::ArchVoussoir => anyhow::bail!(
            "arch voussoir geometry not yet implemented (see docs/ANKERSTEIN-ENGINE.md M101) — shape {:?}",
            shape.id
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_box_has_12_triangles() {
        let triangles = generate_box([25.0, 25.0, 25.0], 0);
        assert_eq!(triangles.len(), 12);
    }

    #[test]
    fn generate_box_bounds_match_the_given_dimensions() {
        let dims = [25.0, 12.5, 25.0];
        let triangles = generate_box(dims, 0);
        let mut min = [f64::INFINITY; 3];
        let mut max = [f64::NEG_INFINITY; 3];
        for tri in &triangles {
            for v in tri.vertices {
                for axis in 0..3 {
                    min[axis] = min[axis].min(v[axis]);
                    max[axis] = max[axis].max(v[axis]);
                }
            }
        }
        for axis in 0..3 {
            let span = max[axis] - min[axis];
            assert!((span - dims[axis]).abs() < 1e-9, "axis {axis}: span {span} != {}", dims[axis]);
        }
    }

    #[test]
    fn generate_shape_dispatches_blocks_prisms_and_rejects_arch_voussoirs() {
        let block = AnkersteinShape {
            id: "test-block".to_string(),
            shape_type: ShapeType::Block,
            dimensions_mm: [25.0, 25.0, 25.0],
            caliber: crate::catalog::Caliber::Gk,
            source_citation: "test fixture".to_string(),
        };
        assert!(generate_shape(&block, 4).is_ok());

        let prism = AnkersteinShape {
            shape_type: ShapeType::Prism,
            dimensions_mm: [50.0, 50.0, 50.0],
            ..block.clone()
        };
        assert!(generate_shape(&prism, 4).is_ok(), "the real 45\u{b0} isosceles case should now be implemented");

        let arch = AnkersteinShape {
            shape_type: ShapeType::ArchVoussoir,
            ..block
        };
        assert!(generate_shape(&arch, 4).is_err(), "M101's arch geometry isn't implemented yet");
    }

    #[test]
    fn generate_prism_has_8_triangles() {
        let triangles = generate_prism(50.0, 50.0, 0);
        assert_eq!(triangles.len(), 8);
    }

    #[test]
    fn generate_prism_bounds_match_a_real_bounding_box() {
        // Real citation (George Hardy, Richter's Anker Stone Building
        // Sets): the fifth historical set's prism shapes are sized so two
        // of them form a real cube - here, a 50x50x50mm one (GK's base
        // 25mm unit, doubled). The prism itself only fills half that
        // volume, but its bounding box should match exactly.
        let triangles = generate_prism(50.0, 50.0, 0);
        let mut min = [f64::INFINITY; 3];
        let mut max = [f64::NEG_INFINITY; 3];
        for tri in &triangles {
            for v in tri.vertices {
                for axis in 0..3 {
                    min[axis] = min[axis].min(v[axis]);
                    max[axis] = max[axis].max(v[axis]);
                }
            }
        }
        for axis in 0..3 {
            let span = max[axis] - min[axis];
            assert!((span - 50.0).abs() < 1e-9, "axis {axis}: bounding-box span {span} != 50.0");
        }
    }

    #[test]
    fn generate_prism_rejects_a_non_isosceles_case_via_generate_shape() {
        let lopsided = AnkersteinShape {
            id: "test-lopsided-prism".to_string(),
            shape_type: ShapeType::Prism,
            dimensions_mm: [50.0, 30.0, 50.0], // unequal X/Y legs — not the supported 45° case
            caliber: crate::catalog::Caliber::Gk,
            source_citation: "test fixture".to_string(),
        };
        assert!(generate_shape(&lopsided, 4).is_err());
    }

    /// Verifies every one of the prism's 8 faces has an outward-pointing
    /// normal — the same real correctness property `spex-ldraw::geometry`
    /// relies on LDraw's own BFC-CCW convention for, checked explicitly
    /// here since this geometry is hand-derived, not sourced from a
    /// pre-certified mesh file.
    #[test]
    fn prism_faces_point_outward() {
        use spex_ldraw::geometry::triangle_normal;
        let triangles = generate_prism(2.0, 3.0, 0);
        // Centroid of the whole solid's *bounding box* is the origin
        // (generate_prism centers on it) — for a right-triangular prism,
        // the true solid centroid is offset toward the right-angle
        // corner, so compare each face's own centroid-to-origin-ish
        // direction only loosely: the real, robust check is that a
        // triangle's normal has a positive dot product with the vector
        // from the prism's centroid to that triangle's own centroid — a
        // standard star-shaped-solid outward-normal test.
        let solid_centroid = [-2.0 / 6.0, -2.0 / 6.0, 0.0]; // mean of the 6 real cross-section*2 corners, roughly
        for tri in &triangles {
            let face_centroid = [
                (tri.vertices[0][0] + tri.vertices[1][0] + tri.vertices[2][0]) / 3.0,
                (tri.vertices[0][1] + tri.vertices[1][1] + tri.vertices[2][1]) / 3.0,
                (tri.vertices[0][2] + tri.vertices[1][2] + tri.vertices[2][2]) / 3.0,
            ];
            let outward_ish = [
                face_centroid[0] - solid_centroid[0],
                face_centroid[1] - solid_centroid[1],
                face_centroid[2] - solid_centroid[2],
            ];
            let n = triangle_normal(tri);
            let dot = n[0] * outward_ish[0] + n[1] * outward_ish[1] + n[2] * outward_ish[2];
            assert!(dot >= 0.0, "face {tri:?} has an inward-pointing normal {n:?} (dot {dot})");
        }
    }
}
