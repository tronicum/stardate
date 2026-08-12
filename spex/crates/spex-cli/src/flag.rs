//! `spex flag` — a construction sheet in, a buildable recipe out.
//!
//! The output is a **recipe**, not an `.ldr`, and that is the one place this
//! deviates from `phase4-kit.md`'s M75 signature. `spex_build::Mosaic` already
//! turns a grid of colour codes into one tile per cell on the real stud grid;
//! a second emitter would be a second set of decisions about the same thing.
//! So the flag joins the pipeline where every other scene in the piece joins
//! it, and the `.ldr` a person can open in an editor falls out of `spex build`
//! / `spex show-build` exactly as `feld`'s and `stonehenge`'s do.
use anyhow::{Context, Result};
use spex_flag::{FlagSpec, DELTA_E_REVIEW_THRESHOLD};
use std::collections::HashMap;
use std::path::Path;

/// Which real LDraw colours a flag is allowed to be built out of.
///
/// Three conditions, and the third was learnt the hard way.
///
/// Opaque, because a transparent tile over a plate is a different object with
/// a different appearance under every light. `Finish::Solid`, because chrome,
/// pearl, rubber, speckle and glitter all change with the viewing angle, and a
/// flag must not. And **in `LDConfig.ldr`'s own "LDraw Solid Colours"
/// section** — because the first Belgian flag built here came out in `30006
/// Modulex_Ochre_Yellow`. Modulex is a different product line at a different
/// scale; its entries are plain opaque `!COLOUR` lines with no material
/// keyword, so the first two conditions let them straight through, and the
/// quantiser picked one purely because it was nearest in Lab. The section
/// heading is the only thing in the real file that distinguishes them, which
/// is why `spex-ldraw` now carries it.
///
/// It is deliberately NOT "colours currently in production". `LDConfig.ldr`
/// does not record production status — it is a colour table, not a catalogue —
/// and inferring it from the file would be inventing data. Narrowing the
/// palette to what is actually purchasable today is a real question for the
/// physical edition and wants a real source (BrickLink's colour availability,
/// or LEGO's own current-colours list) rather than a guess here.
fn permitted_palette(colors: &HashMap<u32, spex_ldraw::LdrawColor>) -> Vec<u32> {
    let mut codes: Vec<u32> = colors
        .values()
        .filter(|c| {
            !c.is_transparent()
                && c.finish == spex_ldraw::Finish::Solid
                && c.luminance == 0
                && c.section == "Solid"
        })
        .map(|c| c.code)
        .collect();
    codes.sort_unstable();
    codes
}

pub fn build(iso2: &str, width_studs: u32, flags_dir: &Path, out: &Path, cache_dir: &Path) -> Result<()> {
    let path = flags_dir.join(format!("{}.json", iso2.to_lowercase()));
    let text = std::fs::read_to_string(&path)
        .with_context(|| format!("no construction sheet at {} — a flag this project cannot cite is a flag it does not build", path.display()))?;
    let spec: FlagSpec = serde_json::from_str(&text)
        .with_context(|| format!("{} is not a flag specification", path.display()))?;

    let errs = spec.validate();
    if !errs.is_empty() {
        for e in &errs {
            eprintln!("  ! {e}");
        }
        anyhow::bail!("{} has {} structural error(s)", path.display(), errs.len());
    }
    if spec.unsupported {
        anyhow::bail!(
            "{} ({}) is marked unsupported: its real construction needs something the element set cannot express, \
             and approximating it would be a fake flag. It is excluded from the Atlas by that mark.",
            spec.iso2,
            spec.name
        );
    }

    let cells = spex_flag::rasterize(&spec, width_studs)?;
    let rows = cells.len();

    let cache = spex_ldraw::LdrawCache::new(cache_dir);
    let full = spex_ldraw::load_colors_full(&cache)?;
    let palette = permitted_palette(&full);
    let rgb: HashMap<u32, [u8; 3]> = full.iter().map(|(k, v)| (*k, v.value)).collect();
    let (coded, report) = spex_flag::quantize(&cells, &rgb, &palette)?;

    println!("{} — {} ({}:{})", spec.iso2, spec.name, spec.ratio[0], spec.ratio[1]);
    println!("  {width_studs} x {rows} studs = {} tiles", width_studs as usize * rows);
    println!("  palette: {} opaque solid LDraw colours considered", palette.len());
    for (src, code, de) in &report.mapping {
        let name = full.get(code).map(|c| c.name.as_str()).unwrap_or("?");
        println!(
            "  rgb({:3},{:3},{:3}) -> {code:>3} {name:<24} dE {de:.2}",
            src[0], src[1], src[2]
        );
    }
    println!(
        "  worst dE {:.2}, mean {:.2}{}",
        report.max_delta_e,
        report.mean_delta_e,
        if report.max_delta_e > DELTA_E_REVIEW_THRESHOLD {
            "   <- OVER THE REVIEW THRESHOLD, do not ship this silently"
        } else {
            ""
        }
    );

    let recipe = serde_json::json!({
        "version": 1,
        "id": format!("flag-{}", spec.iso2.to_lowercase()),
        "title": format!("{} — {} ({} x {} Studs)", spec.iso2, spec.name, width_studs, rows),
        "_note": format!(
            "Generated by `spex flag {} --width-studs {}`. Do not hand-edit: the source of truth is {}, \
             whose construction is cited there. Worst CIEDE2000 against the real LDraw table: {:.2}. {}",
            spec.iso2, width_studs, path.display(), report.max_delta_e,
            spec.note.clone().unwrap_or_default()
        ),
        "scale": {
            "studsPerMetre": 20,
            "note": "The Act III scale, so a flag stands next to the field and the patent bricks at the size they really are relative to one another."
        },
        "steps": [{
            "primitive": "Mosaic",
            "at": {"xStuds": 0, "zStuds": 0, "yPlates": 0},
            "params": {"tilePart": "3070b.dat", "cells": coded}
        }]
    });
    std::fs::write(out, serde_json::to_string_pretty(&recipe)? + "\n")?;
    println!("  wrote {}", out.display());
    Ok(())
}
