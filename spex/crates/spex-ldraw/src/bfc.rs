//! Real LDraw BFC (Back Face Culling) winding resolution.
//!
//! Three real mechanisms compose, and all three have to be tracked together
//! or a composite part ends up with inward-facing normals that only become
//! visible once the geometry is lit:
//!
//! 1. a file declares `0 BFC CERTIFY CCW` (the near-universal case) or `CW`,
//!    and may switch winding mid-file with a bare `0 BFC CW` / `0 BFC CCW`;
//! 2. `0 BFC INVERTNEXT` flips the *next* type-1 reference, and only that one;
//! 3. a reference whose own 3x3 matrix has a negative determinant is itself a
//!    mirroring transform, which flips winding for that reference's whole
//!    subtree.
//!
//! When the composed state says "reversed", the emitted triangle's vertices
//! are stored in reverse order, so that the plain right-hand-rule normal in
//! `geometry::triangle_normal` comes out pointing outward with no further
//! correction anywhere downstream.
//!
//! **A trap worth stating here, because it is invisible until something is
//! rendered** (found by `scripts/mesh-vs-points-spike/`): getting BFC right in
//! this module is necessary but not sufficient. LDraw is Y-down and spex is
//! Y-up, so the conversion to spex's frame negates Y — which is a *mirror*. It
//! flips handedness and therefore inverts every winding this module so
//! carefully established. Whoever performs that conversion must also swap two
//! vertices of each triangle. This module deliberately stays in LDraw's own
//! coordinate frame so that there is exactly one place where that happens.

/// Which way round a face's vertices are wound, in the frame it was authored in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Winding {
    #[default]
    Ccw,
    Cw,
}

impl Winding {
    pub fn flipped(self) -> Winding {
        match self {
            Winding::Ccw => Winding::Cw,
            Winding::Cw => Winding::Ccw,
        }
    }

    /// `true` when a face's stored vertex order must be reversed before its
    /// normal is taken. LDraw's own convention is counter-clockwise as seen
    /// from outside, so CCW is the identity case.
    pub fn is_reversed(self) -> bool {
        self == Winding::Cw
    }

    pub fn xor(self, flip: bool) -> Winding {
        if flip {
            self.flipped()
        } else {
            self
        }
    }
}

/// Per-file BFC state, fed one `0 BFC ...` meta line at a time as the file is
/// read top to bottom.
#[derive(Clone, Debug)]
pub struct BfcState {
    /// Whether this file ever declared `BFC CERTIFY`. An uncertified file is
    /// treated as CCW, but the caller may want to know — real official parts
    /// are certified essentially without exception, so an uncertified one is
    /// worth a warning rather than a silent assumption.
    pub certified: bool,
    /// The file's current winding, which a bare `0 BFC CW`/`CCW` can change
    /// part-way through.
    pub winding: Winding,
    /// Set by `0 BFC INVERTNEXT`, consumed by the next type-1 reference.
    pub invert_next: bool,
    /// Winding inherited from every ancestor reference, composed.
    pub inherited_reversed: bool,
}

impl BfcState {
    pub fn new(inherited_reversed: bool) -> Self {
        BfcState {
            certified: false,
            winding: Winding::Ccw,
            invert_next: false,
            inherited_reversed,
        }
    }

    /// Applies one `0 BFC ...` line's tokens (the whole line, split on
    /// whitespace, including the leading `0` and `BFC`).
    pub fn apply_meta(&mut self, tokens: &[&str]) {
        if tokens.len() < 2 || tokens[1] != "BFC" {
            return;
        }
        let rest = &tokens[2..];
        if rest.iter().any(|t| *t == "NOCERTIFY") {
            self.certified = false;
            return;
        }
        if rest.iter().any(|t| *t == "CERTIFY") {
            self.certified = true;
        }
        // CW/CCW may appear either on the CERTIFY line or on its own later.
        for t in rest {
            match *t {
                "CW" => self.winding = Winding::Cw,
                "CCW" => self.winding = Winding::Ccw,
                "INVERTNEXT" => self.invert_next = true,
                // CLIP/NOCLIP control whether a renderer *culls*, not how a
                // face is wound. This crate always emits geometry and leaves
                // culling to the renderer, so they are deliberately ignored.
                _ => {}
            }
        }
    }

