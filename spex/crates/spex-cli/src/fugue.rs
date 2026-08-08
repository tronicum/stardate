//! `spex fugue-build` — realise a show document's fugue plan as a MIDI file.
//!
//! One artefact. The file this writes is what the browser will load (M69) and
//! what a person opens in a DAW, so review and runtime cannot disagree — which
//! is the whole reason rev 4 deleted the second score format.
//!
//! It prints the relaxations. A generated fugue that had to break a rule says
//! which rule, in which bar, in which voice, and anyone can go and listen to
//! that bar. Silence about it would be the same failure this project keeps
//! engineering against: output that looks perfect because nothing measured it.

use anyhow::{Context, Result};
use spex_fugue::counterpoint::Rule;
use std::path::Path;

pub fn build(show_path: &Path, out: &Path, seed: Option<u64>) -> Result<()> {
    let text = std::fs::read_to_string(show_path)
        .with_context(|| format!("reading {}", show_path.display()))?;
    let doc: serde_json::Value = serde_json::from_str(&text)?;
    let audio = doc.get("audio").with_context(|| {
        format!("{} has no `audio` block — nothing to realise", show_path.display())
    })?;
    let spec: spex_fugue::FugueSpec = serde_json::from_value(audio.clone())
        .with_context(|| "the `audio` block does not match fugue.schema.json")?;
    let seed = seed
        .or_else(|| doc.get("seed").and_then(|v| v.as_u64()))
        .unwrap_or(0);

    let r = spex_fugue::realise(&spec, seed);
    let bytes = spex_fugue::to_smf(&r);
    if let Some(parent) = out.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    std::fs::write(out, &bytes).with_context(|| format!("writing {}", out.display()))?;

    let bars = r.notes.iter().map(|n| n.end_beat()).fold(0.0, f64::max) / r.beats_per_bar;
    println!(
        "{} note(s) in 4 voices over {:.1} bars at {} bpm, seed {}",
        r.notes.len(),
        bars,
        r.bpm,
        seed
    );
    println!(
        "{} entr{} — {}",
        r.entries.len(),
        if r.entries.len() == 1 { "y" } else { "ies" },
        r.entries
            .iter()
            .take(6)
            .map(|e| format!("bar {:.0} {:?}", e.at_bar, e.transposition))
            .collect::<Vec<_>>()
            .join(", ")
    );

    if r.relaxations.is_empty() {
        println!("no rules relaxed");
    } else {
        println!("{} relaxation(s):", r.relaxations.len());
        for rule in [Rule::ParallelPerfect, Rule::Range, Rule::VoiceCrossing, Rule::WeakBeatDissonance] {
            let hits: Vec<String> = r
                .relaxations
                .iter()
                .filter(|x| x.rule == rule)
                .map(|x| format!("bar {:.0} {}", x.bar + 1.0, spex_fugue::counterpoint::VOICE_NAMES[x.voice]))
                .collect();
            if !hits.is_empty() {
                println!("  {} x{}: {}", rule.name(), hits.len(), hits.join(", "));
            }
        }
    }
    println!("wrote {} ({} bytes)", out.display(), bytes.len());
    Ok(())
}
