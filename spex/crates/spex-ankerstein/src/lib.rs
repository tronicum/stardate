//! Real Ankerstein (Richter's Anchor Stone Building Sets, Rudolstadt 1880)
//! rendering — a native-Rust crate mirroring `spex-ldraw`'s own shape
//! module-for-module, but with parametric solid generation instead of
//! LDraw mesh resolution (no equivalent open parts library exists for
//! Ankerstein — see `docs/ANKERSTEIN-ENGINE.md` §1 for the real licensing
//! background and why this crate does not parse AnkerCAD's or AnkerPlan's
//! bundled stone definitions).
pub mod catalog;
pub mod colors;
pub mod geometry;
pub mod scene;
pub mod sets;

pub use catalog::{load_catalog, AnkersteinShape, Caliber, ShapeType};
pub use colors::{load_colors, AnkersteinColorTable};
pub use geometry::generate_shape;
pub use scene::{parse_scene, Placement, Scene};
pub use sets::{load_sets, validate_against_catalog, AnkersteinSet, SetContent};

/// Real GK (Großes Kaliber) base cube edge length: 25mm. The GK grid's
/// unit — every GK shape's dimensions are a whole or fractional multiple
/// of this, same load-bearing role `spex-ldraw::LDU_TO_MM` plays for LDraw.
pub const GK_UNIT_MM: f64 = 25.0;

/// Real KK (Kleines Kaliber) base cube edge length: 20mm — a separate,
/// non-interchangeable scale from GK. See `docs/ANKERSTEIN-ENGINE.md` §6:
/// this crate defaults to GK-only scenes unless a specific story beat
/// needs KK.
pub const KK_UNIT_MM: f64 = 20.0;
