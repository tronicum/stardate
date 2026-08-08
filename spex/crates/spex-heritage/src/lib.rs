//! `spex-heritage` — the real World Heritage index (`docs/FUGEN-ENGINE.md`
//! M73). Two layers, mirroring `spex-ldraw`'s catalog/curation split:
//!
//! - `list`: the raw `HeritageSite` record shape plus the real Wikidata
//!   SPARQL fetch (`fetch_all`, used rarely — see its own doc comment for
//!   why) and the committed-snapshot read/write pair every other command
//!   uses instead.
//! - `curation`: a hand-curated, fail-closed buildability/exclusion layer
//!   on top of the raw snapshot. `is_buildable` is the one function that
//!   answers "can this real site plausibly become a brick model later
//!   (M74+)" — deterministic, reviewable, never a vibe.
//!
//! This crate is data acquisition + curation + query only. No rendering,
//! no geometry — that starts at M74 and is explicitly out of scope here.
pub mod curation;
pub mod list;

pub use curation::{is_buildable, load_curation, Curation, CurationEntry};
pub use list::{fetch_all, read_snapshot, write_snapshot, Category, HeritageSite};
