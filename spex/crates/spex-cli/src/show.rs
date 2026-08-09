//! `spex show-build` — compile an authored `show.json` into a playable show
//! directory.
//!
//! Three things happen here, and only the middle one is interesting:
//!
//! 1. Every scene the document references is built into a real mesh bundle
//!    (M52/M53), exactly as `spex mesh-model` would.
//! 2. The timeline is resolved for one duration and one seed
//!    (`spex_show::resolve`), and every glob is bound to the instance indices
//!    of the bundle that was just built (`spex_show::compile`).
//! 3. `show-resolved.json` is written.
//!
//! # Two source kinds have no generator yet
//!
//! A scene can be an `.ldr` file, a `build` recipe (M72), a `flag` (M75) or a
//! `heritage` site (M73). Only the first exists. Rather than emit a show that
//! silently has no Stonehenge in it, `show-build` **fails and names the
//! milestone**, and `--skip-unbuildable` is the explicit way to say "build the
//! part that exists" — which then prints exactly which scenes were dropped and
//! which shots lost geometry. A missing scene that produces no message is the
//! failure mode worth engineering against: at runtime it is simply an empty
//! frame with no error anywhere.
//!
//! # `--no-bundles`
//!
//! Resolving does not need geometry. `--no-bundles` writes the timeline alone,
//! which is what the duration arithmetic, the schema tests and any tooling
//! about *time* actually want, and it turns a minute of LDraw resolution into
//! a few milliseconds. The resolved document records the difference honestly:
//! its target bindings carry the glob and no instance list, and the schema
//! says that is the one case where a player may expand the glob itself.

use anyhow::{bail, Context, Result};
use spex_ldraw::LdrawCache;
use spex_show::{compile::SceneInstances, ResolveOptions, ResolvedShow, SceneSource, Show};
use std::path::Path;

pub struct BuildOptions {
    /// Repeatable. Each value is one cut of the same document; they share one
    /// `bundles/` directory, because the geometry does not change between a
    /// four-minute and a sixty-minute screening — only the timeline does.
    pub target_sec: Vec<f64>,
    pub seed: Option<u64>,
    pub endless: bool,
    pub no_bundles: bool,
    pub skip_unbuildable: bool,
    pub crease: f64,
}

/// `cuts.json` — which resolved documents this directory holds.
///
/// The viewer needs this to answer `?duration=600` without probing four
/// filenames and treating three 404s as normal. Written even for a single cut,
/// so the reader has one shape to handle rather than two.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct CutsIndex {
    version: u32,
    default: String,
    cuts: Vec<CutEntry>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct CutEntry {
    label: String,
    duration_sec: f64,
    endless: bool,
    file: String,
}

const CUTS_INDEX_VERSION: u32 = 1;

pub fn build(show_path: &Path, out: &Path, cache_dir: &Path, opts: &BuildOptions) -> Result<()> {
    let show: Show = spex_show::load(show_path)?;
    println!(
        "{:?} — {} movement(s), {} shot(s), {} scene(s), base {} bars = {:.3} s",
        show.title,
        show.movements.len(),
        show.shots().count(),
        show.scenes.len(),
        show.base_duration_bars,
        show.base_duration_sec()
    );

    std::fs::create_dir_all(out).with_context(|| format!("creating {}", out.display()))?;

    // The geometry is built exactly once, before any timeline is resolved.
    // Four cuts of one document are four resolutions of the same screenplay;
    // building the same bundles four times would be four chances for them to
    // differ, on top of four times the minutes.
    let instances = if opts.no_bundles {
        println!("--no-bundles: resolving the timeline only, no geometry built");
        SceneInstances::new()
    } else {
        build_bundles(&show, out, cache_dir, opts)?
    };
    let dropped = dropped_scenes(&show, opts);

    let mut requests: Vec<(f64, bool)> = opts.target_sec.iter().map(|s| (*s, false)).collect();
    if opts.endless {
        requests.push((show.base_duration_sec(), true));
    }
    if requests.is_empty() {
        requests.push((show.base_duration_sec(), false));
    }

    let mut entries: Vec<CutEntry> = Vec::new();
    for (i, (target_sec, endless)) in requests.iter().enumerate() {
        let resolve_opts = ResolveOptions {
            target_sec: *target_sec,
            seed: opts.seed.unwrap_or(show.seed),
            endless: *endless,
        };
        let mut resolved = spex_show::resolve(&show, &resolve_opts)?;
        drop_unbuildable(&mut resolved, &dropped);

        if !instances.is_empty() {
            let unmatched = spex_show::bind_targets(&mut resolved, &instances);
            // Never silent. A glob that matches nothing animates nothing, and
            // produces no error at any later point. Reported for the first cut
            // only: the globs are a property of the document, not of the
            // duration, so repeating them per cut is noise that would train a
            // reader to skip the section.
            if i == 0 {
                for u in &unmatched {
                    println!("  ! target matched no instance — {u}");
                }
                if !unmatched.is_empty() {
                    println!(
                        "  {} target(s) matched nothing. `*` does not cross a `/`, and a bundle's \
                         ids are <part>/<n>, so \"scene/*\" is a common way to write \"scene/**\" \
                         by mistake.",
                        unmatched.len()
                    );
                }
            }
        }

        let label = cut_label(*target_sec, *endless);
        // The first cut is always `show-resolved.json`. That keeps a
        // single-cut directory byte-identical in shape to every one built
        // before M66, so nothing that already reads one has to learn a name.
        let file = if i == 0 {
            "show-resolved.json".to_string()
        } else {
            format!("show-resolved-{label}.json")
        };
        let path = out.join(&file);
        let json = serde_json::to_string_pretty(&resolved)?;
        std::fs::write(&path, format!("{json}\n"))
            .with_context(|| format!("writing {}", path.display()))?;
        report(&resolved, &path);

        entries.push(CutEntry {
            label,
            duration_sec: resolved.duration_sec,
            endless: resolved.endless,
            file,
        });
    }

    let index = CutsIndex {
        version: CUTS_INDEX_VERSION,
        default: entries[0].label.clone(),
        cuts: entries,
    };
    let index_path = out.join("cuts.json");
    std::fs::write(&index_path, format!("{}\n", serde_json::to_string_pretty(&index)?))
        .with_context(|| format!("writing {}", index_path.display()))?;
    println!(
        "wrote {} — {} cut(s): {}",
        index_path.display(),
        index.cuts.len(),
        index.cuts.iter().map(|c| c.label.as_str()).collect::<Vec<_>>().join(", ")
    );
    Ok(())
}

