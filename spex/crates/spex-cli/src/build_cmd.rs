//! `spex build` — M72's `spex-build` recipe pipeline: real, grid-legal
//! placements from a JSON recipe, out as a real `.ldr` with provenance.
//!
//! See `docs/fugen/phase4-kit.md`'s M72 section and `crates/spex-build/`
//! for the implementation this wires up. This file is deliberately thin:
//! `spex_build::build` does the real work, this only reads the CLI
//! arguments and prints what acceptance criterion 2 requires (`validate`'s
//! findings — declared and undeclared — printed either way).
use anyhow::{Context, Result};
use spex_build::grid::Illegality;
use std::path::{Path, PathBuf};

pub fn cmd_build(recipe: &Path, out: &PathBuf) -> Result<()> {
    let output = spex_build::build(recipe).with_context(|| format!("building recipe {}", recipe.display()))?;

    println!(
        "{} placement(s) from recipe {:?} (hash {})",
        output.placements.len(),
        output.recipe.id,
        output.recipe_hash
    );

    if output.declared.is_empty() && output.undeclared.is_empty() {
        println!("validate: zero Illegality");
    } else {
        if !output.declared.is_empty() {
            println!("{} declared exception(s) (recipe's own \"knownIllegal\"):", output.declared.len());
            for (problem, reason) in &output.declared {
                println!("  {} — {reason}", describe(problem));
            }
        }
        if !output.undeclared.is_empty() {
            println!("{} UNDECLARED real problem(s):", output.undeclared.len());
            for problem in &output.undeclared {
                println!("  {}", describe(problem));
            }
        }
    }

    if let Some(parent) = out.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    std::fs::write(out, &output.ldr_text).with_context(|| format!("writing {}", out.display()))?;
    println!("wrote {} ({} bytes)", out.display(), output.ldr_text.len());
    Ok(())
}

fn describe(problem: &Illegality) -> String {
    match *problem {
        Illegality::OffGridTranslation { placement_index, axis, ldu } => {
            format!("placement {placement_index}: off-grid translation on {axis} ({ldu} LDU)")
        }
        Illegality::NonAxisRotation { placement_index } => format!("placement {placement_index}: rotation is not one of the real 24 axis-aligned orientations"),
        Illegality::Overlap { a, b, overlap_ldu3 } => format!("placements {a} and {b}: overlap by {overlap_ldu3:.1} LDU^3"),
        Illegality::Floating { placement_index } => format!("placement {placement_index}: floating, no support below and not resting on the ground"),
    }
}
