//! The buildability filter and the exclusion list.
//!
//! # The part of this file that is not a technical decision
//!
//! Sites of atrocity, genocide, slavery and mass death are not rendered as toy
//! bricks. That is a constraint on the work, not a feature of it, and it is
//! data rather than code so that a person can read the whole list at once and
//! argue with it.
//!
//! The code **fails closed**: a site that nobody has classified is excluded.
//! Not included-with-a-warning, not included-pending-review — excluded. The
//! failure mode of the other choice is that a site nobody looked at ends up in
//! a screening, and there is no version of that this project would accept.
//!
//! Buildability is the same shape: a hand-written `buildable: true` with a
//! one-line justification, never inferred from the record. A rule that decided
//! it from criteria or category would be a rule that sounds objective and is
//! actually a guess about architecture, and the guess would be invisible.
use crate::model::{Category, HeritageSite};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// Why a site is excluded. The reason is on the record and not in a comment,
/// because this list gets read by people who did not write it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExclusionReason {
    /// Sites of genocide, mass death, atrocity, slavery, or nuclear testing.
    Atrocity,
    /// An active place of worship. Excluded **by default**; inclusion needs an
    /// explicit recorded decision, which is what moving the entry to
    /// `buildable` with a justification amounts to.
    ActiveWorship,
    /// A site whose subject is a landscape, a species, or a geological
    /// formation. Not a moral exclusion — a Klemmbaustein is the wrong
    /// instrument, and pretending otherwise would be dishonest.
    NotArchitecture,
    /// Excluded for a reason written out in `note`.
    Other,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Excluded {
    pub id: String,
    pub name: String,
    pub reason: ExclusionReason,
    pub note: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Buildable {
    pub id: String,
    pub name: String,
    /// A written sentence, by a person, about what the primitives would build.
    /// Required and checked for length: "yes" is not a justification.
    pub justification: String,
    /// A, B or C — which cut this site appears in.
    pub tier: char,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Curation {
    pub version: u32,
    /// The date a person last read this file end to end. Not decoration:
    /// `phase4-kit.md` requires the exclusion list to be reviewed before every
    /// release, and a review with no date is a review nobody can check.
    pub reviewed: String,
    pub reviewer: String,
    pub buildable: Vec<Buildable>,
    pub excluded: Vec<Excluded>,
}

impl Curation {
    pub fn load(path: &Path) -> Result<Curation> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("reading curation from {}", path.display()))?;
        let c: Curation = serde_json::from_str(&text)
            .with_context(|| format!("{} is not a curation file", path.display()))?;
        Ok(c)
    }

    pub fn buildable_index(&self) -> BTreeMap<&str, &Buildable> {
        self.buildable.iter().map(|b| (b.id.as_str(), b)).collect()
    }

    pub fn excluded_index(&self) -> BTreeMap<&str, &Excluded> {
        self.excluded.iter().map(|e| (e.id.as_str(), e)).collect()
    }

    /// Every structural problem at once, the same way `spex_show::validate`
    /// reports them.
    pub fn validate(&self) -> Vec<String> {
        let mut errs = Vec::new();
        if self.reviewed.trim().is_empty() || self.reviewer.trim().is_empty() {
            errs.push("reviewed/reviewer are empty — this list is required to be read by a person before a release".into());
        }
        if self.excluded.is_empty() {
            errs.push("the exclusion list is empty, which cannot be right: the World Heritage list contains Auschwitz Birkenau".into());
        }
        let excluded = self.excluded_index();
        for b in &self.buildable {
            if b.justification.split_whitespace().count() < 5 {
                errs.push(format!("{} ({}): a justification of under five words is not one", b.id, b.name));
            }
            if !['A', 'B', 'C'].contains(&b.tier) {
                errs.push(format!("{}: tier {:?} is not A, B or C", b.id, b.tier));
            }
            if excluded.contains_key(b.id.as_str()) {
                errs.push(format!("{} ({}) is both buildable and excluded", b.id, b.name));
            }
        }
        for e in &self.excluded {
            if e.note.split_whitespace().count() < 3 {
                errs.push(format!("{} ({}): an exclusion needs a written reason", e.id, e.name));
            }
        }
        errs
    }
}

/// Why a site is or is not in the Atlas — the whole answer, not a boolean.
///
/// A boolean would be the wrong return type for a decision a person has to
/// review: "no" is the same word for "we decided against it", "nobody has
/// looked at it yet" and "a Klemmbaustein cannot express a coral reef", and
/// those three want different follow-up.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verdict {
    Buildable { tier: char },
    Excluded(ExclusionReason),
    /// A natural site. Filtered before curation ever sees it, and it stays
    /// filtered even if somebody curated it — see [`Verdict::Contested`].
    NotCultural(Category),
    /// A person wrote a justification for a site Wikidata calls **natural**.
    /// Not silently resolved either way: it is a person and the data
    /// disagreeing about a fact, and the disagreement is the useful output.
    /// Not buildable until it is settled.
    Contested { tier: char },
    /// **The fail-closed case.** Nobody has classified this site, so it is out.
    Unclassified,
}

impl Verdict {
    pub fn is_buildable(&self) -> bool {
        matches!(self, Verdict::Buildable { .. })
    }
}