/// `240`, `600`, `3600`, `endless` — the labels `?duration=` uses.
///
/// A whole number of seconds when it is one, because those are the four names
/// the spec gives and a URL parameter reading `duration=240.000` would be a
/// different string from the one anybody types.
fn cut_label(target_sec: f64, endless: bool) -> String {
    if endless {
        return "endless".to_string();
    }
    if (target_sec - target_sec.round()).abs() < 1e-6 {
        format!("{}", target_sec.round() as i64)
    } else {
        format!("{target_sec:.3}")
    }
}

/// Which scenes have no generator yet, and why — computed without building
/// anything, so every cut drops exactly the same set.
fn unbuildable(show: &Show) -> Vec<(String, String)> {
    show.scenes
        .iter()
        .filter_map(|scene| match &scene.source {
            SceneSource::Ldr { .. } => None,
            SceneSource::Build { recipe } => {
                Some((scene.id.clone(), format!("build recipe {recipe:?} — M72")))
            }
            SceneSource::Flag { flag } => Some((scene.id.clone(), format!("flag {flag:?} — M75"))),
            SceneSource::Heritage { site_id } => {
                Some((scene.id.clone(), format!("heritage site {site_id:?} — M73")))
            }
            // A real generator exists (see build_bundles below) — an
            // `ankerstein` scene is not unbuildable.
            SceneSource::Ankerstein { .. } => None,
        })
        .collect()
}

fn dropped_scenes(show: &Show, opts: &BuildOptions) -> Vec<String> {
    if opts.no_bundles || !opts.skip_unbuildable {
        return Vec::new();
    }
    unbuildable(show).into_iter().map(|(id, _)| id).collect()
}

/// Removes the scenes that were never built from one resolved cut.
fn drop_unbuildable(resolved: &mut ResolvedShow, dropped: &[String]) {
    if dropped.is_empty() {
        return;
    }
    resolved.scenes.retain(|s| !dropped.contains(&s.id));
    for shot in &mut resolved.shots {
        shot.scenes.retain(|s| !dropped.contains(s));
    }
}

