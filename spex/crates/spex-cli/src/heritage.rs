//! `spex heritage-index` (the real, rarely-run live SPARQL fetch) and
//! `spex heritage-list` (reads the committed snapshot, prints a table) —
//! the exact `gen_wikipedia_crawl.py`/`gen_wikipedia_demo.py` split this
//! repo already established for a slow/rate-limited real external fetch
//! vs. a fast, CI-safe read of what it produced. See
//! `docs/FUGEN-ENGINE.md`'s M73 section.
use anyhow::{Context, Result};
use spex_heritage::{is_buildable, load_curation, read_snapshot, write_snapshot, Category, HeritageSite};
use std::io::IsTerminal;
use std::path::{Path, PathBuf};

/// The real live fetch: queries the Wikidata SPARQL endpoint end to end
/// (paginated) and writes the result as the committed snapshot. Not wired
/// into `scripts/walkthrough.sh` — deliberately, so routine regeneration
/// stays fast and network-independent (see module doc comment).
pub fn run_index(out: &Path) -> Result<()> {
    println!("querying the real Wikidata SPARQL endpoint for World Heritage Site records...");
    println!("(a live network fetch, paginated 500 rows/page — this is not run routinely, see docs/FUGEN-ENGINE.md M73)");
    let sites = spex_heritage::fetch_all(|seen, skipped| {
        println!("  ...{seen} real rows fetched so far ({skipped} skipped so far for missing required fields)");
    })?;
    println!("fetched {} real World Heritage Site records", sites.len());
    write_snapshot(out, &sites)?;
    println!("wrote real committed snapshot to {}", out.display());
    Ok(())
}

/// Finds the most recently dated committed snapshot under `dir`
/// (`wikidata-whs-YYYY-MM-DD.json` sorts correctly as a plain string, since
/// ISO 8601 dates do) so `spex heritage-list` doesn't need a fixed filename
/// hardcoded — a fresh `spex heritage-index` run naturally becomes the new
/// default without any code change.
pub fn find_latest_snapshot(dir: &Path) -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with("wikidata-whs-") && n.ends_with(".json"))
                .unwrap_or(false)
        })
        .collect();
    candidates.sort();
    candidates.pop()
}

const DEFAULT_HERITAGE_DATA_DIR: &str = "scripts/heritage-data";

pub fn run_list(buildable_only: bool, snapshot: Option<PathBuf>, curation_path: &Path) -> Result<()> {
    let snapshot_path = match snapshot {
        Some(p) => p,
        None => find_latest_snapshot(Path::new(DEFAULT_HERITAGE_DATA_DIR)).with_context(|| {
            format!(
                "no {DEFAULT_HERITAGE_DATA_DIR}/wikidata-whs-*.json snapshot found — run \
                 `spex heritage-index -o {DEFAULT_HERITAGE_DATA_DIR}/wikidata-whs-<date>.json` first, or pass --snapshot explicitly"
            )
        })?,
    };
    let sites = read_snapshot(&snapshot_path).with_context(|| format!("reading snapshot {}", snapshot_path.display()))?;

    let shown: Vec<&HeritageSite> = if buildable_only {
        let curation = load_curation(curation_path)
            .with_context(|| format!("loading curation file {} (required for --buildable)", curation_path.display()))?;
        sites.iter().filter(|s| is_buildable(s, &curation)).collect()
    } else {
        sites.iter().collect()
    };

    print!("{}", render_table(&shown, &snapshot_path, sites.len(), buildable_only));
    Ok(())
}

fn category_label(category: Category) -> &'static str {
    match category {
        Category::Cultural => "Cultural",
        Category::Natural => "Natural",
        Category::Mixed => "Mixed",
    }
}

/// Fixed, non-gradient colors per category — the same "a category, not a
/// magnitude" choice `graph-diff --merge`'s viewer coloring made for
/// added/removed/changed/unchanged (see `crates/spex-graph/src/layout.rs`).
fn category_color(category: Category) -> [u8; 3] {
    match category {
        Category::Cultural => [90, 160, 255],  // blue
        Category::Natural => [90, 210, 120],   // green
        Category::Mixed => [230, 190, 60],     // gold
    }
}

