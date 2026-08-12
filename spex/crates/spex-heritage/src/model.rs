//! One World Heritage site, as this project is allowed to hold it.
//!
//! # The licensing finding that shapes this whole crate
//!
//! The World Heritage Centre's syndication terms state that *"any
//! republication, online or in any other form, of any UNESCO/WHC data requires
//! prior written authorization"*, that content may not be modified, and that a
//! specific copyright notice must accompany any use. A generative artwork that
//! displays site names cannot live under that — unless authorisation is
//! obtained, which is Stefan's decision and not one this code makes.
//!
//! So **every displayed field here comes from Wikidata, which is CC0**. The
//! WHC list may be consulted during development as a cross-check on the count,
//! and the number written down; no WHC text is redistributed, and the World
//! Heritage Emblem appears nowhere. The work may say in plain prose that a
//! site *is* a World Heritage Site — that is a fact, not a mark.
//!
//! `source` is on the record rather than in a README for the same reason a
//! recipe carries its own scale note: provenance that lives somewhere else is
//! provenance that gets separated from the thing it describes.
use serde::{Deserialize, Serialize};

/// UNESCO's own three categories — and derived from the criteria rather than
/// carried as a separate field, because that is how the categories are
/// *defined*: criteria i–vi are cultural, vii–x are natural, and a site with
/// both is mixed. Deriving it means the category cannot disagree with the
/// criteria, which a second field eventually would.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Category {
    Cultural,
    Natural,
    Mixed,
    /// The criteria are missing from the record. Not an error and not a guess:
    /// `is_buildable` fails closed on it, like everything else here.
    Unknown,
}

pub const CULTURAL_CRITERIA: [&str; 6] = ["i", "ii", "iii", "iv", "v", "vi"];
pub const NATURAL_CRITERIA: [&str; 4] = ["vii", "viii", "ix", "x"];

impl Category {
    pub fn from_criteria(criteria: &[String]) -> Category {
        let cultural = criteria.iter().any(|c| CULTURAL_CRITERIA.contains(&c.as_str()));
        let natural = criteria.iter().any(|c| NATURAL_CRITERIA.contains(&c.as_str()));
        match (cultural, natural) {
            (true, true) => Category::Mixed,
            (true, false) => Category::Cultural,
            (false, true) => Category::Natural,
            (false, false) => Category::Unknown,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeritageSite {
    /// Wikidata QID, e.g. `Q9259`.
    pub id: String,
    pub name: String,
    /// ISO 3166-1 alpha-2. Plural because a site can be transnational — the
    /// Struve Geodetic Arc crosses ten countries, and a single-country field
    /// would have quietly dropped nine of them.
    pub state_parties: Vec<String>,
    pub inscribed_year: Option<u32>,
    /// "i".."x", lowercase Roman.
    pub criteria: Vec<String>,
    pub category: Category,
    pub lat: Option<f64>,
    pub lon: Option<f64>,
    /// Provenance, in the record: `wikidata:Q...`.
    pub source: String,
}

impl HeritageSite {
    /// Whether the record carries everything the Atlas needs to **place and
    /// label** a site: a name, a state party, a year, and a position.
    ///
    /// Criteria are deliberately not in here, and that is a measurement rather
    /// than a lowered bar. Wikidata's coverage of `P2614` is about 55 %: the
    /// 2026-08-12 snapshot has 1 411 sites, of which 1 379 can be placed and
    /// labelled and only 782 carry criteria. Quedlinburg, Westminster, Genoa
    /// and Hallstatt are all complete, named, dated records with no criteria
    /// qualifier — the gap is in Wikidata, not in the query, and folding it
    /// into one boolean would report a data-coverage problem as a broken
    /// pipeline. [`Self::is_categorised`] is the other number.
    pub fn is_complete(&self) -> bool {
        !self.name.is_empty()
            && !self.state_parties.is_empty()
            && self.inscribed_year.is_some()
            && self.lat.is_some()
            && self.lon.is_some()
    }

    /// Whether Wikidata says which criteria this site was inscribed under, and
    /// therefore whether its cultural/natural category can be derived.
    pub fn is_categorised(&self) -> bool {
        self.category != Category::Unknown
    }
}

/// The committed snapshot: a real query, on a real date, with its own query
/// text inside it.
///
/// The query is carried in the file rather than only in the source, because a
/// snapshot whose query you cannot read is a snapshot you cannot reproduce or
/// argue with. Same reason `docs/agents/working-mode.md` wants snapshots
/// committed at all.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeritageSnapshot {
    pub version: u32,
    /// ISO date the query was run.
    pub fetched: String,
    pub endpoint: String,
    pub query: String,
    pub sites: Vec<HeritageSite>,
}

pub const SNAPSHOT_VERSION: u32 = 1;

#[cfg(test)]
mod tests {
    use super::*;

    fn c(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn categories_come_from_the_criteria_that_define_them() {
        assert_eq!(Category::from_criteria(&c(&["i", "iv"])), Category::Cultural);
        assert_eq!(Category::from_criteria(&c(&["vii"])), Category::Natural);
        // Mount Athos is i, ii, iv, v, vi, vii — cultural and natural at once,
        // which is what "mixed" means.
        assert_eq!(Category::from_criteria(&c(&["i", "ii", "iv", "v", "vi", "vii"])), Category::Mixed);
        assert_eq!(Category::from_criteria(&[]), Category::Unknown);
    }

    /// "ii" must not be read as a prefix of "iii", and "x" must not match
    /// inside "ix". Whole-token comparison is what prevents that, and it is
    /// worth pinning because a substring search here would classify most of
    /// the list wrong and still look plausible.
    #[test]
    fn roman_numerals_are_compared_whole() {
        assert_eq!(Category::from_criteria(&c(&["iii"])), Category::Cultural);
        assert_eq!(Category::from_criteria(&c(&["ix"])), Category::Natural);
        assert_eq!(Category::from_criteria(&c(&["xi"])), Category::Unknown);
    }
}
