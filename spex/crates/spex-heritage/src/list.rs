//! The live Wikidata query, and reading the snapshot it produced.
//!
//! Two functions over one dataset, and the split is the one this repo already
//! uses for the Wikipedia scripts: [`fetch`] goes to the network and is run
//! rarely and on purpose; [`load_snapshot`] reads the committed file and is
//! what everything else calls. A build that queries a live endpoint is a build
//! whose output depends on a Tuesday.
use crate::model::{Category, HeritageSite, HeritageSnapshot, SNAPSHOT_VERSION};
use anyhow::{Context, Result};
use std::collections::BTreeMap;
use std::path::Path;

pub const ENDPOINT: &str = "https://query.wikidata.org/sparql";
const USER_AGENT: &str = "spex/0.1 (Iunctura Archiv IA-2026-002; https://github.com/tronicum/stardate)";

/// Every World Heritage site Wikidata knows, with the fields the Atlas needs.
///
/// # Why this shape
///
/// `wdt:P1435 wd:Q9259` is "has heritage designation: World Heritage Site" —
/// the truthy path, so a site that once held the designation and lost it does
/// not appear. The inscription year comes off the **statement**'s `pq:P580`
/// qualifier rather than from a top-level property, because that is where
/// Wikidata puts "when this designation started" and a site can hold several
/// designations with different dates.
///
/// The criteria come off the same statement as `pq:P2260`, and they are
/// returned as their item labels ("i", "iv", ...) via `GROUP_CONCAT`, so one
/// row is one site. Without the concat a site with six criteria is six rows
/// and every later count is wrong.
///
/// `P297` is the state party's ISO 3166-1 alpha-2 code, concatenated for the
/// same reason: the Struve Geodetic Arc has ten.
///
/// # Two things learnt by running it
///
/// **`wdt:P757` is required, not optional.** The designation alone returned
/// 3 386 items against a real list of roughly 1 250, because Wikidata also
/// designates the *component parts* of serial sites — every Struve station,
/// every Le Corbusier building. Only the inscribed sites themselves carry a
/// World Heritage Site ID, so requiring one is the filter that separates a
/// site from a piece of one. The ID is used to *select* and is not carried
/// into the record: the displayed fields stay Wikidata's.
///
/// **The criteria qualifier is `pq:P2614`, and the criteria items are
/// labelled `(iii)`, `(vi)` — the bare numeral in brackets.** Two guesses died
/// first, `P2260` and `P1013`, and each time every one of 3 386 records came
/// back with an empty criteria list and category `Unknown`. Because the
/// curation filter fails closed, that would have produced an Atlas of nothing
/// at all rather than an error — which is the argument for `heritage-index`
/// printing the category histogram: a field that arrives empty *everywhere* is
/// not a gap in the data. It was settled by reading one real item's claims
/// (Babylon, Q100329356) rather than by guessing a fourth time.
///
/// **`pq:P1448` is the label of last resort.** Roughly a third of the sites
/// have no English `rdfs:label`, but the designation statement almost always
/// carries the official inscribed name as a qualifier. It is read off the
/// *statement*, so it is the name under this designation — Babylon has a
/// second P1435 statement with a different name entirely.
pub const QUERY: &str = r#"SELECT ?site ?siteLabel
       (GROUP_CONCAT(DISTINCT ?iso; separator=",") AS ?isos)
       (GROUP_CONCAT(DISTINCT ?critLabel; separator=",") AS ?crits)
       (SAMPLE(?year) AS ?inscribed)
       (SAMPLE(?coord) AS ?location)
       (SAMPLE(?official) AS ?officialName)
WHERE {
  ?site p:P1435 ?stmt .
  ?stmt ps:P1435 wd:Q9259 .
  ?site wdt:P757 ?whs .
  OPTIONAL { ?stmt pq:P580 ?start . BIND(YEAR(?start) AS ?year) }
  OPTIONAL {
    ?stmt pq:P2614 ?crit .
    ?crit rdfs:label ?critLabel . FILTER(LANG(?critLabel) = "en")
  }
  OPTIONAL { ?stmt pq:P1448 ?official . FILTER(LANG(?official) = "en") }
  OPTIONAL { ?site wdt:P17 ?country . ?country wdt:P297 ?iso }
  OPTIONAL { ?site wdt:P625 ?coord }
  SERVICE wikibase:label { bd:serviceParam wikibase:language "en" }
}
GROUP BY ?site ?siteLabel"#;

