//! The hand-curated buildability/exclusion layer on top of the raw
//! Wikidata snapshot (`crate::list::HeritageSite`). See `is_buildable`'s
//! doc comment for the fail-closed rule this whole module exists to
//! enforce, and `docs/FUGEN-ENGINE.md`'s M73 section for the ethical
//! reasoning behind the exclusion list.
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::list::{Category, HeritageSite};

/// One hand-curated buildability decision for a single real site. Never
/// inferred — `buildable` is only ever `true` because a human explicitly
/// set it, and `justification` is a real, specific one-line reason (not a
/// generic template), matching the milestone's acceptance criteria ("≥ 40
/// curated buildable sites, each with a written justification").
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CurationEntry {
    pub buildable: bool,
    pub justification: String,
}

/// The curation file's shape (`spec/heritage.schema.json`'s `curationFile`
/// def) — two independent tables, both keyed by Wikidata QID:
/// - `sites`: reviewed sites, one `CurationEntry` each. A QID **absent**
///   from this map has simply not been reviewed yet — see `is_buildable`.
/// - `excluded`: the ethical exclusion list (genocide/atrocity/mass-death
///   sites; active places of worship by default) — QID -> a one-line
///   written reason. Checked independently of `sites` so it always wins,
///   even against a curation-file typo that also marked the same QID
///   `buildable: true` in `sites`.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Curation {
    #[serde(default)]
    pub sites: HashMap<String, CurationEntry>,
    #[serde(default)]
    pub excluded: HashMap<String, String>,
}

/// Reads a curation file from disk. No default/fallback — a missing or
/// malformed curation file is a real error, not silently treated as "no
/// sites are buildable" (that behavior lives in `is_buildable` instead,
/// applied per-site against a real loaded `Curation`).
pub fn load_curation(path: &Path) -> Result<Curation> {
    let text = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))
}

/// The buildability filter (`docs/FUGEN-ENGINE.md` M73) — deterministic and
/// reviewable, not a vibe. A site qualifies for the Atlas when ALL hold:
///
/// 1. `category` is `Cultural` or `Mixed` — a natural landscape isn't a
///    Klemmbaustein subject, and pretending otherwise would be dishonest.
/// 2. The site has an explicit hand-curated `buildable: true` entry in
///    `curation.sites` — **never inferred, never defaulted to true**. A
///    site with no curation entry at all is NOT buildable: this is the
///    fail-closed rule the milestone spec calls out by name.
/// 3. The site is not on `curation.excluded` (checked regardless of what
///    `sites` says, so it always wins).
pub fn is_buildable(site: &HeritageSite, curation: &Curation) -> bool {
    if site.category != Category::Cultural && site.category != Category::Mixed {
        return false;
    }
    if curation.excluded.contains_key(&site.id) {
        return false;
    }
    curation.sites.get(&site.id).map(|entry| entry.buildable).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cultural_site(id: &str) -> HeritageSite {
        HeritageSite {
            id: id.to_string(),
            name: "Test Site".to_string(),
            state_parties: vec!["Testland".to_string()],
            inscribed_year: 2000,
            criteria: vec!["i".to_string()],
            category: Category::Cultural,
            lat: 0.0,
            lon: 0.0,
            source: format!("wikidata:{id}"),
        }
    }

    fn natural_site(id: &str) -> HeritageSite {
        HeritageSite { category: Category::Natural, ..cultural_site(id) }
    }

    fn mixed_site(id: &str) -> HeritageSite {
        HeritageSite { category: Category::Mixed, ..cultural_site(id) }
    }

    #[test]
    fn a_site_with_no_curation_entry_at_all_is_not_buildable() {
        let curation = Curation::default();
        assert!(!is_buildable(&cultural_site("Q1"), &curation));
    }

    #[test]
    fn a_cultural_site_explicitly_marked_buildable_is_buildable() {
        let mut curation = Curation::default();
        curation.sites.insert(
            "Q1".to_string(),
            CurationEntry { buildable: true, justification: "simple pyramidal massing, real test fixture".to_string() },
        );
        assert!(is_buildable(&cultural_site("Q1"), &curation));
    }

    #[test]
    fn a_cultural_site_explicitly_marked_not_buildable_stays_not_buildable() {
        let mut curation = Curation::default();
        curation
            .sites
            .insert("Q1".to_string(), CurationEntry { buildable: false, justification: "too organic/irregular a form".to_string() });
        assert!(!is_buildable(&cultural_site("Q1"), &curation));
    }

    #[test]
    fn a_natural_site_is_never_buildable_even_if_marked_true() {
        let mut curation = Curation::default();
        curation.sites.insert("Q1".to_string(), CurationEntry { buildable: true, justification: "irrelevant".to_string() });
        assert!(!is_buildable(&natural_site("Q1"), &curation));
    }

    #[test]
    fn a_mixed_site_can_be_buildable() {
        let mut curation = Curation::default();
        curation.sites.insert(
            "Q1".to_string(),
            CurationEntry { buildable: true, justification: "the built component is simple enough on its own".to_string() },
        );
        assert!(is_buildable(&mixed_site("Q1"), &curation));
    }

    #[test]
    fn the_exclusion_list_wins_even_over_an_explicit_buildable_true() {
        let mut curation = Curation::default();
        curation.sites.insert(
            "Q1".to_string(),
            CurationEntry { buildable: true, justification: "a curation error that should never win".to_string() },
        );
        curation.excluded.insert("Q1".to_string(), "real atrocity/genocide site — never rendered as a toy".to_string());
        assert!(!is_buildable(&cultural_site("Q1"), &curation));
    }

    #[test]
    fn load_curation_round_trips_through_disk() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("curation.json");
        let mut curation = Curation::default();
        curation
            .sites
            .insert("Q1".to_string(), CurationEntry { buildable: true, justification: "real, specific reason".to_string() });
        curation.excluded.insert("Q2".to_string(), "real, specific exclusion reason".to_string());
        fs::write(&path, serde_json::to_string_pretty(&curation).unwrap()).unwrap();

        let loaded = load_curation(&path).unwrap();
        assert!(loaded.sites.get("Q1").unwrap().buildable);
        assert_eq!(loaded.excluded.get("Q2").unwrap(), "real, specific exclusion reason");
    }

    #[test]
    fn load_curation_errors_on_a_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        assert!(load_curation(&dir.path().join("does-not-exist.json")).is_err());
    }
}