/// The filter, in the order the conditions actually apply.
///
/// **Exclusion first**, so an excluded site stays reported as excluded rather
/// than as "not architecture" — the reason is the point of the record, and a
/// site can be both.
///
/// **Then curation, then category** — and that order is a decision the real
/// snapshot forced. Wikidata carries criteria for only 55 % of the list, so
/// deriving the category first would have thrown out Quedlinburg, Westminster,
/// Genoa and Hallstatt: complete, named, obviously architectural records whose
/// only defect is a missing qualifier. A written justification by a person is
/// a *better* classification than an absent one, so an explicitly curated site
/// survives an `Unknown` category.
///
/// It does **not** survive a `Natural` one. There the data says something
/// positive and a person said the opposite, and quietly preferring either is
/// worse than reporting [`Verdict::Contested`] and making somebody look.
pub fn verdict(site: &HeritageSite, curation: &Curation) -> Verdict {
    if let Some(e) = curation.excluded.iter().find(|e| e.id == site.id) {
        return Verdict::Excluded(e.reason);
    }
    let curated = curation.buildable.iter().find(|b| b.id == site.id);
    match (curated, site.category) {
        (Some(b), Category::Cultural | Category::Mixed | Category::Unknown) => {
            Verdict::Buildable { tier: b.tier }
        }
        (Some(b), Category::Natural) => Verdict::Contested { tier: b.tier },
        (None, Category::Cultural | Category::Mixed) => Verdict::Unclassified,
        (None, c) => Verdict::NotCultural(c),
    }
}

/// `phase4-kit.md`'s own signature, kept so the spec and the code read the
/// same. It is a thin wrapper over [`verdict`], which is the function anything
/// that has to explain itself should call.
pub fn is_buildable(site: &HeritageSite, curation: &Curation) -> bool {
    verdict(site, curation).is_buildable()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn site(id: &str, category: Category) -> HeritageSite {
        HeritageSite {
            id: id.into(),
            name: format!("site {id}"),
            state_parties: vec!["DK".into()],
            inscribed_year: Some(1994),
            criteria: vec![],
            category,
            lat: Some(0.0),
            lon: Some(0.0),
            source: format!("wikidata:{id}"),
        }
    }

    fn curation() -> Curation {
        Curation {
            version: 1,
            reviewed: "2026-08-12".into(),
            reviewer: "test".into(),
            buildable: vec![Buildable {
                id: "Q1".into(),
                name: "yes".into(),
                justification: "a colonnade, a wall and a gable, all in the vocabulary".into(),
                tier: 'A',
            }],
            excluded: vec![Excluded {
                id: "Q2".into(),
                name: "no".into(),
                reason: ExclusionReason::Atrocity,
                note: "a site of mass death".into(),
            }],
        }
    }

    /// The single most important behaviour in this file: a site nobody has
    /// classified is OUT. If this test ever goes green the other way round,
    /// something got into a screening that no person ever looked at.
    #[test]
    fn an_unclassified_site_is_excluded() {
        let v = verdict(&site("Q999", Category::Cultural), &curation());
        assert_eq!(v, Verdict::Unclassified);
        assert!(!v.is_buildable());
    }

    #[test]
    fn exclusion_beats_everything_and_keeps_its_reason() {
        // Q2 is on the exclusion list *and* cultural *and* would otherwise be
        // unclassified. The reason has to survive all of that.
        assert_eq!(
            verdict(&site("Q2", Category::Cultural), &curation()),
            Verdict::Excluded(ExclusionReason::Atrocity)
        );
    }

    #[test]
    fn a_natural_site_is_not_a_klemmbaustein_subject() {
        assert_eq!(
            verdict(&site("Q7", Category::Natural), &curation()),
            Verdict::NotCultural(Category::Natural)
        );
    }

    /// Quedlinburg's case, and the reason the order is curation-then-category:
    /// Wikidata has no criteria qualifier for it, so the category is Unknown,
    /// and a person's written justification is the better classification.
    #[test]
    fn a_curated_site_survives_a_missing_category() {
        assert_eq!(
            verdict(&site("Q1", Category::Unknown), &curation()),
            Verdict::Buildable { tier: 'A' }
        );
    }

    /// But an absent category is not the same as a category that says
    /// something else. A person and the data disagreeing is reported, not
    /// resolved, and it is not buildable meanwhile.
    #[test]
    fn a_person_and_the_data_disagreeing_is_reported_not_resolved() {
        let v = verdict(&site("Q1", Category::Natural), &curation());
        assert_eq!(v, Verdict::Contested { tier: 'A' });
        assert!(!v.is_buildable());
    }

    #[test]
    fn a_curated_cultural_site_carries_its_tier() {
        assert_eq!(
            verdict(&site("Q1", Category::Cultural), &curation()),
            Verdict::Buildable { tier: 'A' }
        );
    }

    #[test]
    fn validate_rejects_an_empty_exclusion_list_and_a_one_word_justification() {
        let mut c = curation();
        c.excluded.clear();
        c.buildable[0].justification = "yes".into();
        let errs = c.validate();
        assert_eq!(errs.len(), 2, "{errs:?}");
    }
}
