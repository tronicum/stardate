//! Real LDraw (https://ldraw.org) fetching, parsing, resolution, and
//! surface sampling — a native-Rust replacement for what was previously
//! prototyped in `unibrick/`'s Python scripts. See `BRICKs.md` in the repo
//! root for the domain glossary/licensing background.
pub mod cache;
pub mod colors;
pub mod geometry;
pub mod sampling;
pub mod scene;

pub use cache::LdrawCache;
pub use colors::{load_colors, ColorTable};
pub use geometry::{place, resolve_part, triangle_area, triangle_normal, Triangle};
pub use sampling::{sample_point_in_triangle, sample_surface, shade_color, to_point_cloud, Sample};
pub use scene::{parse_scene, ModelSource, Placement, Scene};

/// Real LDraw unit conversion — 1 LDU = 0.4mm.
pub const LDU_TO_MM: f64 = 0.4;