fn render_table(sites: &[&HeritageSite], snapshot_path: &Path, total_in_snapshot: usize, buildable_only: bool) -> String {
    let use_color = std::io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none();
    let mut out = String::new();

    let heading = if buildable_only {
        format!("spex heritage-list --buildable  ({} of {total_in_snapshot} real sites, from {})\n\n", sites.len(), snapshot_path.display())
    } else {
        format!("spex heritage-list  ({total_in_snapshot} real sites, from {})\n\n", snapshot_path.display())
    };
    out.push_str(&heading);

    if sites.is_empty() {
        out.push_str("(no sites to show)\n");
        return out;
    }

    let name_width = sites.iter().map(|s| s.name.chars().count()).max().unwrap_or(4).clamp(4, 48);
    out.push_str(&format!(
        "{:<10} {:<width$} {:<6} {:<8} {:<10} {}\n",
        "QID",
        "NAME",
        "YEAR",
        "CATEGORY",
        "CRITERIA",
        "STATE PARTIES",
        width = name_width
    ));
    out.push_str(&"-".repeat(name_width + 60)); // separator; exact width isn't load-bearing
    out.push('\n');

    let mut cultural = 0usize;
    let mut natural = 0usize;
    let mut mixed = 0usize;
    for site in sites {
        match site.category {
            Category::Cultural => cultural += 1,
            Category::Natural => natural += 1,
            Category::Mixed => mixed += 1,
        }
        let name = truncate(&site.name, name_width);
        let criteria = if site.criteria.is_empty() { "-".to_string() } else { site.criteria.join(",") };
        let countries = if site.state_parties.is_empty() { "-".to_string() } else { site.state_parties.join(", ") };
        let row = format!(
            "{:<10} {:<width$} {:<6} {:<8} {:<10} {}",
            site.id,
            name,
            site.inscribed_year,
            category_label(site.category),
            criteria,
            countries,
            width = name_width
        );
        if use_color {
            let [r, g, b] = category_color(site.category);
            out.push_str(&format!("\x1b[38;2;{r};{g};{b}m{row}\x1b[0m\n"));
        } else {
            out.push_str(&row);
            out.push('\n');
        }
    }

    out.push_str(&format!("\n{} shown — Cultural: {cultural}, Natural: {natural}, Mixed: {mixed}\n", sites.len()));
    out
}

fn truncate(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        let mut t: String = s.chars().take(max_chars.saturating_sub(1)).collect();
        t.push('…');
        t
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use spex_heritage::HeritageSite;

    fn site(id: &str, name: &str, category: Category) -> HeritageSite {
        HeritageSite {
            id: id.to_string(),
            name: name.to_string(),
            state_parties: vec!["Testland".to_string()],
            inscribed_year: 1999,
            criteria: vec!["i".to_string()],
            category,
            lat: 1.0,
            lon: 2.0,
            source: format!("wikidata:{id}"),
        }
    }

    #[test]
    fn find_latest_snapshot_picks_the_lexicographically_greatest_dated_file() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("wikidata-whs-2025-01-01.json"), "[]").unwrap();
        std::fs::write(dir.path().join("wikidata-whs-2026-08-08.json"), "[]").unwrap();
        std::fs::write(dir.path().join("wikidata-whs-2025-12-31.json"), "[]").unwrap();
        std::fs::write(dir.path().join("not-a-snapshot.json"), "[]").unwrap();

        let latest = find_latest_snapshot(dir.path()).unwrap();
        assert_eq!(latest.file_name().unwrap().to_str().unwrap(), "wikidata-whs-2026-08-08.json");
    }

    #[test]
    fn find_latest_snapshot_returns_none_when_dir_has_no_snapshots() {
        let dir = tempfile::tempdir().unwrap();
        assert!(find_latest_snapshot(dir.path()).is_none());
    }

    #[test]
    fn render_table_lists_every_site_and_summarizes_by_category() {
        let a = site("Q1", "Alpha Site", Category::Cultural);
        let b = site("Q2", "Beta Site", Category::Natural);
        let sites = vec![&a, &b];
        let text = render_table(&sites, Path::new("snap.json"), 2, false);
        assert!(text.contains("Q1"));
        assert!(text.contains("Alpha Site"));
        assert!(text.contains("Q2"));
        assert!(text.contains("Cultural: 1, Natural: 1, Mixed: 0"));
    }

    #[test]
    fn render_table_notes_buildable_filtering_in_the_heading() {
        let a = site("Q1", "Alpha Site", Category::Cultural);
        let sites = vec![&a];
        let text = render_table(&sites, Path::new("snap.json"), 5, true);
        assert!(text.starts_with("spex heritage-list --buildable"));
        assert!(text.contains("1 of 5 real sites"));
    }

    #[test]
    fn truncate_leaves_short_strings_alone() {
        assert_eq!(truncate("short", 20), "short");
    }

    #[test]
    fn truncate_shortens_and_marks_long_strings() {
        let truncated = truncate("a very very long site name indeed", 10);
        assert_eq!(truncated.chars().count(), 10);
        assert!(truncated.ends_with('…'));
    }
}