    /// Whether a face read in this file's current state must have its vertex
    /// order reversed.
    pub fn face_reversed(&self) -> bool {
        self.inherited_reversed ^ self.winding.is_reversed()
    }

    /// Consumes any pending `INVERTNEXT` and folds in the determinant sign of
    /// the reference's own matrix, giving the winding state the referenced
    /// file's whole subtree inherits.
    pub fn winding_for_reference(&mut self, matrix: &[f64; 9]) -> bool {
        let invert = std::mem::take(&mut self.invert_next);
        let mirrored = determinant3(matrix) < 0.0;
        self.face_reversed() ^ invert ^ mirrored
    }
}

/// Determinant of a row-major flat 3x3 matrix. Negative means the transform
/// mirrors, which flips winding for everything under it.
pub fn determinant3(m: &[f64; 9]) -> f64 {
    m[0] * (m[4] * m[8] - m[5] * m[7]) - m[1] * (m[3] * m[8] - m[5] * m[6])
        + m[2] * (m[3] * m[7] - m[4] * m[6])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::IDENTITY;

    #[test]
    fn identity_has_positive_determinant_and_does_not_flip() {
        assert!(determinant3(&IDENTITY) > 0.0);
        let mut s = BfcState::new(false);
        assert!(!s.winding_for_reference(&IDENTITY));
    }

    #[test]
    fn a_mirroring_matrix_flips_winding() {
        // Real LDraw parts genuinely use these: a part placed with one axis
        // negated to make the mirrored half of a symmetric shape.
        let mirror_x = [-1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
        assert!(determinant3(&mirror_x) < 0.0);
        let mut s = BfcState::new(false);
        assert!(s.winding_for_reference(&mirror_x));
    }

    #[test]
    fn invertnext_applies_to_exactly_one_reference() {
        let mut s = BfcState::new(false);
        s.apply_meta(&["0", "BFC", "INVERTNEXT"]);
        assert!(s.winding_for_reference(&IDENTITY), "the flagged reference is inverted");
        assert!(!s.winding_for_reference(&IDENTITY), "the one after it is not");
    }

    #[test]
    fn invertnext_and_a_mirror_cancel_each_other() {
        let mirror_z = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, -1.0];
        let mut s = BfcState::new(false);
        s.apply_meta(&["0", "BFC", "INVERTNEXT"]);
        assert!(!s.winding_for_reference(&mirror_z), "two flips are no flip");
    }

    #[test]
    fn certify_cw_reverses_every_face_in_the_file() {
        let mut s = BfcState::new(false);
        s.apply_meta(&["0", "BFC", "CERTIFY", "CW"]);
        assert!(s.certified);
        assert!(s.face_reversed());
    }

    #[test]
    fn a_bare_cw_can_switch_winding_part_way_through_a_file() {
        let mut s = BfcState::new(false);
        s.apply_meta(&["0", "BFC", "CERTIFY", "CCW"]);
        assert!(!s.face_reversed());
        s.apply_meta(&["0", "BFC", "CW"]);
        assert!(s.face_reversed());
        s.apply_meta(&["0", "BFC", "CCW"]);
        assert!(!s.face_reversed());
    }

    #[test]
    fn nocertify_clears_certification() {
        let mut s = BfcState::new(false);
        s.apply_meta(&["0", "BFC", "CERTIFY", "CCW"]);
        s.apply_meta(&["0", "BFC", "NOCERTIFY"]);
        assert!(!s.certified);
    }

    #[test]
    fn inherited_reversal_composes_with_the_files_own_winding() {
        let mut s = BfcState::new(true); // an ancestor already flipped us
        assert!(s.face_reversed());
        s.apply_meta(&["0", "BFC", "CERTIFY", "CW"]);
        assert!(!s.face_reversed(), "two flips are no flip");
    }

    #[test]
    fn non_bfc_meta_lines_are_ignored() {
        let mut s = BfcState::new(false);
        s.apply_meta(&["0", "Name:", "3005.dat"]);
        s.apply_meta(&["0", "!LICENSE", "Redistributable", "under", "CCAL", "version", "2.0"]);
        assert!(!s.certified);
        assert!(!s.invert_next);
        assert_eq!(s.winding, Winding::Ccw);
    }
}