/// Runs the query and returns a snapshot ready to commit.
///
/// `fetched` is passed in rather than read from the clock: the caller knows
/// what date it means by, and a function that stamps itself is a function you
/// cannot test.
pub fn fetch(fetched: &str) -> Result<HeritageSnapshot> {
    let body = ureq::get(ENDPOINT)
        .query("query", QUERY)
        .query("format", "json")
        .header("User-Agent", USER_AGENT)
        .header("Accept", "application/sparql-results+json")
        .call()
        .context("querying the Wikidata SPARQL endpoint")?
        .body_mut()
        .read_to_string()
        .context("reading the SPARQL response")?;
    let sites = parse_sparql(&body)?;
    Ok(HeritageSnapshot {
        version: SNAPSHOT_VERSION,
        fetched: fetched.to_string(),
        endpoint: ENDPOINT.to_string(),
        query: QUERY.to_string(),
        sites,
    })
}

/// Turns a SPARQL JSON result set into records.
///
/// Separate from [`fetch`] and public so it can be tested against a real saved
/// response without a network — the same reason `spex-fugue` has an SMF
/// *reader*: a parser tested only through the thing that produced it is tested
/// against itself.
pub fn parse_sparql(json: &str) -> Result<Vec<HeritageSite>> {
    let v: serde_json::Value = serde_json::from_str(json).context("the response is not JSON")?;
    let rows = v["results"]["bindings"]
        .as_array()
        .context("no results.bindings — this is not a SPARQL result set")?;
    let mut sites = Vec::with_capacity(rows.len());
    for row in rows {
        let uri = row["site"]["value"].as_str().unwrap_or_default();
        let Some(qid) = uri.rsplit('/').next().filter(|q| q.starts_with('Q')) else {
            continue;
        };
        let name = row["siteLabel"]["value"].as_str().unwrap_or_default().to_string();
        // A label that is still the QID means Wikidata has no English label.
        // Fall back to the official inscribed name off the statement; with
        // neither, record an empty name, because the QID pretending to be one
        // would put "Q170082" on a screen and `is_complete` would call it
        // complete.
        let name = if name == qid {
            row["officialName"]["value"].as_str().unwrap_or_default().to_string()
        } else {
            name
        };

        let split = |key: &str| -> Vec<String> {
            row[key]["value"]
                .as_str()
                .unwrap_or_default()
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect()
        };
        let mut state_parties = split("isos");
        state_parties.sort();
        state_parties.dedup();
        let mut criteria: Vec<String> = split("crits")
            .into_iter()
            .filter_map(|l| criterion_numeral(&l))
            .collect();
        criteria.sort_by_key(|c| criterion_order(c));
        criteria.dedup();

        let inscribed_year = row["inscribed"]["value"].as_str().and_then(|s| s.parse::<u32>().ok());
        let (lat, lon) = row["location"]["value"]
            .as_str()
            .and_then(parse_point)
            .map_or((None, None), |(la, lo)| (Some(la), Some(lo)));

        sites.push(HeritageSite {
            id: qid.to_string(),
            name,
            state_parties,
            inscribed_year,
            category: Category::from_criteria(&criteria),
            criteria,
            lat,
            lon,
            source: format!("wikidata:{qid}"),
        });
    }
    sites.sort_by(|a, b| a.id.cmp(&b.id));
    sites.dedup_by(|a, b| a.id == b.id);
    Ok(sites)
}

/// Wikidata labels the criteria items as "World Heritage selection criterion
/// (i)" and similar; this pulls the numeral out and rejects anything else
/// rather than passing a stray label through as a criterion.
fn criterion_numeral(label: &str) -> Option<String> {
    let inner = label.rsplit_once('(').and_then(|(_, r)| r.split_once(')')).map(|(l, _)| l);
    let candidate = inner.unwrap_or(label).trim().to_ascii_lowercase();
    let known = crate::model::CULTURAL_CRITERIA
        .iter()
        .chain(crate::model::NATURAL_CRITERIA.iter());
    known.into_iter().any(|k| *k == candidate).then_some(candidate)
}

fn criterion_order(c: &str) -> usize {
    crate::model::CULTURAL_CRITERIA
        .iter()
        .chain(crate::model::NATURAL_CRITERIA.iter())
        .position(|k| *k == c)
        .unwrap_or(usize::MAX)
}

/// `Point(lon lat)` — WKT, and note the order: **longitude first**. Getting
/// this backwards puts every site in the wrong hemisphere and still produces a
/// plausible-looking map, which is exactly the sort of defect that survives to
/// a screening.
fn parse_point(wkt: &str) -> Option<(f64, f64)> {
    let inner = wkt.trim().strip_prefix("Point(")?.strip_suffix(')')?;
    let (lon, lat) = inner.split_once(' ')?;
    Some((lat.trim().parse().ok()?, lon.trim().parse().ok()?))
}

