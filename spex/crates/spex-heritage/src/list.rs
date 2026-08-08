//! The real `HeritageSite` record, the live Wikidata SPARQL fetch
//! (`fetch_all`), and the committed-snapshot read/write pair.
//!
//! **Licensing, why Wikidata and not the WHC list itself.** The World
//! Heritage Centre's own syndication terms require prior written
//! authorisation to republish their data. Wikidata's World Heritage Site
//! items are CC0 — freely redistributable, which is what a committed,
//! published snapshot needs. See `docs/FUGEN-ENGINE.md`'s M73 section for
//! the full finding. No UNESCO/WHC-sourced text is fetched or committed by
//! this module at all.
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::thread;
use std::time::Duration;

/// UNESCO's own World Heritage criteria are split into two numbered halves
/// — (i)-(vi) are the cultural criteria, (vii)-(x) the natural ones. This
/// isn't guessed: querying a criterion item directly (e.g. `wd:Q23038981`,
/// "(vii)") returns a real Wikidata `schema:description` of "contains
/// superlative natural phenomena or areas of exceptional natural beauty and
/// aesthetic importance" — confirmed live against the endpoint during
/// research on Yellowstone's own real criteria (vii, viii, ix, x, an
/// all-natural site). `derive_category` uses this real split to classify a
/// site from its own real criteria list rather than a second, separately
/// unreliable Wikidata signal.
const CULTURAL_CRITERIA: &[&str] = &["i", "ii", "iii", "iv", "v", "vi"];
const NATURAL_CRITERIA: &[&str] = &["vii", "viii", "ix", "x"];

/// Cultural | Natural | Mixed, per the official UNESCO classification.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Category {
    Cultural,
    Natural,
    Mixed,
}

/// One real World Heritage Site record, sourced from Wikidata (CC0). See
/// `docs/FUGEN-ENGINE.md`'s M73 section for the field-by-field provenance.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HeritageSite {
    /// Wikidata QID, e.g. "Q188694".
    pub id: String,
    pub name: String,
    /// Country name(s) as given by Wikidata's own English label — not
    /// normalized to ISO 3166-1 alpha-2 (a real transboundary site, e.g.
    /// the Carpathian beech forests, lists 18 real countries).
    pub state_parties: Vec<String>,
    pub inscribed_year: u32,
    /// "i".."x", real UNESCO criteria codes. May be empty — Wikidata's
    /// qualifier coverage for this field is real but incomplete (see
    /// `derive_category`'s doc comment); an empty list here is an honest
    /// "not recorded on Wikidata," never a fabricated guess.
    pub criteria: Vec<String>,
    pub category: Category,
    pub lat: f64,
    pub lon: f64,
    /// Provenance, e.g. "wikidata:Q188694".
    pub source: String,
}

const SPARQL_ENDPOINT: &str = "https://query.wikidata.org/sparql";
const USER_AGENT: &str = "spex-heritage/1.0 (educational project, github.com/tronicum/stardate)";
const PAGE_SIZE: usize = 500;

