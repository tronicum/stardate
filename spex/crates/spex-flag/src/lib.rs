//! M75 — flags as real brick mosaics.
//!
//! The Atlas needs each site's state party's flag, and the piece's diplomatic
//! dimension makes them load-bearing rather than decorative: a flag built from
//! 1×1 tiles in real LDraw colours is the same argument as everything else
//! here, which is that the standardised module renders the world.
//!
//! Three things happen, in order, and each one is separately checkable:
//!
//!   `model`     — the construction sheet, transcribed from a cited document
//!   `raster`    — the sheet at mosaic resolution, analytic, no filtering
//!   `quantize`  — every cell onto a real brick, with the ΔE it cost
//!
//! # What this crate does NOT do, and why
//!
//! It does not emit LDraw. `spex_build::Mosaic` already turns a grid of colour
//! codes into one tile per cell on the real stud grid — it is what `feld.json`
//! (3 600 modules, A3-S05) is built out of. A second emitter here would be a
//! second set of decisions about the same thing, and the two would disagree
//! eventually. So `spex flag` writes a **recipe**, the recipe goes through the
//! same builder as every other scene in the piece, and the `.ldr` a person can
//! open falls out of the pipeline that already exists.
//!
//! That is a deliberate deviation from `phase4-kit.md`'s `emit_flag_ldr`
//! signature, recorded here rather than silently.
//!
//! It also depends on nothing but serde and anyhow — no `spex-ldraw` — so the
//! real colour table arrives as a plain map from the caller. A flag is a
//! declarative document, and reading one should not require a parts cache.
pub mod model;
pub mod quantize;
pub mod raster;

pub use model::{Counterchange, Element, FlagColor, FlagSpec};
pub use quantize::{ciede2000, quantize, srgb_to_lab, QuantizeReport};
pub use raster::rasterize;

/// Above this the flag is reported for review rather than shipped quietly.
/// `phase4-kit.md`'s acceptance criterion 3, as a constant so the number has
/// exactly one home.
pub const DELTA_E_REVIEW_THRESHOLD: f64 = 12.0;
