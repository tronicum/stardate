//! M61 — the duration resolver: one document, four cuts.
//!
//! The 4:00, 10:00, 60:00 and endless versions are **not four edits**. They
//! are four resolutions of the same `show.json`, and this module is what makes
//! that sentence true rather than aspirational.
//!
//! # Why this cannot be "play it slower"
//!
//! The tempo is the same in every cut, because the music is the same music. A
//! longer cut therefore has to contain *more material*, not the same material
//! stretched over a metronome that has been slowed down. So two mechanisms,
//! and they do different jobs:
//!
//! - `tier` decides **which shots a cut contains at all**. Tier 1 is in
//!   everything; the 10:00 adds tier 2, the 60:00 adds tier 3.
//! - `scaling` decides **how the survivors respond to the time available**:
//!   `fixed` never moves (the Kick is two beats in every cut, because a flash
//!   that lasts ten seconds is not a flash), `stretch` scales within its own
//!   min/max, `repeat` loops an authored unit an integer number of times.
//!
//! # Water-filling, and why it is a loop
//!
//! Sharing the stretchable time in proportion to `weight` is one line. The
//! clamps are what make it iterative: a shot that would receive more than its
//! `maxBars` is pinned there and *leaves the pool*, and the time it gave back
//! has to be redistributed among the rest — which can push another shot over
//! *its* maximum. The loop terminates because every pass removes at least one
//! shot from the pool.
//!
//! If the clamps make the target unreachable the resolver **errors with the
//! exact shortfall**. Silently delivering a 9:47 "ten-minute" cut would be the
//! worse failure by a wide margin: nobody would ever look.
//!
//! # Beats, not floats
//!
//! Water-filling produces arbitrary reals, and the screenplay's own rule is
//! that cuts land on the grid. So when the target is a whole number of beats
//! *and* every fixed shot is too — which is the case for 240 s (336 beats),
//! 600 s (840) and 3600 s (5040) — every resolved duration is rounded to a
//! whole beat by **largest remainder**, which sums to the target exactly by
//! construction rather than approximately. When the target is not beat-aligned
//! (any duration a caller invents), the continuous solution is kept and
//! `beatAligned` says so. Neither case is allowed to miss the target.

use crate::model::*;
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The resolved document's own version, independent of `show.json`'s.
pub const RESOLVED_FORMAT_VERSION: u32 = 1;

/// How exact "exact" is. One millisecond is well under a frame at any rate the
/// piece will ever run at, and far above f64 noise over an hour of material.
pub const DURATION_TOLERANCE_SEC: f64 = 1e-3;

#[derive(Debug, Clone)]
pub struct ResolveOptions {
    pub target_sec: f64,
    pub seed: u64,
    /// Endless resolves exactly like the canonical cut and marks the output;
    /// the viewer loops it with a per-cycle seed advance (M82). A loop is the
    /// same cut played again, not a different edit.
    pub endless: bool,
}

impl ResolveOptions {
    pub fn canonical(show: &Show) -> Self {
        Self { target_sec: show.base_duration_sec(), seed: show.seed, endless: false }
    }
}