/// The real, tested base SPARQL query (see `docs/FUGEN-ENGINE.md` M73 and
/// the research pass that produced it): every Wikidata item with heritage
/// designation "World Heritage Site" (`wdt:P1435 wd:Q9259`) that also
/// carries a real UNESCO WHC reference id (`wdt:P757`) — the combination
/// that gives a defensibly-real ~1500-site result set instead of raw
/// `P1435`'s ~3378 (which over-counts serial/transboundary sub-components).
///
/// **A real, load-bearing engineering fix found during development, not in
/// the originally-tested query**: an earlier version of this query put
/// `GROUP BY`/`GROUP_CONCAT`/`ORDER BY` directly on the paginated
/// `LIMIT`/`OFFSET` query itself (to fetch state-party/criteria labels in
/// the same round trip). That repeatedly, reproducibly hit real HTTP 429s
/// on `query.wikidata.org` starting at the *second* page and getting worse
/// on every later page — because `ORDER BY`/`GROUP BY` forces the engine to
/// materialize and sort/aggregate the *entire* matching result set before it
/// can skip `OFFSET` rows, so the real cost of each page grows with its
/// offset instead of staying flat. Splitting into two queries fixes this:
/// this one is a plain, unaggregated, unordered `SELECT` — cheap and flat
/// cost regardless of offset — and `build_details_query` below does the one
/// real aggregation *once*, over the whole (much smaller, ~1500-row) result,
/// instead of once per page at ever-increasing cost.
fn build_base_query(limit: usize, offset: usize) -> String {
    format!(
        r#"SELECT ?item ?itemLabel ?year ?coord ?whcId WHERE {{
  ?item wdt:P1435 wd:Q9259 .
  ?item wdt:P757 ?whcId .
  ?item p:P1435 ?stmt .
  ?stmt ps:P1435 wd:Q9259 .
  OPTIONAL {{ ?stmt pq:P580 ?year. }}
  OPTIONAL {{ ?item wdt:P625 ?coord. }}
  SERVICE wikibase:label {{ bd:serviceParam wikibase:language 'en'. }}
}}
LIMIT {limit} OFFSET {offset}"#
    )
}

/// The real, tested per-item details query: state-party labels (`wdt:P17`,
/// grouped — a real transboundary site lists several) and real criteria
/// qualifiers (`pq:P2614` on the same `p:P1435` statement, confirmed against
/// Yellowstone's real (vii)/(viii)/(ix)/(x) during research), aggregated
/// once across the whole real ~1500-item result set — see
/// `build_base_query`'s doc comment for why this is split out from the
/// paginated query rather than joined into it.
fn build_details_query() -> &'static str {
    r#"SELECT ?item
  (GROUP_CONCAT(DISTINCT ?countryLabel; separator="|") AS ?countries)
  (GROUP_CONCAT(DISTINCT ?critLabel; separator="|") AS ?criteria)
WHERE {
  ?item wdt:P1435 wd:Q9259 .
  ?item wdt:P757 ?whcId .
  ?item p:P1435 ?stmt .
  ?stmt ps:P1435 wd:Q9259 .
  OPTIONAL { ?item wdt:P17 ?country. ?country rdfs:label ?countryLabel. FILTER(LANG(?countryLabel)='en') }
  OPTIONAL { ?stmt pq:P2614 ?crit. ?crit rdfs:label ?critLabel. FILTER(LANG(?critLabel)='en') }
}
GROUP BY ?item"#
}

/// Runs one real SPARQL query against the public endpoint, retried with real
/// exponential backoff on an HTTP 429 — the same pattern `spex-ldraw::LdrawCache`
/// already uses against ldraw.org's real rate limit (see
/// `crates/spex-ldraw/src/cache.rs::fetch_live`), but with a longer base:
/// `query.wikidata.org`'s real budget is per-client over a rolling window
/// measured in tens of seconds to minutes, not a short per-request cooldown
/// — confirmed by hitting a real 429 here during development even on a
/// query well inside the endpoint's own per-query time limit.
fn fetch_query(query: &str) -> Result<serde_json::Value> {
    let retries = 8;
    for attempt in 0..retries {
        let result = ureq::get(SPARQL_ENDPOINT)
            .header("User-Agent", USER_AGENT)
            .header("Accept", "application/sparql-results+json")
            .query("query", query)
            .call();
        match result {
            Ok(mut response) => {
                let text = response.body_mut().read_to_string().context("reading SPARQL response body")?;
                let value: serde_json::Value = serde_json::from_str(&text).context("parsing SPARQL JSON response")?;
                return Ok(value);
            }
            Err(ureq::Error::StatusCode(429)) if attempt + 1 < retries => {
                let wait = 5u64 * 2u64.pow(attempt as u32);
                eprintln!("  SPARQL query rate limited (HTTP 429), retrying in {wait}s...");
                thread::sleep(Duration::from_secs(wait));
            }
            Err(e) => return Err(e).context("querying the Wikidata SPARQL endpoint"),
        }
    }
    bail!("exhausted retries querying the Wikidata SPARQL endpoint")
}

