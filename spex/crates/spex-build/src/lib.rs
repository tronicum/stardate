//! M72 — `spex-build`: parametric, grid-legal brick construction. See
//! `docs/fugen/phase4-kit.md`'s M72 section for the full spec this crate
//! implements.
pub mod grid;

pub use grid::{validate, FootprintTable, GridPos, Illegality, Orientation, Placement};
