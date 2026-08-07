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
    pub target_sec: Option<f64>,
    pub seed: Option<u64>,
    pub endless: bool,
    pub no_bundles: bool,
    pub skip_unbuildable: bool,
    pub crease: f64,
}

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

    let resolve_opts = ResolveOptions {
        target_sec: opts.target_sec.unwrap_or_else(|| show.base_duration_sec()),
        seed: opts.seed.unwrap_or(show.seed),
        endless: opts.endless,
    };
    let mut resolved = spex_show::resolve(&show, &resolve_opts)?;

    std::fs::create_dir_all(out).with_context(|| format!("creating {}", out.display()))?;

    let instances = if opts.no_bundles {
        println!("--no-bundles: resolving the timeline only, no geometry built");
        SceneInstances::new()
    } else {
        build_bundles(&show, out, cache_dir, opts, &mut resolved)?
    };

    if !instances.is_empty() {
        let unmatched = spex_show::bind_targets(&mut resolved, &instances);
        // Never silent. A glob that matches nothing animates nothing, and
        // produces no error at any later point.
        for u in &unmatched {
            println!("  ! target matched no instance — {u}");
        }
        if !unmatched.is_empty() {
            println!(
                "  {} target(s) matched nothing. `*` does not cross a `/`, and a bundle's ids are \
                 <part>/<n>, so \"scene/*\" is a common way to write \"scene/**\" by mistake.",
                unmatched.len()
            );
        }
    }

    let path = out.join("show-resolved.json");
    let json = serde_json::to_string_pretty(&resolved)?;
    std::fs::write(&path, format!("{json}\n")).with_context(|| format!("writing {}", path.display()))?;

    report(&resolved, &path);
    Ok(())
}

fn build_bundles(
    show: &Show,
    out: &Path,
    cache_dir: &Path,
    opts: &BuildOptions,
    resolved: &mut ResolvedShow,
) -> Result<SceneInstances> {
    let cache = LdrawCache::new(cache_dir);
    let mut instances = SceneInstances::new();
    let mut skipped: Vec<(String, String)> = Vec::new();

    for scene in &show.scenes {
        let path = match &scene.source {
            SceneSource::Ldr { path } => path.clone(),
            SceneSource::Build { recipe } => {
                skipped.push((scene.id.clone(), format!("build recipe {recipe:?} — M72")));
                continue;
            }
            SceneSource::Flag { flag } => {
                skipped.push((scene.id.clone(), format!("flag {flag:?} — M75")));
                continue;
            }
            SceneSource::Heritage { site_id } => {
                skipped.push((scene.id.clone(), format!("heritage site {site_id:?} — M73")));
                continue;
            }
        };

        let dir = out.join("bundles").join(&scene.id);
        let parsed = crate::mesh::parse_scene_arg(&cache, &path)
            .with_context(|| format!("scene {:?} from {path:?}", scene.id))?;
        let stats = crate::mesh::build_scene_bundle(&cache, &parsed, opts.crease, &dir)?;
        println!(
            "  {} <- {path}: {} instance(s), {} part(s), {} triangle(s)",
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
        let dropped: Vec<&str> = skipped.iter().map(|(id, _)| id.as_str()).collect();
        resolved.scenes.retain(|s| !dropped.contains(&s.id.as_str()));
        for shot in &mut resolved.shots {
            shot.scenes.retain(|s| !dropped.contains(&s.as_str()));
        }
    }

    Ok(instances)
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