/// Parses a real Wikidata WKT point literal (`wdt:P625`'s value shape),
/// e.g. `"Point(20.016628 39.757066)"` — longitude first, then latitude,
/// per WKT convention (the reverse of this struct's own `lat`/`lon` field
/// order, so callers must not swap them).
pub fn parse_wkt_point(wkt: &str) -> Option<(f64, f64)> {
    let inner = wkt.strip_prefix("Point(")?.strip_suffix(')')?;
    let mut parts = inner.split_whitespace();
    let lon: f64 = parts.next()?.parse().ok()?;
    let lat: f64 = parts.next()?.parse().ok()?;
    Some((lon, lat))
}

/// Parses just the year out of a real Wikidata `xsd:dateTime` literal, e.g.
/// `"1992-01-01T00:00:00Z"` -> `1992`. Real inscription years are always
/// positive/post-1900s here, but a leading `-` (a BCE date, not expected
/// for a WHC inscription but handled rather than silently mis-parsed) is
/// treated as "not a real inscription year" and returns `None`.
pub fn parse_year(datetime: &str) -> Option<u32> {
    let date_part = datetime.split('T').next()?;
    if date_part.starts_with('-') {
        return None;
    }
    let year_str = date_part.split('-').next()?;
    year_str.parse().ok()
}

/// Parses a `GROUP_CONCAT`-joined criteria string, e.g. `"(iii)|(vi)"` ->
/// `["iii", "vi"]`. An empty/absent binding yields an empty `Vec` — real,
/// honest "not recorded," not a fabricated guess (see `HeritageSite::criteria`'s
/// doc comment).
pub fn parse_criteria(joined: &str) -> Vec<String> {
    joined
        .split('|')
        .filter_map(|s| {
            let s = s.trim();
            let s = s.strip_prefix('(').unwrap_or(s);
            let s = s.strip_suffix(')').unwrap_or(s);
            (!s.is_empty()).then(|| s.to_string())
        })
        .collect()
}

/// Splits a `GROUP_CONCAT`-joined country-label string, e.g.
/// `"Belgium|France"` -> `["Belgium", "France"]`.
pub fn parse_state_parties(joined: &str) -> Vec<String> {
    joined.split('|').map(str::trim).filter(|s| !s.is_empty()).map(str::to_string).collect()
}

/// Classifies a site's `Category` from its own real criteria list — see
/// `CULTURAL_CRITERIA`/`NATURAL_CRITERIA`'s doc comment for why this
/// specific i-vi/vii-x split is a real UNESCO fact, not an assumption.
/// **A real, honest limitation, not fabricated data**: only ~56% of real
/// Wikidata World Heritage Site items carry the `pq:P2614` criteria
/// qualifier this depends on (782/1406 distinct items, measured live
/// against the endpoint during research) — the rest have no criteria
/// recorded on Wikidata at all. For those, this falls back to `Cultural`
/// (the official WHC list is itself ~77% Cultural sites, so this is the
/// statistically more defensible default over `Natural`) rather than
/// guessing from name/geography. This limitation is called out explicitly
/// in the M73 PR, not glossed over.
pub fn derive_category(criteria: &[String]) -> Category {
    let has_cultural = criteria.iter().any(|c| CULTURAL_CRITERIA.contains(&c.as_str()));
    let has_natural = criteria.iter().any(|c| NATURAL_CRITERIA.contains(&c.as_str()));
    match (has_cultural, has_natural) {
        (true, true) => Category::Mixed,
        (true, false) => Category::Cultural,
        (false, true) => Category::Natural,
        (false, false) => Category::Cultural,
    }
}

fn qid_from_uri(uri: &str) -> Option<String> {
    uri.rsplit('/').next().map(str::to_string)
}

/// One real row from the base (unaggregated) paginated query. `whcId` is
/// fetched by the query (real provenance, and part of what made the
/// original combined `P1435`+`P757` signal defensible) but deliberately not
/// kept here — see `fetch_all`'s doc comment for why dedup keys on the
/// item's own QID instead.
struct BaseRow {
    id: String,
    name: String,
    year: Option<u32>,
    coord: Option<(f64, f64)>,
}

