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

/// Dispatches on a catalog shape's `shape_type`. Only `Block` is
/// implemented so far (see `docs/ANKERSTEIN-ENGINE.md` M98) — `Prism`
/// (M99) and `ArchVoussoir` (M101) are real, planned next steps, not
/// silently unsupported placeholders.
pub fn generate_shape(shape: &AnkersteinShape, color_code: u32) -> anyhow::Result<Vec<Triangle>> {
    match shape.shape_type {
        ShapeType::Block => Ok(generate_box(shape.dimensions_mm, color_code)),
        ShapeType::Prism => anyhow::bail!(
            "prism geometry not yet implemented (see docs/ANKERSTEIN-ENGINE.md M99) — shape {:?}",
            shape.id
        ),
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
    fn generate_shape_dispatches_blocks_and_rejects_unimplemented_types() {
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
            ..block
        };
        assert!(generate_shape(&prism, 4).is_err());
    }
}