fn build_bundles(
    show: &Show,
    out: &Path,
    cache_dir: &Path,
    opts: &BuildOptions,
) -> Result<SceneInstances> {
    let cache = LdrawCache::new(cache_dir);
    let mut instances = SceneInstances::new();
    // One source of truth for "which scenes have no generator", shared with
    // `dropped_scenes` — so what is built and what is removed from every cut
    // cannot disagree.
    let skipped = unbuildable(show);

    for scene in &show.scenes {
        let dir = out.join("bundles").join(&scene.id);
        let (stats, label) = match &scene.source {
            SceneSource::Ldr { path } => {
                let parsed = crate::mesh::parse_scene_arg(&cache, path)
                    .with_context(|| format!("scene {:?} from {path:?}", scene.id))?;
                let stats = crate::mesh::build_scene_bundle(&cache, &parsed, opts.crease, &dir)?;
                (stats, path.clone())
            }
            SceneSource::Ankerstein { scene: scene_path, color } => {
                // Not an LDraw scene at all — a real Ankerstein assembly,
                // resolved through `spex_ankerstein::to_part_geometry`
                // (crates/spex-ankerstein/src/geometry.rs) rather than
                // `resolve_part_full`. Proves the same mesh-bundle pipeline
                // (`MeshBundleBuilder::add_part`/`add_instance`, `write`)
                // generalizes beyond LDraw/LEGO, which is the whole point.
                let parsed = spex_ankerstein::parse_scene(Path::new(scene_path))
                    .with_context(|| format!("scene {:?}: parsing Ankerstein scene {scene_path:?}", scene.id))?;
                let color_code = crate::ankerstein::color_code_for(color)
                    .with_context(|| format!("scene {:?}", scene.id))?;
                let stats = crate::ankerstein::build_scene_mesh_bundle(&parsed, color_code, opts.crease, &dir)?;
                (stats, scene_path.clone())
            }
            _ => continue,
        };
        println!(
            "  {} <- {label}: {} instance(s), {} part(s), {} triangle(s)",
            scene.id, stats.instance_count, stats.part_count, stats.total_triangles
        );
        instances.insert(scene.id.clone(), read_instance_ids(&dir)?);
    }

    if !skipped.is_empty() {
        for (id, why) in &skipped {
            println!("  - scene {id:?} has no generator yet: {why}");
        }
        if !opts.skip_unbuildable {
            bail!(
                "{} scene(s) cannot be built yet ({}). Pass --skip-unbuildable to compile the \
                 rest — the resolved show will then be missing that geometry, which is worth \
                 knowing before you watch it rather than after.",
                skipped.len(),
                skipped.iter().map(|(id, _)| id.as_str()).collect::<Vec<_>>().join(", ")
            );
        }
    }

    Ok(instances)
}

/// Prints what a built show directory actually contains — the cuts, the
/// length, the shot list.
///
/// `spex show` and `spex show-export` both call it, and both do so *before*
/// handing the directory to a browser: a screening that turns out to be the
/// wrong cut is worth finding out about in the terminal, in the second before
/// the window opens, rather than four minutes into watching it.
pub fn describe(show_dir: &Path) -> Result<()> {
    let text = std::fs::read_to_string(show_dir.join("show-resolved.json"))
        .with_context(|| format!("reading show-resolved.json in {}", show_dir.display()))?;
    let doc: serde_json::Value = serde_json::from_str(&text)?;
    println!(
        "{} — {:.3} s, {} shot(s), seed {}",
        doc.get("title").and_then(|v| v.as_str()).unwrap_or("(untitled)"),
        doc.get("durationSec").and_then(|v| v.as_f64()).unwrap_or(0.0),
        doc.get("shots").and_then(|v| v.as_array()).map_or(0, Vec::len),
        doc.get("seed").and_then(|v| v.as_u64()).unwrap_or(0),
    );
    if let Ok(index) = std::fs::read_to_string(show_dir.join("cuts.json")) {
        let index: serde_json::Value = serde_json::from_str(&index)?;
        let labels: Vec<String> = index
            .get("cuts")
            .and_then(|v| v.as_array())
            .map(|cuts| {
                cuts.iter()
                    .filter_map(|c| c.get("label").and_then(|v| v.as_str()).map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
        println!("cuts: {} (?duration=)", labels.join(", "));
    }
    Ok(())
}

/// A bundle's own instance ids, in bundle order — index *is* instance index.
fn read_instance_ids(bundle_dir: &Path) -> Result<Vec<String>> {
    let path = bundle_dir.join("mesh.json");
    let text = std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let manifest: serde_json::Value = serde_json::from_str(&text)?;
    let ids = manifest
        .get("instanceIds")
        .and_then(|v| v.as_array())
        .with_context(|| format!("{} has no instanceIds array", path.display()))?;
    Ok(ids.iter().filter_map(|v| v.as_str().map(str::to_string)).collect())
}

fn report(r: &ResolvedShow, path: &Path) {
    println!(
        "resolved {} shot(s) to {:.3} s (asked {:.3}), tier <= {}, seed {}{}",
        r.shots.len(),
        r.duration_sec,
        r.target_sec,
        u8::from(r.shots.iter().map(|s| s.tier).max().unwrap_or(spex_show::Tier::Always)),
        r.seed,
        if r.endless { ", endless" } else { "" }
    );
    println!(
        "{} at {} bpm — {}",
        if r.beat_aligned { "every boundary on a beat" } else { "boundaries NOT beat-aligned (the target is not a whole number of beats)" },
        r.tempo.bpm,
        format_shots(r)
    );
    println!("wrote {}", path.display());
}

fn format_shots(r: &ResolvedShow) -> String {
    r.shots
        .iter()
        .map(|s| format!("{} {:.3}s", s.id, s.duration_sec))
        .collect::<Vec<_>>()
        .join(" | ")
}