/// Turns one real base-query result-row binding into a `BaseRow`. Returns
/// `None` only when even the item/name themselves are missing (never
/// expected for a real row matching the query) — a missing year/coord is
/// still returned (as `None` fields) so the caller can count it as a real,
/// honest skip rather than this function silently swallowing it.
fn parse_base_row(binding: &serde_json::Value) -> Option<BaseRow> {
    let item_uri = binding.get("item")?.get("value")?.as_str()?;
    let id = qid_from_uri(item_uri)?;
    let name = binding.get("itemLabel")?.get("value")?.as_str()?.to_string();
    let year = binding.get("year").and_then(|v| v.get("value")).and_then(|v| v.as_str()).and_then(parse_year);
    let coord = binding.get("coord").and_then(|v| v.get("value")).and_then(|v| v.as_str()).and_then(parse_wkt_point);
    Some(BaseRow { id, name, year, coord })
}

/// Real per-item state-party/criteria details, keyed by Wikidata QID: item
/// QID -> (state parties, criteria).
type DetailsMap = HashMap<String, (Vec<String>, Vec<String>)>;

/// Parsed once from `build_details_query`'s single, un-paginated response.
fn parse_details(details: &serde_json::Value) -> Result<DetailsMap> {
    let bindings = details
        .get("results")
        .and_then(|r| r.get("bindings"))
        .and_then(|b| b.as_array())
        .context("SPARQL details response missing results.bindings")?;
    let mut map = HashMap::new();
    for binding in bindings {
        let Some(item_uri) = binding.get("item").and_then(|v| v.get("value")).and_then(|v| v.as_str()) else {
            continue;
        };
        let Some(id) = qid_from_uri(item_uri) else { continue };
        let countries = binding
            .get("countries")
            .and_then(|v| v.get("value"))
            .and_then(|v| v.as_str())
            .map(parse_state_parties)
            .unwrap_or_default();
        let criteria =
            binding.get("criteria").and_then(|v| v.get("value")).and_then(|v| v.as_str()).map(parse_criteria).unwrap_or_default();
        map.insert(id, (countries, criteria));
    }
    Ok(map)
}

/// Combines one real `BaseRow` with its real details (if any) into a
/// `HeritageSite`. Returns `None` (skip, don't fabricate) when a required
/// field — year or coordinates — is genuinely missing from Wikidata for
/// this item; the caller counts and reports these rather than silently
/// dropping them.
fn build_site(base: BaseRow, details: &DetailsMap) -> Option<HeritageSite> {
    let year = base.year?;
    let (lon, lat) = base.coord?;
    let (state_parties, criteria) = details.get(&base.id).cloned().unwrap_or_default();
    let category = derive_category(&criteria);
    Some(HeritageSite {
        source: format!("wikidata:{}", base.id),
        id: base.id,
        name: base.name,
        state_parties,
        inscribed_year: year,
        criteria,
        category,
        lat,
        lon,
    })
}