pub fn load_snapshot(path: &Path) -> Result<HeritageSnapshot> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("reading the heritage snapshot from {}", path.display()))?;
    let snap: HeritageSnapshot = serde_json::from_str(&text)
        .with_context(|| format!("{} is not a heritage snapshot", path.display()))?;
    anyhow::ensure!(
        snap.version == SNAPSHOT_VERSION,
        "{} is snapshot version {}, this build reads {SNAPSHOT_VERSION}",
        path.display(),
        snap.version
    );
    Ok(snap)
}

/// How many sites each state party has, for the note and for a sanity check
/// against the WHC's own published per-country counts.
pub fn by_state_party(sites: &[HeritageSite]) -> BTreeMap<String, usize> {
    let mut out = BTreeMap::new();
    for s in sites {
        for iso in &s.state_parties {
            *out.entry(iso.clone()).or_insert(0) += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A real-shaped SPARQL result set, written out rather than fetched, so
    /// the parser has something to be wrong about without a network.
    const SAMPLE: &str = r#"{"results":{"bindings":[
      {"site":{"value":"http://www.wikidata.org/entity/Q10884"},
       "siteLabel":{"value":"Stonehenge, Avebury and Associated Sites"},
       "isos":{"value":"GB"},
       "crits":{"value":"World Heritage selection criterion (i),World Heritage selection criterion (ii),World Heritage selection criterion (iii)"},
       "inscribed":{"value":"1986"},
       "location":{"value":"Point(-1.826111 51.178889)"}},
      {"site":{"value":"http://www.wikidata.org/entity/Q170082"},
       "siteLabel":{"value":"Q170082"},
       "isos":{"value":"BY,EE,FI,LT,LV,MD,NO,RU,SE,UA"},
       "crits":{"value":"World Heritage selection criterion (ii),World Heritage selection criterion (iv),World Heritage selection criterion (vi)"},
       "inscribed":{"value":"2005"},
       "location":{"value":"Point(24.0 59.0)"}}
    ]}}"#;

    #[test]
    fn parses_a_real_shaped_result_set() {
        let sites = parse_sparql(SAMPLE).unwrap();
        assert_eq!(sites.len(), 2);
        let sh = sites.iter().find(|s| s.id == "Q10884").unwrap();
        assert_eq!(sh.name, "Stonehenge, Avebury and Associated Sites");
        assert_eq!(sh.state_parties, vec!["GB"]);
        assert_eq!(sh.criteria, vec!["i", "ii", "iii"]);
        assert_eq!(sh.category, Category::Cultural);
        assert_eq!(sh.inscribed_year, Some(1986));
        assert!(sh.is_complete());
        assert_eq!(sh.source, "wikidata:Q10884");
    }

    /// Longitude comes first in WKT. Stonehenge is at 51.18 N, 1.83 W — if
    /// these two ever swap, it lands in the Indian Ocean.
    #[test]
    fn a_point_is_longitude_then_latitude() {
        let sh = parse_sparql(SAMPLE).unwrap().into_iter().find(|s| s.id == "Q10884").unwrap();
        assert!((sh.lat.unwrap() - 51.178889).abs() < 1e-9, "lat {:?}", sh.lat);
        assert!((sh.lon.unwrap() + 1.826111).abs() < 1e-9, "lon {:?}", sh.lon);
    }

    /// The Struve Geodetic Arc crosses ten countries. A single-country field
    /// would have silently kept one.
    #[test]
    fn a_transnational_site_keeps_every_state_party() {
        let arc = parse_sparql(SAMPLE).unwrap().into_iter().find(|s| s.id == "Q170082").unwrap();
        assert_eq!(arc.state_parties.len(), 10);
        assert!(arc.state_parties.contains(&"NO".to_string()));
    }

    /// A label Wikidata could not resolve comes back as the QID. Recording it
    /// as the name would put "Q170082" on screen; an empty name is the honest
    /// record, and `is_complete` then reports the gap.
    #[test]
    fn an_unresolved_label_is_recorded_as_missing_not_as_the_qid() {
        let arc = parse_sparql(SAMPLE).unwrap().into_iter().find(|s| s.id == "Q170082").unwrap();
        assert_eq!(arc.name, "");
        assert!(!arc.is_complete());
    }

    #[test]
    fn a_stray_label_is_not_a_criterion() {
        assert_eq!(criterion_numeral("World Heritage selection criterion (iv)").as_deref(), Some("iv"));
        assert_eq!(criterion_numeral("cultural heritage"), None);
        assert_eq!(criterion_numeral("(xi)"), None);
    }
}
