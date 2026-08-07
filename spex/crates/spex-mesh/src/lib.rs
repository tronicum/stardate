//! `spex-brick-mesh` — real LDraw geometry packed into a bundle the viewer can
//! upload once: a small JSON manifest plus tightly packed binary buffers.
//!
//! This is the seam where LDraw's own coordinate frame becomes spex's, and
//! that conversion is a mirror — see [`bundle::to_output_position`]. It is the
//! only place in the codebase allowed to perform it.
pub mod bundle;
pub mod lod;
pub mod material;
pub mod weld;

pub use bundle::{
    srgb_to_linear, to_output_matrix, to_output_position, to_output_triangles, FullColorTable,
    Manifest, MeshBundleBuilder, MeshBundleStats, PartBuffers, FORMAT_VERSION,
};
pub use lod::{chain_is_stud, lod1, lod2};
pub use material::{from_ldraw, PbrMaterial, SpeckleParams};
pub use weld::{weld_and_smooth, WeldedMesh, DEFAULT_CREASE_DEGREES};