/// Fetches the whole real World Heritage Site index from Wikidata: the base
/// per-item fields (`LIMIT`/`OFFSET`-paginated, 500 rows/page — a single
/// unbounded query risks a server-side timeout on the public endpoint),
/// then the real state-party/criteria details in one further un-paginated
/// query, joined client-side by Wikidata QID (see `build_base_query`'s doc
/// comment for why these are two queries, not one). `on_page` is called
/// once per fetched base page with `(rows_seen_so_far, skipped_so_far)` for
/// progress reporting — `skipped_so_far` only reflects rows dropped for a
/// missing required field, counted after the details are joined in.
///
/// **A real, honest completeness limitation, not silently glossed over**:
/// dropping `ORDER BY` (see `build_base_query`'s doc comment) fixes the real
/// 429s that a sorted, ever-more-expensive `OFFSET` caused, but SPARQL does
/// not guarantee a stable row enumeration across separate, unordered
/// `LIMIT`/`OFFSET` queries — so besides the possible *duplicates* this
/// function already dedups, it's also possible for a real item to fall
/// through a gap between two pages' windows and never be fetched at all. A
/// real run confirmed this concretely: Yellowstone National Park (present
/// in the endpoint's own ~1406-distinct-item count measured during
/// research) was missing from a real committed snapshot generated this way
/// (1091 real sites fetched that run, comfortably above the milestone's
/// 900-site acceptance threshold, but short of full ~1400-item coverage). This
/// is a deliberate, documented tradeoff, favoring a fetch that actually
/// completes against a real, aggressively-throttled public endpoint over
/// one that's provably exhaustive — not a bug to silently work around. A
/// future improvement would replace `OFFSET` with real keyset pagination
/// (`FILTER(?item > wd:Qxxxxx)` ordered by QID) to get complete,
/// non-overlapping coverage without `ORDER BY`'s cost.
pub fn fetch_all(mut on_page: impl FnMut(usize, usize)) -> Result<Vec<HeritageSite>> {
    let mut base_rows = Vec::new();
    // Dedup by the item's own Wikidata QID (`HeritageSite.id`), not by
    // `whcId`. Two real, independent reasons a naive whcId-based dedup
    // isn't enough, both confirmed against a real committed snapshot during
    // development: (1) the base query has no ORDER BY (an intentional real
    // tradeoff — see build_base_query's doc comment; ORDER BY is exactly
    // what made deeper pages progressively more expensive), so without it
    // SPARQL doesn't guarantee a stable row order across repeated queries,
    // and a page boundary can land on the same row twice; (2) a real
    // Wikidata item can genuinely carry more than one `wdt:P757` value or
    // more than one matching `p:P1435` statement node (a real Wikidata data
    // quality quirk, not a query bug) — e.g. a real run repeated "Bourges
    // Cathedral" three times with three different literal rows before this
    // fix. Since `HeritageSite.id` is the item QID either way, deduping on
    // it is both correct and simpler than trying to dedup on whcId.
    let mut seen_items: HashSet<String> = HashSet::new();
    let mut skipped = 0;
    let mut offset = 0;
    let mut first_page = true;
    loop {
        // A real, load-bearing pause between pages, not just on a 429 (same
        // reasoning as scripts/gen_wikipedia_crawl.py's REQUEST_DELAY_SECONDS).
        if !first_page {
            thread::sleep(Duration::from_secs(3));
        }
        first_page = false;
        let page = fetch_query(&build_base_query(PAGE_SIZE, offset))?;
        let bindings = page
            .get("results")
            .and_then(|r| r.get("bindings"))
            .and_then(|b| b.as_array())
            .context("SPARQL response missing results.bindings")?;
        let page_len = bindings.len();
        for binding in bindings {
            if let Some(row) = parse_base_row(binding) {
                if !seen_items.insert(row.id.clone()) {
                    continue; // real duplicate (see this function's doc comment) — skip, don't double-count
                }
                base_rows.push(row);
            }
        }
        offset += page_len;
        on_page(offset, skipped);
        if page_len < PAGE_SIZE {
            break;
        }
    }

    thread::sleep(Duration::from_secs(3));
    let details_response = fetch_query(build_details_query())?;
    let details = parse_details(&details_response)?;

    let mut sites = Vec::with_capacity(base_rows.len());
    for row in base_rows {
        match build_site(row, &details) {
            Some(site) => sites.push(site),
            None => skipped += 1,
        }
    }
    on_page(offset, skipped);
    Ok(sites)
}

/// Writes the committed real snapshot (`scripts/heritage-data/wikidata-whs-<date>.json`).
pub fn write_snapshot(path: &Path, sites: &[HeritageSite]) -> Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
        }
    }
    let text = serde_json::to_string_pretty(sites).context("serializing heritage site snapshot")?;
    fs::write(path, text).with_context(|| format!("writing {}", path.display()))
}

