//! `spex heritage-index` (fetches, rarely) and `spex heritage-list` (reads the
//! committed snapshot, which is what everything else does).
//!
//! The same split as `gen_wikipedia_crawl.py` / `gen_wikipedia_demo.py`: a
//! build that queries a live endpoint is a build whose output depends on a
//! Tuesday, and a snapshot you cannot re-fetch is a snapshot you cannot argue
//! with. Both, or neither.
use anyhow::{Context, Result};
use spex_heritage::{by_state_party, Category, Curation, Verdict};
use std::path::Path;

pub fn index(out: &Path, fetched: &str) -> Result<()> {
    println!("querying {} ...", spex_heritage::ENDPOINT);
    let snap = spex_heritage::fetch(fetched)?;
    let complete = snap.sites.iter().filter(|s| s.is_complete()).count();
    let categorised = snap.sites.iter().filter(|s| s.is_categorised()).count();
    let by_cat = |c: Category| snap.sites.iter().filter(|s| s.category == c).count();
    println!(
        "  {} site(s); {complete} placeable and labelled; {categorised} with criteria",
        snap.sites.len()
    );
    println!(
        "  cultural {}, natural {}, mixed {}, uncategorised {}",
        by_cat(Category::Cultural),
        by_cat(Category::Natural),
        by_cat(Category::Mixed),
        by_cat(Category::Unknown)
    );
    let parties = by_state_party(&snap.sites);
    println!("  {} state part(ies)", parties.len());
    if let Some(dir) = out.parent() {
        std::fs::create_dir_all(dir).ok();
    }
    std::fs::write(out, serde_json::to_string_pretty(&snap)? + "\n")
        .with_context(|| format!("writing {}", out.display()))?;
    println!("  wrote {}", out.display());
    println!(
        "\nAcceptance criterion 1 wants this count recorded against the WHC's own published\n\
         total for the same date, with the delta explained. That comparison is a reading of\n\
         a WHC page by a person; no WHC text goes into a committed file."
    );
    Ok(())
}

pub fn list(snapshot: &Path, curation_path: &Path, buildable_only: bool, limit: usize) -> Result<()> {
    let snap = spex_heritage::load_snapshot(snapshot)?;
    println!(
        "{} — {} site(s), fetched {}",
        snapshot.display(),
        snap.sites.len(),
        snap.fetched
    );

    // No curation file is a legitimate state, not an error: M73's code lands
    // before its curation does, and `heritage-list` without `--buildable` is
    // exactly how a person reads the snapshot in order to write one.
    let curation = if curation_path.exists() {
        let c = Curation::load(curation_path)?;
        let errs = c.validate();
        if !errs.is_empty() {
            for e in &errs {
                eprintln!("  ! {e}");
            }
            anyhow::bail!("{} has {} problem(s)", curation_path.display(), errs.len());
        }
        println!(
            "curation: {} buildable, {} excluded, last read by {} on {}",
            c.buildable.len(),
            c.excluded.len(),
            c.reviewer,
            c.reviewed
        );
        Some(c)
    } else {
        if buildable_only {
            anyhow::bail!(
                "--buildable needs a curation file and there is none at {}. \
                 The filter fails closed: with nothing classified, nothing is buildable.",
                curation_path.display()
            );
        }
        println!("curation: none at {} — showing the raw snapshot", curation_path.display());
        None
    };

    let mut shown = 0;
    let mut counts = (0usize, 0usize, 0usize, 0usize, 0usize);
    println!(
        "\n{:<10} {:<52} {:<8} {:<5} {:<26} {}",
        "QID", "name", "parties", "year", "criteria", "verdict"
    );
    for site in &snap.sites {
        let v = curation.as_ref().map(|c| spex_heritage::verdict(site, c));
        match v {
            Some(Verdict::Buildable { .. }) => counts.0 += 1,
            Some(Verdict::Excluded(_)) => counts.1 += 1,
            Some(Verdict::NotCultural(_)) => counts.2 += 1,
            Some(Verdict::Unclassified) => counts.3 += 1,
            Some(Verdict::Contested { .. }) => counts.4 += 1,
            None => {}
        }
        if buildable_only && !matches!(v, Some(Verdict::Buildable { .. })) {
            continue;
        }
        if shown >= limit {
            continue;
        }
        shown += 1;
        let verdict = match &v {
            Some(Verdict::Buildable { tier }) => format!("buildable, tier {tier}"),
            Some(Verdict::Excluded(r)) => format!("excluded: {r:?}"),
            Some(Verdict::NotCultural(c)) => format!("{c:?}"),
            Some(Verdict::Unclassified) => "unclassified".into(),
            Some(Verdict::Contested { tier }) => format!("CONTESTED (curated tier {tier}, Wikidata says natural)"),
            None => format!("{:?}", site.category),
        };
        println!(
            "{:<10} {:<52} {:<8} {:<5} {:<26} {}",
            site.id,
            truncate(&site.name, 52),
            truncate(&site.state_parties.join("/"), 8),
            site.inscribed_year.map(|y| y.to_string()).unwrap_or_else(|| "-".into()),
            truncate(&site.criteria.join(" "), 26),
            verdict
        );
    }
    if curation.is_some() {
        println!(
            "\nbuildable {}, excluded {}, not cultural {}, unclassified {}, contested {} \
             (unclassified is OUT — the filter fails closed; contested is a person and Wikidata disagreeing, and is also OUT until settled)",
            counts.0, counts.1, counts.2, counts.3, counts.4
        );
    }
    if !buildable_only && snap.sites.len() > shown {
        println!("({} of {} shown; --limit for more)", shown, snap.sites.len());
    }
    Ok(())
}

fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        format!("{}...", s.chars().take(n.saturating_sub(3)).collect::<String>())
    }
}
