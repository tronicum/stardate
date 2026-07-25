//! Real LDraw line primitives — type 2 (edge) and type 5 (conditional edge).
//!
//! These are not decoration. The black outline a type-2 line produces *is* the
//! visual signature of a rendered brick, and a type-5 line is how a curved
//! surface keeps a crisp silhouette without its whole tessellation showing.
//! `resolve_part` has skipped both since the crate was written; `PartGeometry`
//! keeps them.
//!
//! **The conditional-edge test, stated once so nobody implements it backwards:**
//! a type-5 line is drawn when its two control points project to the **same**
//! side of the line in screen space — that means the two facets meeting at
//! that edge face the same way, so the edge is on the silhouette. When the
//! control points land on opposite sides, the edge is in the interior of a
//! curved surface and must *not* be drawn. Getting this inverted draws exactly
//! the tessellation and hides exactly the silhouette, which is the opposite of
//! the point. (three.js's own `LDrawLoader` discards when the signs differ,
//! for the same reason.)

/// Which kind of real LDraw line this is.
#[derive(Clone, Debug, PartialEq)]
pub enum EdgeKind {
    /// A type-2 line: always drawn.
    Hard,
    /// A type-5 line: drawn only when both control points project to the same
    /// side of the edge. The control points are carried in the same
    /// coordinate frame as the edge's own endpoints.
    Conditional { control: [[f64; 3]; 2] },
}

/// One real LDraw line primitive, in the top-level part's local frame.
#[derive(Clone, Debug, PartialEq)]
pub struct Edge {
    pub vertices: [[f64; 3]; 2],
    /// `None` means LDraw colour 16 — "inherit" — resolved per instance
    /// rather than baked in, so one resolved part can be rendered in any
    /// colour without being resolved again. In practice edges usually carry
    /// colour 24 ("edge colour"), which is also left unresolved here.
    pub color_code: Option<u32>,
    pub kind: EdgeKind,
    /// Index into `PartGeometry::sources` — which real LDraw file this line
    /// came from. This is what lets a LOD pass drop studs and tubes by
    /// *reference path* rather than by guessing from geometry.
    pub source: u16,
}

impl Edge {
    pub fn is_conditional(&self) -> bool {
        matches!(self.kind, EdgeKind::Conditional { .. })
    }

    /// Length in LDraw units. Used for budgeting, and for dropping
    /// degenerate lines some real part files contain.
    pub fn length(&self) -> f64 {
        let [a, b] = self.vertices;
        ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2) + (b[2] - a[2]).powi(2)).sqrt()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hard(a: [f64; 3], b: [f64; 3]) -> Edge {
        Edge { vertices: [a, b], color_code: Some(24), kind: EdgeKind::Hard, source: 0 }
    }

    #[test]
    fn length_of_a_unit_edge() {
        assert!((hard([0.0, 0.0, 0.0], [3.0, 4.0, 0.0]).length() - 5.0).abs() < 1e-12);
    }

    #[test]
    fn a_degenerate_edge_has_zero_length() {
        assert_eq!(hard([1.0, 2.0, 3.0], [1.0, 2.0, 3.0]).length(), 0.0);
    }

    #[test]
    fn conditional_edges_are_distinguishable() {
        let e = Edge {
            vertices: [[0.0; 3], [1.0, 0.0, 0.0]],
            color_code: None,
            kind: EdgeKind::Conditional { control: [[0.0, 1.0, 0.0], [0.0, -1.0, 0.0]] },
            source: 3,
        };
        assert!(e.is_conditional());
        assert!(!hard([0.0; 3], [1.0, 0.0, 0.0]).is_conditional());
    }
}