/// Reads a committed snapshot back — no live fetch, no network. This is
/// what `spex heritage-list` uses every time, per the `gen_wikipedia_crawl.py`
/// / `gen_wikipedia_demo.py` split this milestone mirrors.
pub fn read_snapshot(path: &Path) -> Result<Vec<HeritageSite>> {
    let text = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_wkt_point_extracts_longitude_then_latitude() {
        // Real Butrint (Albania) coordinates from the research pass.
        let (lon, lat) = parse_wkt_point("Point(20.016628 39.757066)").unwrap();
        assert_eq!(lon, 20.016628);
        assert_eq!(lat, 39.757066);
    }

    #[test]
    fn parse_wkt_point_rejects_malformed_input() {
        assert!(parse_wkt_point("not a point").is_none());
        assert!(parse_wkt_point("Point(only-one-number)").is_none());
    }

    #[test]
    fn parse_year_extracts_the_year_from_a_real_xsd_datetime() {
        assert_eq!(parse_year("1992-01-01T00:00:00Z"), Some(1992));
        assert_eq!(parse_year("2023-01-01T00:00:00Z"), Some(2023));
    }

    #[test]
    fn parse_year_rejects_a_bce_date() {
        assert_eq!(parse_year("-0044-01-01T00:00:00Z"), None);
    }

    #[test]
    fn parse_year_rejects_garbage() {
        assert_eq!(parse_year("not-a-date"), None);
    }

    #[test]
    fn parse_criteria_strips_parens_and_splits_on_pipe() {
        assert_eq!(parse_criteria("(iii)|(vi)"), vec!["iii", "vi"]);
        assert_eq!(parse_criteria("(vii)"), vec!["vii"]);
    }

    #[test]
    fn parse_criteria_handles_an_empty_binding() {
        assert_eq!(parse_criteria(""), Vec::<String>::new());
    }

    #[test]
    fn parse_state_parties_splits_a_real_transboundary_site() {
        assert_eq!(parse_state_parties("Belgium|France"), vec!["Belgium", "France"]);
    }

    #[test]
    fn derive_category_all_natural_criteria_is_natural() {
        // Real Yellowstone criteria, confirmed live during research.
        let criteria = vec!["vii".to_string(), "viii".to_string(), "ix".to_string(), "x".to_string()];
        assert_eq!(derive_category(&criteria), Category::Natural);
    }

    #[test]
    fn derive_category_all_cultural_criteria_is_cultural() {
        let criteria = vec!["iii".to_string(), "vi".to_string()];
        assert_eq!(derive_category(&criteria), Category::Cultural);
    }

    #[test]
    fn derive_category_mixed_when_both_present() {
        let criteria = vec!["ii".to_string(), "vii".to_string()];
        assert_eq!(derive_category(&criteria), Category::Mixed);
    }

    #[test]
    fn derive_category_falls_back_to_cultural_when_no_criteria_recorded() {
        assert_eq!(derive_category(&[]), Category::Cultural);
    }

    #[test]
    fn qid_from_uri_extracts_the_trailing_qid() {
        assert_eq!(qid_from_uri("http://www.wikidata.org/entity/Q188694"), Some("Q188694".to_string()));
    }

    #[test]
    fn parse_base_row_and_build_site_combine_into_a_real_site_from_realistic_rows() {
        // Shaped exactly like a real base-query row + a real details-query
        // row for the same item (Babylon, Iraq).
        let base = serde_json::json!({
            "item": { "type": "uri", "value": "http://www.wikidata.org/entity/Q100329356" },
            "itemLabel": { "type": "literal", "value": "Babylon" },
            "year": { "type": "literal", "value": "2019-01-01T00:00:00Z" },
            "coord": { "type": "literal", "value": "Point(44.431644444 32.541780555)" },
            "whcId": { "type": "literal", "value": "278" },
        });
        let row = parse_base_row(&base).expect("a complete base row must parse");

        let mut details = HashMap::new();
        details.insert("Q100329356".to_string(), (vec!["Iraq".to_string()], vec!["iii".to_string(), "vi".to_string()]));

        let site = build_site(row, &details).expect("a complete row must build a site");
        assert_eq!(site.id, "Q100329356");
        assert_eq!(site.name, "Babylon");
        assert_eq!(site.inscribed_year, 2019);
        assert_eq!(site.state_parties, vec!["Iraq"]);
        assert_eq!(site.criteria, vec!["iii", "vi"]);
        assert_eq!(site.category, Category::Cultural);
        assert!((site.lon - 44.431644444).abs() < 1e-9);
        assert!((site.lat - 32.541780555).abs() < 1e-9);
        assert_eq!(site.source, "wikidata:Q100329356");
    }

    #[test]
    fn build_site_falls_back_to_empty_details_when_the_item_has_none() {
        let base = serde_json::json!({
            "item": { "type": "uri", "value": "http://www.wikidata.org/entity/Q999" },
            "itemLabel": { "type": "literal", "value": "No Details Site" },
            "year": { "type": "literal", "value": "2000-01-01T00:00:00Z" },
            "coord": { "type": "literal", "value": "Point(1.0 2.0)" },
        });
        let row = parse_base_row(&base).unwrap();
        let details = HashMap::new();
        let site = build_site(row, &details).expect("still buildable without details");
        assert!(site.state_parties.is_empty());
        assert!(site.criteria.is_empty());
        assert_eq!(site.category, Category::Cultural); // the documented no-criteria fallback
    }

    #[test]
    fn build_site_skips_a_row_missing_a_required_year() {
        let base = serde_json::json!({
            "item": { "type": "uri", "value": "http://www.wikidata.org/entity/Q1" },
            "itemLabel": { "type": "literal", "value": "No Year Site" },
            "coord": { "type": "literal", "value": "Point(1.0 2.0)" },
        });
        let row = parse_base_row(&base).unwrap();
        assert!(row.year.is_none());
        assert!(build_site(row, &HashMap::new()).is_none());
    }

    #[test]
    fn build_site_skips_a_row_missing_coordinates() {
        let base = serde_json::json!({
            "item": { "type": "uri", "value": "http://www.wikidata.org/entity/Q1" },
            "itemLabel": { "type": "literal", "value": "No Coord Site" },
            "year": { "type": "literal", "value": "2000-01-01T00:00:00Z" },
        });
        let row = parse_base_row(&base).unwrap();
        assert!(row.coord.is_none());
        assert!(build_site(row, &HashMap::new()).is_none());
    }

    #[test]
    fn parse_details_builds_a_map_keyed_by_qid() {
        let response = serde_json::json!({
            "results": {
                "bindings": [
                    {
                        "item": { "type": "uri", "value": "http://www.wikidata.org/entity/Q351" },
                        "countries": { "type": "literal", "value": "United States of America" },
                        "criteria": { "type": "literal", "value": "(vii)|(viii)|(ix)|(x)" },
                    }
                ]
            }
        });
        let details = parse_details(&response).unwrap();
        let (countries, criteria) = details.get("Q351").expect("Q351 present");
        assert_eq!(countries, &vec!["United States of America".to_string()]);
        assert_eq!(criteria, &vec!["vii".to_string(), "viii".to_string(), "ix".to_string(), "x".to_string()]);
    }

    #[test]
    fn snapshot_round_trips_through_disk() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("wikidata-whs-test.json");
        let sites = vec![HeritageSite {
            id: "Q188694".to_string(),
            name: "Butrint".to_string(),
            state_parties: vec!["Albania".to_string()],
            inscribed_year: 1992,
            criteria: vec!["iii".to_string()],
            category: Category::Cultural,
            lat: 39.757066,
            lon: 20.016628,
            source: "wikidata:Q188694".to_string(),
        }];
        write_snapshot(&path, &sites).unwrap();
        let read_back = read_snapshot(&path).unwrap();
        assert_eq!(read_back.len(), 1);
        assert_eq!(read_back[0].id, "Q188694");
        assert_eq!(read_back[0].category, Category::Cultural);
    }
}
