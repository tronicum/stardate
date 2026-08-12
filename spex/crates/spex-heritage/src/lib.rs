//! M73 — the real World Heritage index.
//!
//! Three files, and the interesting one is not the code.
//!
//!   `model`     — one site, as this project is allowed to hold it
//!   `list`      — the live Wikidata query, and reading the committed snapshot
//!   `curation`  — the buildability filter and the exclusion list
//!
//! `curation.rs` is where the milestone actually lives. The code there is
//! twenty lines; what it enforces is that a site nobody has classified stays
//! out of the Atlas, and that sites of atrocity are data on a list a person
//! reads rather than a condition somewhere in a filter.
//!
//! Everything displayed comes from **Wikidata (CC0)**, never from the World
//! Heritage Centre — see `model.rs` for the licensing finding that forces
//! that, and `docs/fugen/phase4-kit.md` for the one decision this code does
//! not make.
pub mod curation;
pub mod list;
pub mod model;

/// The snapshot this build reads, by name.
///
/// Dated, and named here rather than defaulted in the CLI, so that taking a
/// new snapshot is one deliberate edit in one place instead of a flag people
/// pass differently. `docs/agents/working-mode.md`'s committed-snapshot
/// pattern: the data has a date, and the code says which date it is standing on.
pub const CURRENT_SNAPSHOT: &str = "scripts/heritage-data/wikidata-whs-2026-08-12.json";
pub const CURATION_PATH: &str = "scripts/heritage-data/curation.json";

pub use curation::{is_buildable, verdict, Buildable, Curation, Excluded, ExclusionReason, Verdict};
pub use list::{by_state_party, fetch, load_snapshot, parse_sparql, ENDPOINT, QUERY};
pub use model::{Category, HeritageSite, HeritageSnapshot, SNAPSHOT_VERSION};
