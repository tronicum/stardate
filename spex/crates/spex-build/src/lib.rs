//! M72 — `spex-build`: parametric, grid-legal brick construction. See
//! `docs/fugen/phase4-kit.md`'s M72 section for the full spec this crate
//! implements.
pub mod grid;
pub mod primitives;
pub mod recipe;

pub use grid::{validate, FootprintTable, GridPos, Illegality, Orientation, Placement};
pub use primitives::Primitive;
pub use recipe::{build, build_recipe, content_hash, write_ldr, BuildOutput, Recipe};