// ---------------------------------------------------------------------------
// The resolved document. Mirrors `spec/show-resolved.schema.json`.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolvedTempo {
    pub bpm: f64,
    #[serde(rename = "beatsPerBar")]
    pub beats_per_bar: u32,
    /// Derived and written out, so no consumer re-derives it slightly
    /// differently. 20/7 at the default tempo.
    #[serde(rename = "barSeconds")]
    pub bar_seconds: f64,
    #[serde(rename = "beatSeconds")]
    pub beat_seconds: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolvedScene {
    pub id: String,
    pub prefix: String,
    /// Path relative to the show directory, e.g. `bundles/monolith`.
    pub bundle: String,
    #[serde(default, rename = "instanceCount", skip_serializing_if = "Option::is_none")]
    pub instance_count: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolvedShow {
    pub version: u32,
    pub generator: String,
    pub id: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,
    #[serde(rename = "archiveSignature")]
    pub archive_signature: String,
    pub tempo: ResolvedTempo,
    #[serde(rename = "targetSec")]
    pub target_sec: f64,
    #[serde(rename = "durationSec")]
    pub duration_sec: f64,
    /// Whether every shot boundary landed on a beat. False only when the
    /// caller asked for a duration that is not a whole number of beats.
    #[serde(rename = "beatAligned")]
    pub beat_aligned: bool,
    pub endless: bool,
    pub seed: u64,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub palette: BTreeMap<String, [f32; 3]>,
    pub scenes: Vec<ResolvedScene>,
    pub shots: Vec<ResolvedShot>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credits: Option<Credits>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolvedShot {
    pub id: String,
    pub title: String,
    #[serde(rename = "movementId")]
    pub movement_id: String,
    #[serde(rename = "movementTitle")]
    pub movement_title: String,
    #[serde(rename = "romanNumeral")]
    pub roman_numeral: String,
    pub tier: Tier,
    #[serde(rename = "startSec")]
    pub start_sec: f64,
    #[serde(rename = "endSec")]
    pub end_sec: f64,
    #[serde(rename = "durationSec")]
    pub duration_sec: f64,
    #[serde(rename = "startBar")]
    pub start_bar: f64,
    #[serde(rename = "durationBars")]
    pub duration_bars: f64,
    #[serde(default, rename = "repeatCount", skip_serializing_if = "Option::is_none")]
    pub repeat_count: Option<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scenes: Vec<String>,
    pub camera: ResolvedCameraTrack,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tracks: Vec<ResolvedTrack>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cues: Vec<ResolvedCue>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolvedKey<T> {
    #[serde(rename = "timeSec")]
    pub time_sec: f64,
    pub value: T,
    pub easing: Easing,
}

/// A glob, and what it turned out to mean.
///
/// The glob is kept verbatim although nothing selects with it any more: it is
/// what lets a HUD, a diff or a person tell what the index list was *supposed*
/// to be. `instances` is absent when the show was resolved without building
/// bundles, which is the one case a player is allowed to expand it itself.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TargetBinding {
    pub glob: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scene: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instances: Option<Vec<u32>>,
}

impl TargetBinding {
    pub fn unbound(glob: &str) -> Self {
        Self { glob: glob.to_string(), scene: None, instances: None }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ResolvedTrack {
    Transform { target: TargetBinding, keys: Vec<ResolvedKey<TransformValue>> },
    Dissolve {
        target: TargetBinding,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        stagger: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        order: Option<DissolveOrder>,
        keys: Vec<ResolvedKey<f64>>,
    },
    Material { target: TargetBinding, property: MaterialProperty, keys: Vec<ResolvedKey<f64>> },
    /// Linear RGB. See `Track::Color`.
    Color { target: TargetBinding, keys: Vec<ResolvedKey<[f64; 3]>> },
    Post { property: PostProperty, keys: Vec<ResolvedKey<f64>> },
    Hud { element: String, keys: Vec<ResolvedKey<f64>> },
    PointCloud { target: TargetBinding, keys: Vec<ResolvedKey<f64>> },
}

impl ResolvedTrack {
    /// The binding this track animates, if it animates geometry at all. A post
    /// or HUD track addresses the frame, not the bricks.
    pub fn target_mut(&mut self) -> Option<&mut TargetBinding> {
        match self {
            ResolvedTrack::Transform { target, .. }
            | ResolvedTrack::Dissolve { target, .. }
            | ResolvedTrack::Material { target, .. }
            | ResolvedTrack::Color { target, .. }
            | ResolvedTrack::PointCloud { target, .. } => Some(target),
            ResolvedTrack::Post { .. } | ResolvedTrack::Hud { .. } => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolvedCameraTrack {
    pub mode: CameraMode,
    #[serde(default, rename = "fovDeg", skip_serializing_if = "Option::is_none")]
    pub fov_deg: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keys: Option<Vec<ResolvedKey<CameraValue>>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub orbit: Option<OrbitSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dolly: Option<DollySpec>,
    #[serde(default, rename = "exponentialZoom", skip_serializing_if = "Option::is_none")]
    pub exponential_zoom: Option<ZoomSpec>,
    #[serde(default, rename = "motionBlur", skip_serializing_if = "Option::is_none")]
    pub motion_blur: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolvedCue {
    #[serde(rename = "timeSec")]
    pub time_sec: f64,
    pub kind: CueKind,
    #[serde(rename = "shotId")]
    pub shot_id: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub payload: BTreeMap<String, serde_json::Value>,
}

// ---------------------------------------------------------------------------
// The resolver
// ---------------------------------------------------------------------------

/// One shot as the duration solver sees it: an index back into the document,
/// and the three numbers that decide what happens to it.
struct Candidate<'a> {
    movement: &'a Movement,
    shot: &'a Shot,
    /// Seconds. For a repeat shot this is `count * unitSec` once solved.
    duration: f64,
    min: f64,
    max: f64,
    /// Set for repeat shots once the continuous solution has been rounded.
    repeat_count: Option<u32>,
    fixed: bool,
}

pub fn resolve(show: &Show, opts: &ResolveOptions) -> Result<ResolvedShow> {
    let errors = validate(show);
    if !errors.is_empty() {
        bail!("cannot resolve an invalid show:\n  - {}", errors.join("\n  - "));
    }
    if !(opts.target_sec.is_finite() && opts.target_sec > 0.0) {
        bail!("target duration must be a positive number of seconds, got {}", opts.target_sec);
    }

    let bar = show.tempo.bar_seconds();
    let beat = show.tempo.beat_seconds();
    let target = opts.target_sec;

    // 1. Tier filter. A cut is a selection.
    let max_tier = tier_for(target);
    let mut cands: Vec<Candidate> = show
        .shots()
        .filter(|(_, s)| s.tier <= max_tier)
        .map(|(m, s)| candidate(m, s, bar))
        .collect();
    if cands.is_empty() {
        bail!(
            "no shot survives the tier filter at {target:.3} s (cuts below 600 s keep tier 1 only) \
             — a cut with no shots is not a cut"
        );
    }

    // 2. The fixed budget is not negotiable.
    let fixed: f64 = cands.iter().filter(|c| c.fixed).map(|c| c.duration).sum();
    if fixed > target + DURATION_TOLERANCE_SEC {
        bail!(
            "the fixed shots alone are {fixed:.3} s, longer than the {target:.3} s asked for \
             (over by {:.3} s). Dropping a fixed shot to make room would be worse than failing: \
             they are fixed because their length is the point.",
            fixed - target
        );
    }

    // 3+4. Water-fill the stretchable pool, then round the repeats to whole
    // passes and re-fill with what the rounding gave back or took.
    water_fill(&mut cands, target - fixed)?;
    if round_repeats(&mut cands, bar) {
        let fixed_now: f64 = cands.iter().filter(|c| c.fixed).map(|c| c.duration).sum();
        water_fill(&mut cands, target - fixed_now)?;
    }

    // 5. Did it actually land on the target?
    let total: f64 = cands.iter().map(|c| c.duration).sum();
    if (total - target).abs() > DURATION_TOLERANCE_SEC {
        // The advice has to be reachable advice. At 3600 s every tier is
        // already in, so "add tier-4 material" would be nonsense — there is no
        // tier 4, and a resolver that invents one is worse than one that says
        // plainly that the document is too short.
        let advice = match max_tier {
            Tier::Full => "Widen a shot's minBars/maxBars, or author more material: every tier is \
                           already in this cut."
                .to_string(),
            _ => format!(
                "Widen a shot's minBars/maxBars, or add tier-{} material.",
                u8::from(max_tier) + 1
            ),
        };
        bail!(
            "the clamps make {target:.3} s unreachable: the timeline resolves to {total:.3} s, \
             {:+.3} s off. {advice}",
            total - target
        );
    }

    // 6. Put every boundary on a beat, where the arithmetic allows it.
    let beat_aligned = quantise_to_beats(&mut cands, target, beat);

    // 7. Absolutise.
    let mut shots = Vec::with_capacity(cands.len());
    let mut t = 0.0;
    for c in &cands {
        shots.push(absolutise(c, t, bar, beat));
        t += c.duration;
    }

    Ok(ResolvedShow {
        version: RESOLVED_FORMAT_VERSION,
        generator: format!("spex-show {}", env!("CARGO_PKG_VERSION")),
        id: show.id.clone(),
        title: show.title.clone(),
        subtitle: show.subtitle.clone(),
        archive_signature: show.archive_signature.clone(),
        tempo: ResolvedTempo {
            bpm: show.tempo.bpm,
            beats_per_bar: show.tempo.beats_per_bar,
            bar_seconds: bar,
            beat_seconds: beat,
        },
        target_sec: target,
        duration_sec: t,
        beat_aligned,
        endless: opts.endless,
        seed: opts.seed,
        palette: show.palette.clone(),
        scenes: show
            .scenes
            .iter()
            .map(|s| ResolvedScene {
                id: s.id.clone(),
                prefix: s.prefix.clone(),
                bundle: format!("bundles/{}", s.id),
                instance_count: None,
            })
            .collect(),
        shots,
        credits: show.credits.clone(),
    })
}

/// Tier 1 everywhere; 2 from ten minutes; 3 from an hour.
pub fn tier_for(target_sec: f64) -> Tier {
    if target_sec >= 3600.0 {
        Tier::Full
    } else if target_sec >= 600.0 {
        Tier::Long
    } else {
        Tier::Always
    }
}

fn candidate<'a>(movement: &'a Movement, shot: &'a Shot, bar: f64) -> Candidate<'a> {
    let d = shot.duration_bars * bar;
    let (min, max) = match (&shot.scaling, &shot.repeat) {
        (Scaling::Fixed, _) => (d, d),
        (Scaling::Repeat, Some(r)) => {
            (r.min_count as f64 * r.unit_bars * bar, r.max_count as f64 * r.unit_bars * bar)
        }
        _ => (
            shot.min_bars.map(|b| b * bar).unwrap_or(0.0),
            shot.max_bars.map(|b| b * bar).unwrap_or(f64::INFINITY),
        ),
    };
    Candidate {
        movement,
        shot,
        duration: d,
        min,
        max,
        repeat_count: None,
        fixed: shot.scaling == Scaling::Fixed,
    }
}

/// Distribute `room` across the non-fixed shots in proportion to `weight`,
/// clamping to each shot's own range, until nothing clamps.
///
/// Terminates because a clamped shot leaves the pool and never returns, so
/// each pass strictly shrinks the pool or is the last.
fn water_fill(cands: &mut [Candidate], room: f64) -> Result<()> {
    let pool: Vec<usize> = (0..cands.len()).filter(|&i| !cands[i].fixed).collect();
    if pool.is_empty() {
        if room.abs() > DURATION_TOLERANCE_SEC {
            bail!(
                "every shot in this cut is `fixed`, so there is nothing to stretch, and the \
                 target is {room:+.3} s away from what they add up to"
            );
        }
        return Ok(());
    }

    let mut active: Vec<usize> = pool.clone();
    let mut remaining = room;

    loop {
        let total_weight: f64 = active.iter().map(|&i| cands[i].shot.weight.max(0.0)).sum();
        let n = active.len() as f64;
        let mut clamped = false;
        let mut next = Vec::with_capacity(active.len());

        for &i in &active {
            // A pool where every weight is zero shares equally rather than
            // dividing by zero: weightless shots are an authoring omission,
            // not an instruction to give them nothing.
            let share = if total_weight > 0.0 {
                remaining * cands[i].shot.weight.max(0.0) / total_weight
            } else {
                remaining / n
            };
            if share > cands[i].max {
                cands[i].duration = cands[i].max;
                remaining -= cands[i].max;
                clamped = true;
            } else if share < cands[i].min {
                cands[i].duration = cands[i].min;
                remaining -= cands[i].min;
                clamped = true;
            } else {
                cands[i].duration = share;
                next.push(i);
            }
        }

        if !clamped {
            return Ok(());
        }
        if next.is_empty() {
            // Everything is pinned. Whatever is left over is the shortfall the
            // caller gets told about by the assertion in `resolve`.
            return Ok(());
        }
        active = next;
    }
}

/// Turns each repeat shot's continuous duration into an integer number of
/// passes. Returns whether anything changed, which is the signal to re-fill.
fn round_repeats(cands: &mut [Candidate], bar: f64) -> bool {
    let mut changed = false;
    for c in cands.iter_mut() {
        let Some(rep) = c.shot.repeat.as_ref() else { continue };
        if c.shot.scaling != Scaling::Repeat {
            continue;
        }
        let unit_sec = rep.unit_bars * bar;
        let count = (c.duration / unit_sec).round().clamp(rep.min_count as f64, rep.max_count as f64);
        let d = count * unit_sec;
        if (d - c.duration).abs() > f64::EPSILON {
            changed = true;
        }
        c.duration = d;
        c.repeat_count = Some(count as u32);
        // A rounded repeat is no longer negotiable; the residual it just gave
        // back belongs to the stretch pool, which is why `resolve` re-fills.
        c.fixed = true;
    }
    changed
}

/// Round every duration to a whole beat without moving the total.
///
/// Largest remainder: floor everything, then hand the leftover beats out one
/// at a time to whoever was rounded down hardest. Sums to the target *by
/// construction*, which repeated rounding-and-hoping does not.
///
/// Returns false — and changes nothing — when the arithmetic cannot work:
/// a target that is not a whole number of beats, or a fixed shot that is not.
/// Those are not errors. They just mean this particular cut's boundaries are
/// where the water-filling put them.
fn quantise_to_beats(cands: &mut [Candidate], target: f64, beat: f64) -> bool {
    let is_int = |x: f64| (x - x.round()).abs() < 1e-6;
    let target_beats = target / beat;
    if !is_int(target_beats) {
        return false;
    }
    if cands.iter().filter(|c| c.fixed).any(|c| !is_int(c.duration / beat)) {
        return false;
    }

    let movable: Vec<usize> = (0..cands.len()).filter(|&i| !cands[i].fixed).collect();
    if movable.is_empty() {
        return true; // already integral, by the check above
    }

    let fixed_beats: f64 = cands.iter().filter(|c| c.fixed).map(|c| c.duration / beat).sum();
    let mut budget = (target_beats - fixed_beats).round() as i64;
    if budget < movable.len() as i64 {
        // Fewer whole beats than shots to put them in: one of them would have
        // to be zero-length. Leave the continuous solution alone.
        return false;
    }

    // Floor first, one beat minimum, and remember what each shot lost.
    let mut remainders: Vec<(usize, f64)> = Vec::with_capacity(movable.len());
    for &i in &movable {
        let exact = cands[i].duration / beat;
        let floor = exact.floor().max(1.0);
        cands[i].duration = floor * beat;
        budget -= floor as i64;
        remainders.push((i, exact - floor));
    }
    if budget < 0 {
        return false;
    }

    // Ties break on shot order, so the same document always resolves the same
    // way — determinism here is not decoration, it is AC4.
    remainders.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal).then(a.0.cmp(&b.0)));
    let mut k = 0usize;
    while budget > 0 {
        let (i, _) = remainders[k % remainders.len()];
        cands[i].duration += beat;
        budget -= 1;
        k += 1;
    }
    true
}

fn absolutise(c: &Candidate, start: f64, bar: f64, beat: f64) -> ResolvedShot {
    let shot = c.shot;
    let end = start + c.duration;
    let at = |t: f64, snap: bool| -> f64 {
        let abs = start + t.clamp(0.0, 1.0) * c.duration;
        if !snap {
            return abs;
        }
        // Snap to the nearest beat, but never outside the shot: a landing
        // brick that lands after its own shot has ended is not on the beat,
        // it is gone.
        ((abs / beat).round() * beat).clamp(start, end)
    };

    let map_keys = |keys: &[Keyframe<f64>]| -> Vec<ResolvedKey<f64>> {
        keys.iter()
            .map(|k| ResolvedKey { time_sec: at(k.t, k.snap_to_beat), value: k.value, easing: k.easing })
            .collect()
    };
    // The same three lines for a triple. A closure cannot be generic, and one
    // generic helper would have to take `at` as a parameter — which is where
    // this got interesting enough to be worth a comment rather than a rewrite.
    let map_rgb_keys = |keys: &[Keyframe<[f64; 3]>]| -> Vec<ResolvedKey<[f64; 3]>> {
        keys.iter()
            .map(|k| ResolvedKey { time_sec: at(k.t, k.snap_to_beat), value: k.value, easing: k.easing })
            .collect()
    };

    let tracks = shot
        .tracks
        .iter()
        .map(|t| match t {
            Track::Transform { target, keys } => ResolvedTrack::Transform {
                target: TargetBinding::unbound(target),
                keys: keys
                    .iter()
                    .map(|k| ResolvedKey {
                        time_sec: at(k.t, k.snap_to_beat),
                        value: k.value.clone(),
                        easing: k.easing,
                    })
                    .collect(),
            },
            Track::Dissolve { target, stagger, order, keys } => ResolvedTrack::Dissolve {
                target: TargetBinding::unbound(target),
                stagger: *stagger,
                order: *order,
                keys: map_keys(keys),
            },
            Track::Material { target, property, keys } => ResolvedTrack::Material {
                target: TargetBinding::unbound(target),
                property: *property,
                keys: map_keys(keys),
            },
            Track::Color { target, keys } => {
                ResolvedTrack::Color { target: TargetBinding::unbound(target), keys: map_rgb_keys(keys) }
            }
            Track::Post { property, keys } => {
                ResolvedTrack::Post { property: *property, keys: map_keys(keys) }
            }
            Track::Hud { element, keys } => {
                ResolvedTrack::Hud { element: element.clone(), keys: map_keys(keys) }
            }
            Track::PointCloud { target, keys } => ResolvedTrack::PointCloud {
                target: TargetBinding::unbound(target),
                keys: map_keys(keys),
            },
        })
        .collect();

    let camera = ResolvedCameraTrack {
        mode: shot.camera.mode,
        fov_deg: shot.camera.fov_deg,
        keys: shot.camera.keys.as_ref().map(|keys| {
            keys.iter()
                .map(|k| ResolvedKey {
                    time_sec: at(k.t, k.snap_to_beat),
                    value: k.value.clone(),
                    easing: k.easing,
                })
                .collect()
        }),
        orbit: shot.camera.orbit.clone(),
        dolly: shot.camera.dolly.clone(),
        exponential_zoom: shot.camera.exponential_zoom.clone(),
        motion_blur: shot.camera.motion_blur,
    };

    ResolvedShot {
        id: shot.id.clone(),
        title: shot.title.clone(),
        movement_id: c.movement.id.clone(),
        movement_title: c.movement.title.clone(),
        roman_numeral: c.movement.roman_numeral.clone(),
        tier: shot.tier,
        start_sec: start,
        end_sec: end,
        duration_sec: c.duration,
        start_bar: start / bar,
        duration_bars: c.duration / bar,
        repeat_count: c.repeat_count,
        scenes: shot.scenes.clone(),
        camera,
        tracks,
        cues: shot
            .cues
            .iter()
            .map(|cue| ResolvedCue {
                time_sec: at(cue.t, false),
                kind: cue.kind,
                shot_id: shot.id.clone(),
                payload: cue.payload.clone(),
            })
            .collect(),
        note: shot.note.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shot(id: &str, weight: f64, scaling: Scaling, bars: f64, min: Option<f64>, max: Option<f64>, tier: Tier) -> Shot {
        Shot {
            id: id.into(),
            title: id.into(),
            weight,
            scaling,
            duration_bars: bars,
            min_bars: min,
            max_bars: max,
            tier,
            repeat: None,
            scenes: vec![],
            camera: CameraTrack {
                mode: CameraMode::Keyed,
                fov_deg: None,
                keys: Some(vec![Keyframe {
                    t: 0.0,
                    value: CameraValue::default(),
                    easing: Easing::Linear,
                    snap_to_beat: false,
                }]),
                orbit: None,
                dolly: None,
                exponential_zoom: None,
                motion_blur: None,
            },
            tracks: vec![],
            cues: vec![],
            note: None,
        }
    }

    fn show_of(shots: Vec<Shot>) -> Show {
        Show {
            audio: None,
            version: FORMAT_VERSION,
            id: "t".into(),
            title: "T".into(),
            subtitle: None,
            archive_signature: "IA-0".into(),
            tempo: Tempo::default(),
            base_duration_bars: shots.iter().map(|s| s.duration_bars).sum(),
            seed: 0,
            palette: Default::default(),
            scenes: vec![],
            movements: vec![Movement {
                id: "m".into(),
                title: "M".into(),
                roman_numeral: "I".into(),
                shots,
            }],
            credits: None,
        }
    }

    fn resolved_total(r: &ResolvedShow) -> f64 {
        r.shots.iter().map(|s| s.duration_sec).sum()
    }

    #[test]
    fn the_three_canonical_cuts_are_exact_to_the_millisecond() {
        // Generous ranges, so no clamp can make a target unreachable.
        let show = show_of(vec![
            shot("F", 0.0, Scaling::Fixed, 2.0, None, None, Tier::Always),
            shot("A", 1.0, Scaling::Stretch, 6.0, Some(2.0), Some(4000.0), Tier::Always),
            shot("B", 2.0, Scaling::Stretch, 9.0, Some(2.0), Some(4000.0), Tier::Always),
        ]);
        for target in [240.0, 600.0, 3600.0] {
            let r = resolve(&show, &ResolveOptions { target_sec: target, seed: 0, endless: false })
                .unwrap_or_else(|e| panic!("{target}: {e}"));
            assert!((r.duration_sec - target).abs() < DURATION_TOLERANCE_SEC, "{target} -> {}", r.duration_sec);
            assert!((resolved_total(&r) - target).abs() < DURATION_TOLERANCE_SEC);
            assert!(r.beat_aligned, "{target} should land on the beat grid");
        }
    }

    /// The fixed shot is the one thing a longer cut must not touch.
    #[test]
    fn a_fixed_shot_is_the_same_length_in_every_cut() {
        let show = show_of(vec![
            shot("KICK", 0.0, Scaling::Fixed, 0.5, None, None, Tier::Always),
            shot("A", 1.0, Scaling::Stretch, 8.0, Some(1.0), Some(4000.0), Tier::Always),
        ]);
        let mut seen = vec![];
        for target in [240.0, 600.0, 3600.0] {
            let r = resolve(&show, &ResolveOptions { target_sec: target, seed: 0, endless: false }).unwrap();
            seen.push(r.shots.iter().find(|s| s.id == "KICK").unwrap().duration_sec);
        }
        assert!(seen.windows(2).all(|w| (w[0] - w[1]).abs() < 1e-12), "{seen:?}");
        // Half a bar at 84 bpm 4/4 is two beats: 10/7 s.
        assert!((seen[0] - 10.0 / 7.0).abs() < 1e-9, "{}", seen[0]);
    }

    #[test]
    fn a_cut_shorter_than_its_own_fixed_material_is_an_error_with_the_number_in_it() {
        let show = show_of(vec![
            shot("F", 0.0, Scaling::Fixed, 84.0, None, None, Tier::Always),
            shot("A", 1.0, Scaling::Stretch, 4.0, None, None, Tier::Always),
        ]);
        let err = resolve(&show, &ResolveOptions { target_sec: 60.0, seed: 0, endless: false })
            .unwrap_err()
            .to_string();
        assert!(err.contains("240.000"), "{err}");
        assert!(err.contains("180.000"), "{err}");
    }

    #[test]
    fn clamps_that_make_the_target_unreachable_error_rather_than_miss_it() {
        let show = show_of(vec![shot("A", 1.0, Scaling::Stretch, 4.0, Some(1.0), Some(8.0), Tier::Always)]);
        let err = resolve(&show, &ResolveOptions { target_sec: 600.0, seed: 0, endless: false })
            .unwrap_err()
            .to_string();
        assert!(err.contains("unreachable"), "{err}");
        // 8 bars is 22.857 s; the message has to say how far off it is.
        assert!(err.contains("22.857"), "{err}");
    }

    #[test]
    fn tier_two_appears_at_ten_minutes_and_tier_three_at_an_hour() {
        let show = show_of(vec![
            shot("T1", 1.0, Scaling::Stretch, 4.0, Some(1.0), Some(4000.0), Tier::Always),
            shot("T2", 1.0, Scaling::Stretch, 4.0, Some(1.0), Some(4000.0), Tier::Long),
            shot("T3", 1.0, Scaling::Stretch, 4.0, Some(1.0), Some(4000.0), Tier::Full),
        ]);
        let ids = |target: f64| -> Vec<String> {
            resolve(&show, &ResolveOptions { target_sec: target, seed: 0, endless: false })
                .unwrap()
                .shots
                .iter()
                .map(|s| s.id.clone())
                .collect()
        };
        assert_eq!(ids(240.0), vec!["T1"]);
        assert_eq!(ids(599.0), vec!["T1"], "just under ten minutes is still the short cut");
        assert_eq!(ids(600.0), vec!["T1", "T2"]);
        assert_eq!(ids(3600.0), vec!["T1", "T2", "T3"]);
    }

    #[test]
    fn shots_are_contiguous_and_start_where_the_previous_one_ended() {
        let show = show_of(vec![
            shot("A", 1.0, Scaling::Stretch, 4.0, Some(1.0), Some(400.0), Tier::Always),
            shot("B", 3.0, Scaling::Stretch, 4.0, Some(1.0), Some(400.0), Tier::Always),
            shot("C", 2.0, Scaling::Stretch, 4.0, Some(1.0), Some(400.0), Tier::Always),
        ]);
        let r = resolve(&show, &ResolveOptions { target_sec: 240.0, seed: 0, endless: false }).unwrap();
        assert!((r.shots[0].start_sec).abs() < 1e-12);
        for w in r.shots.windows(2) {
            assert!((w[0].end_sec - w[1].start_sec).abs() < 1e-12);
        }
        assert!((r.shots.last().unwrap().end_sec - 240.0).abs() < DURATION_TOLERANCE_SEC);
    }

    /// Weight is a *share*, and the point of the test is that the shares are
    /// the authored ratio once the clamps are out of the way.
    #[test]
    fn weight_decides_the_share_when_nothing_clamps() {
        let show = show_of(vec![
            shot("A", 1.0, Scaling::Stretch, 4.0, Some(0.5), Some(4000.0), Tier::Always),
            shot("B", 3.0, Scaling::Stretch, 4.0, Some(0.5), Some(4000.0), Tier::Always),
        ]);
        let r = resolve(&show, &ResolveOptions { target_sec: 240.0, seed: 0, endless: false }).unwrap();
        let a = r.shots[0].duration_sec;
        let b = r.shots[1].duration_sec;
        // Beat quantisation moves each by at most one beat.
        assert!((b / a - 3.0).abs() < 0.05, "{a} : {b}");
    }

    #[test]
    fn a_repeat_shot_resolves_to_a_whole_number_of_passes() {
        let mut s = shot("ATL", 1.0, Scaling::Repeat, 6.0, None, None, Tier::Always);
        s.repeat = Some(RepeatSpec { unit_bars: 2.0, min_count: 1, max_count: 40 });
        let show = show_of(vec![
            s,
            shot("A", 1.0, Scaling::Stretch, 8.0, Some(1.0), Some(4000.0), Tier::Always),
        ]);
        for target in [240.0, 600.0, 3600.0] {
            let r = resolve(&show, &ResolveOptions { target_sec: target, seed: 0, endless: false }).unwrap();
            let atl = r.shots.iter().find(|x| x.id == "ATL").unwrap();
            let count = atl.repeat_count.expect("a repeat shot must report its count");
            assert!(count >= 1 && count <= 40);
            // The duration really is count whole passes of 2 bars.
            assert!((atl.duration_bars - count as f64 * 2.0).abs() < 1e-6, "{atl:?}");
            assert!((r.duration_sec - target).abs() < DURATION_TOLERANCE_SEC);
        }
    }

    /// AC1. Deterministic pseudo-randomness (splitmix64) rather than a crate,
    /// so a failure is reproducible from the seed printed in the panic.
    #[test]
    fn two_hundred_random_configurations_either_hit_the_target_or_say_why_not() {
        let mut state = 0x9E3779B97F4A7C15u64;
        let mut next = move || {
            state = state.wrapping_add(0x9E3779B97F4A7C15);
            let mut z = state;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
            (z ^ (z >> 31)) as f64 / u64::MAX as f64
        };

        let mut resolved = 0;
        let mut refused = 0;
        for case in 0..200 {
            let n = 2 + (next() * 6.0) as usize;
            let shots: Vec<Shot> = (0..n)
                .map(|i| {
                    let min = 0.5 + next() * 4.0;
                    let max = min + next() * 200.0;
                    let scaling = if next() < 0.15 { Scaling::Fixed } else { Scaling::Stretch };
                    let bars = min + (max - min) * 0.25;
                    shot(
                        &format!("S{i}"),
                        next() * 4.0,
                        scaling,
                        bars,
                        if scaling == Scaling::Fixed { None } else { Some(min) },
                        if scaling == Scaling::Fixed { None } else { Some(max) },
                        Tier::Always,
                    )
                })
                .collect();
            let show = show_of(shots);
            let target = 60.0 + next() * 7140.0;

            match resolve(&show, &ResolveOptions { target_sec: target, seed: 0, endless: false }) {
                Ok(r) => {
                    resolved += 1;
                    let sum = resolved_total(&r);
                    assert!(
                        (sum - target).abs() < DURATION_TOLERANCE_SEC,
                        "case {case}: asked {target:.6}, got {sum:.6}"
                    );
                    assert!(
                        (r.duration_sec - sum).abs() < 1e-9,
                        "case {case}: durationSec disagrees with the shots it contains"
                    );
                    for s in &r.shots {
                        assert!(s.duration_sec > 0.0, "case {case}: {} has no length", s.id);
                    }
                }
                Err(e) => {
                    refused += 1;
                    let m = e.to_string();
                    assert!(
                        m.contains("unreachable") || m.contains("longer than") || m.contains("nothing to stretch"),
                        "case {case}: refused without saying why: {m}"
                    );
                }
            }
        }
        // Both branches must actually occur, or the test is only exercising one.
        assert!(resolved > 20, "only {resolved} of 200 resolved");
        assert!(refused > 0, "no configuration was refused — the clamps are not being tested");
    }

    #[test]
    fn endless_resolves_like_the_canonical_cut_and_says_so() {
        let show = show_of(vec![shot("A", 1.0, Scaling::Stretch, 84.0, Some(1.0), Some(4000.0), Tier::Always)]);
        let a = resolve(&show, &ResolveOptions { target_sec: 240.0, seed: 7, endless: false }).unwrap();
        let b = resolve(&show, &ResolveOptions { target_sec: 240.0, seed: 7, endless: true }).unwrap();
        assert!(!a.endless && b.endless);
        assert_eq!(a.shots.len(), b.shots.len());
        assert!((a.duration_sec - b.duration_sec).abs() < 1e-12);
    }

    /// AC4's precondition. Nothing in the resolver consults the seed yet — the
    /// seeded choices are M74's site selection and M82's cycle advance — so
    /// this asserts the weaker thing that is actually true today, and will
    /// keep asserting the right thing once those land.
    #[test]
    fn the_same_seed_resolves_byte_identically() {
        let show = show_of(vec![
            shot("A", 1.0, Scaling::Stretch, 4.0, Some(1.0), Some(400.0), Tier::Always),
            shot("B", 2.5, Scaling::Stretch, 4.0, Some(1.0), Some(400.0), Tier::Always),
        ]);
        let opts = ResolveOptions { target_sec: 240.0, seed: 263865, endless: false };
        let a = serde_json::to_string(&resolve(&show, &opts).unwrap()).unwrap();
        let b = serde_json::to_string(&resolve(&show, &opts).unwrap()).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn a_snapped_keyframe_lands_on_a_beat_and_never_outside_its_shot() {
        let mut s = shot("A", 1.0, Scaling::Stretch, 8.0, Some(1.0), Some(400.0), Tier::Always);
        s.tracks.push(Track::Dissolve { target: "x/**".into(), stagger: None, order: None, keys: vec![
                Keyframe { t: 0.37, value: 1.0, easing: Easing::Linear, snap_to_beat: true },
                Keyframe { t: 1.0, value: 0.0, easing: Easing::Linear, snap_to_beat: true },
            ],
        });
        let show = show_of(vec![s]);
        let r = resolve(&show, &ResolveOptions { target_sec: 240.0, seed: 0, endless: false }).unwrap();
        let beat = r.tempo.beat_seconds;
        let shot0 = &r.shots[0];
        let ResolvedTrack::Dissolve { keys, .. } = &shot0.tracks[0] else { panic!() };
        for k in keys {
            let beats = k.time_sec / beat;
            assert!((beats - beats.round()).abs() < 1e-9, "{} is not on a beat", k.time_sec);
            assert!(k.time_sec >= shot0.start_sec - 1e-9 && k.time_sec <= shot0.end_sec + 1e-9);
        }
    }
}
