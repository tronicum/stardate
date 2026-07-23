//! Real LDraw (https://ldraw.org) fetching, parsing, resolution, and
//! surface sampling — a native-Rust replacement for what was previously
//! prototyped in `unibrick/`'s Python scripts. See `BRICKs.md` in the repo
//! root for the domain glossary/licensing background.
pub mod cache;
pub mod colors;

pub use cache::LdrawCache;
pub use colors::{load_colors, ColorTable};

/// Real LDraw unit conversion — 1 LDU = 0.4mm.
pub const LDU_TO_MM: f64 = 0.4;
