//! M60 — the authored show document.
//!
//! The screenplay has to be **data**, not code. Everything after this depends
//! on it: the four duration cuts, seeded editions, seeking, the director HUD,
//! re-authoring a shot without recompiling anything. A document a resolver
//! can transform is the difference between a film and a program that happens
//! to produce one.
//!
//! # Time is authored in bars
//!
//! The spec's first draft put every duration in seconds. The screenplay is
//! not written in seconds — it is written in **bars**, at 84 bpm in 4/4, and
//! it sets itself the rule that *every cut lands on a bar line*. Authoring in
//! seconds makes that rule unenforceable: 2.857142857 s is a bar only if you
//! type enough sevens.
//!
//! So a shot states `durationBars`, and `durationSec` is derived. Exactly one
//! of the two may appear, and the derived value is what the resolver works
//! in. The canonical cut is **84 bars = 240.000 s exactly**, which is only
//! exact because 84 × 20/7 is.
//!
//! # `target` is a glob, resolved once
//!
//! `"monolith/*"`, `"atlas/site-07/**"`, `"flag/dk/tile-*"` — matched against
//! the bundle's own `instanceIds` at load time and turned into an index list.
//! Never re-globbed per frame: at Atlas scale that would be a string match
//! per brick per track per frame.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const FORMAT_VERSION: u32 = 1;

/// 84 bpm in 4/4 — the tempo the whole piece is written at. One bar is
/// 4 beats × 60/84 s = **20/7 s**, and the canonical cut is 84 of them.
pub const DEFAULT_BPM: f64 = 84.0;
pub const DEFAULT_BEATS_PER_BAR: u32 = 4;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tempo {
    pub bpm: f64,
    #[serde(rename = "beatsPerBar")]
    pub beats_per_bar: u32,
}

impl Default for Tempo {
    fn default() -> Self {
        Self { bpm: DEFAULT_BPM, beats_per_bar: DEFAULT_BEATS_PER_BAR }
    }
}

impl Tempo {
    /// Seconds per bar. At the default tempo this is exactly 20/7.
    pub fn bar_seconds(&self) -> f64 {
        (self.beats_per_bar as f64) * 60.0 / self.bpm
    }
    pub fn beat_seconds(&self) -> f64 {
        60.0 / self.bpm
    }
}

/// The authored source document — `show.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Show {
    pub version: u32,
    pub id: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,
    #[serde(rename = "archiveSignature")]
    pub archive_signature: String,
    #[serde(default)]
    pub tempo: Tempo,
    /// The canonical cut, in bars. 84 at the default tempo — 240.000 s.
    #[serde(rename = "baseDurationBars")]
    pub base_duration_bars: f64,
    /// Default edition seed. Every random choice in the piece descends from
    /// this one number, which is what makes an edition reproducible.
    pub seed: u64,
    /// Named colours, **linear** rgb — the same convention as `mesh.json`,
    /// for the same reason: three.js reads raw components as linear.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub palette: BTreeMap<String, [f32; 3]>,
    pub scenes: Vec<SceneRef>,
    pub movements: Vec<Movement>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credits: Option<Credits>,
}

impl Show {
    pub fn base_duration_sec(&self) -> f64 {
        self.base_duration_bars * self.tempo.bar_seconds()
    }
    pub fn shots(&self) -> impl Iterator<Item = (&Movement, &Shot)> {
        self.movements.iter().flat_map(|m| m.shots.iter().map(move |s| (m, s)))
    }
}

/// Where a scene's geometry comes from. Four kinds, because four different
/// milestones produce geometry and the document should not care which.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum SceneSource {
    /// A real `.ldr` file in this repository.
    Ldr { path: String },
    /// A `spex-build` recipe (M72).
    Build { recipe: String },
    /// A `spex-flag` specification id (M75).
    Flag { flag: String },
    /// A World Heritage site id (M73).
    Heritage { #[serde(rename = "siteId")] site_id: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneRef {
    pub id: String,
    pub source: SceneSource,
    /// Instance-id prefix, so choreography can address subsets by glob.
    pub prefix: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Movement {
    pub id: String,
    pub title: String,
    #[serde(rename = "romanNumeral")]
    pub roman_numeral: String,
    pub shots: Vec<Shot>,
}

impl Movement {
    pub fn duration_bars(&self) -> f64 {
        self.shots.iter().map(|s| s.duration_bars).sum()
    }
}

/// How a shot's duration responds to the cut it is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Scaling {
    /// Never changes, in any cut. The Kick is two beats in the 4:00 and two
    /// beats in the 60:00 — a flash that lasts ten seconds is not a flash.
    Fixed,
    /// Scales with the cut, clamped to min/max.
    Stretch,
    /// The body loops N times; N scales, one pass does not.
    Repeat,
}

/// Which cuts a shot appears in. Tier 1 is in all of them; the 10:00 adds
/// tier 2, the 60:00 adds tier 3. A cut is a *selection*, not a speed change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "u8", into = "u8")]
pub enum Tier {
    Always = 1,
    Long = 2,
    Full = 3,
}

impl TryFrom<u8> for Tier {
    type Error = String;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            1 => Ok(Tier::Always),
            2 => Ok(Tier::Long),
            3 => Ok(Tier::Full),
            other => Err(format!("tier must be 1, 2 or 3; got {other}")),
        }
    }
}

impl From<Tier> for u8 {
    fn from(t: Tier) -> u8 {
        t as u8
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RepeatSpec {
    #[serde(rename = "unitBars")]
    pub unit_bars: f64,
    #[serde(rename = "minCount")]
    pub min_count: u32,
    #[serde(rename = "maxCount")]
    pub max_count: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Shot {
    pub id: String,
    pub title: String,
    /// Proportional share of the movement's stretchable time.
    pub weight: f64,
    pub scaling: Scaling,
    /// **In bars.** See this module's header for why not seconds.
    #[serde(rename = "durationBars")]
    pub duration_bars: f64,
    #[serde(default, rename = "minBars", skip_serializing_if = "Option::is_none")]
    pub min_bars: Option<f64>,
    #[serde(default, rename = "maxBars", skip_serializing_if = "Option::is_none")]
    pub max_bars: Option<f64>,
    pub tier: Tier,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repeat: Option<RepeatSpec>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scenes: Vec<String>,
    pub camera: CameraTrack,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tracks: Vec<Track>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cues: Vec<Cue>,
    /// Free-form direction, carried into the resolved output so the piece
    /// documents itself — shown by the director HUD (`?director=1`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Easing {
    Linear,
    QuadIn,
    QuadOut,
    QuadInOut,
    CubicIn,
    CubicOut,
    CubicInOut,
    ExpoIn,
    ExpoOut,
    /// Instant. The edge lines in A1-S03 arrive in **one frame** — a fade
    /// would make legibility gradual, and the whole shot is about it not
    /// being gradual.
    Step,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Keyframe<T> {
    /// Normalised shot-local time, 0..1 — so a keyframe survives retiming.
    pub t: f64,
    pub value: T,
    #[serde(default = "default_easing")]
    pub easing: Easing,
    /// Snap to the nearest beat instead of to normalised time. The resolver
    /// rewrites `t`. This is how a landing brick lands *on* the beat rather
    /// than near it.
    #[serde(default, rename = "snapToBeat", skip_serializing_if = "std::ops::Not::not")]
    pub snap_to_beat: bool,
}

fn default_easing() -> Easing {
    Easing::Linear
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct TransformValue {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<[f64; 3]>,
    /// Euler XYZ in degrees. Ignored if `quaternion` is present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rotation: Option<[f64; 3]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quaternion: Option<[f64; 4]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scale: Option<f64>,
}

/// Material properties a track may animate. Deliberately a closed set: an
/// open string would let a show reference a property no renderer has.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MaterialProperty {
    Opacity,
    Roughness,
    Metalness,
    EmissiveIntensity,
    Transmission,
    /// M57's edge lines, separately from the solid. Added while authoring
    /// A1-S03: the crossfade lands the mesh and the outlines arrive **one
    /// frame later**, and with no channel of its own that beat could only be
    /// written as a comment. A closed set is only useful if adding to it is
    /// the normal way to make a shot authorable.
    EdgeOpacity,
}

/// Post-chain parameters M58 exposes. Same closed-set reasoning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PostProperty {
    Exposure,
    BloomThreshold,
    BloomStrength,
    BloomRadius,
    Vignette,
    GradeStrength,
    Grain,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Track {
    Transform { target: String, keys: Vec<Keyframe<TransformValue>> },
    Dissolve { target: String, keys: Vec<Keyframe<f64>> },
    Material { target: String, property: MaterialProperty, keys: Vec<Keyframe<f64>> },
    Post { property: PostProperty, keys: Vec<Keyframe<f64>> },
    Hud { element: String, keys: Vec<Keyframe<f64>> },
    /// The mesh↔point crossfade. A1-S03 is the centre of the whole piece and
    /// it is exactly this track.
    PointCloud { target: String, keys: Vec<Keyframe<f64>> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CameraMode {
    Keyed,
    Orbit,
    Dolly,
    /// The Kick.
    ExponentialZoom,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct CameraValue {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<[f64; 3]>,
    #[serde(default, rename = "lookAt", skip_serializing_if = "Option::is_none")]
    pub look_at: Option<[f64; 3]>,
    #[serde(default, rename = "fovDeg", skip_serializing_if = "Option::is_none")]
    pub fov_deg: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrbitSpec {
    pub center: [f64; 3],
    pub radius: f64,
    pub height: f64,
    #[serde(rename = "startDeg")]
    pub start_deg: f64,
    #[serde(rename = "endDeg")]
    pub end_deg: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DollySpec {
    pub from: [f64; 3],
    pub to: [f64; 3],
    #[serde(rename = "lookAt")]
    pub look_at: [f64; 3],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ZoomSpec {
    pub from: f64,
    pub to: f64,
    #[serde(rename = "lookAt")]
    pub look_at: [f64; 3],
    /// Which way the camera pulls back. A distance and a look-at point do
    /// not determine a position — M63 found the gap while implementing the
    /// Kick, and a director that silently picks an axis is a director that
    /// picks a different one after a refactor. Optional, defaulting to the
    /// piece's own framing axis (`[0, 0.15, 1]`, straight back with a slight
    /// rise), so every document written before this stays valid.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub direction: Option<[f64; 3]>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CameraTrack {
    pub mode: CameraMode,
    #[serde(default, rename = "fovDeg", skip_serializing_if = "Option::is_none")]
    pub fov_deg: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keys: Option<Vec<Keyframe<CameraValue>>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub orbit: Option<OrbitSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dolly: Option<DollySpec>,
    #[serde(default, rename = "exponentialZoom", skip_serializing_if = "Option::is_none")]
    pub exponential_zoom: Option<ZoomSpec>,
    /// Shutter-style motion blur strength, 0..1.
    #[serde(default, rename = "motionBlur", skip_serializing_if = "Option::is_none")]
    pub motion_blur: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CueKind {
    Audio,
    Hud,
    Seed,
    Marker,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Cue {
    /// Normalised shot-local time.
    pub t: f64,
    pub kind: CueKind,
    #[serde(default)]
    pub payload: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Credits {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lines: Vec<String>,
    /// Attribution that must appear regardless of cut length — the LDraw
    /// library's CCAL terms among them. Not stretchable, not tier-gated.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required: Vec<String>,
}

/// Everything structurally wrong with a document, in one pass.
///
/// One error at a time would mean one build per mistake; an author fixing a
/// screenplay wants the whole list.
pub fn validate(show: &Show) -> Vec<String> {
    let mut errs = Vec::new();
    if show.version != FORMAT_VERSION {
        errs.push(format!("version is {}, expected {FORMAT_VERSION}", show.version));
    }
    if show.movements.is_empty() {
        errs.push("a show with no movements has nothing to play".into());
    }
    let scene_ids: Vec<&str> = show.scenes.iter().map(|s| s.id.as_str()).collect();

    let mut seen: Vec<&str> = Vec::new();
    for (movement, shot) in show.shots() {
        let where_ = format!("{}/{}", movement.id, shot.id);
        if seen.contains(&shot.id.as_str()) {
            errs.push(format!("{where_}: duplicate shot id — targets and cues address shots by id"));
        }
        seen.push(&shot.id);

        if shot.duration_bars <= 0.0 {
            errs.push(format!("{where_}: durationBars must be > 0"));
        }
        if let (Some(min), Some(max)) = (shot.min_bars, shot.max_bars) {
            if min > max {
                errs.push(format!("{where_}: minBars {min} > maxBars {max}"));
            }
        }
        if shot.scaling == Scaling::Fixed && (shot.min_bars.is_some() || shot.max_bars.is_some()) {
            errs.push(format!(
                "{where_}: scaling is 'fixed' but min/maxBars are set — a fixed shot has one duration in every cut"
            ));
        }
        if shot.scaling == Scaling::Repeat && shot.repeat.is_none() {
            errs.push(format!("{where_}: scaling is 'repeat' but no repeat spec says what a pass is"));
        }
        for scene in &shot.scenes {
            if !scene_ids.contains(&scene.as_str()) {
                errs.push(format!("{where_}: references scene {scene:?}, which the document does not define"));
            }
        }
        for (i, cue) in shot.cues.iter().enumerate() {
            if !(0.0..=1.0).contains(&cue.t) {
                errs.push(format!("{where_}: cue {i} is at t={}, outside 0..1", cue.t));
            }
        }
        errs.extend(camera_errors(&where_, &shot.camera));
        for (i, track) in shot.tracks.iter().enumerate() {
            errs.extend(track_errors(&format!("{where_} track {i}"), track));
        }
    }
    errs
}

fn camera_errors(where_: &str, cam: &CameraTrack) -> Vec<String> {
    // Each mode needs its own parameters and no others. A document that says
    // `orbit` and carries only a dolly is a silent mis-shot at runtime.
    let (needed, present): (&str, bool) = match cam.mode {
        CameraMode::Keyed => ("keys", cam.keys.is_some()),
        CameraMode::Orbit => ("orbit", cam.orbit.is_some()),
        CameraMode::Dolly => ("dolly", cam.dolly.is_some()),
        CameraMode::ExponentialZoom => ("exponentialZoom", cam.exponential_zoom.is_some()),
    };
    if !present {
        return vec![format!("{where_}: camera mode is {:?} but has no `{needed}` block", cam.mode)];
    }
    Vec::new()
}

fn keys_ordered<T>(where_: &str, keys: &[Keyframe<T>]) -> Vec<String> {
    let mut errs = Vec::new();
    if keys.is_empty() {
        errs.push(format!("{where_}: no keyframes"));
        return errs;
    }
    for (i, k) in keys.iter().enumerate() {
        if !(0.0..=1.0).contains(&k.t) {
            errs.push(format!("{where_}: key {i} at t={}, outside 0..1", k.t));
        }
        if i > 0 && k.t < keys[i - 1].t {
            errs.push(format!("{where_}: key {i} at t={} goes backwards", k.t));
        }
    }
    errs
}

fn track_errors(where_: &str, track: &Track) -> Vec<String> {
    match track {
        Track::Transform { keys, .. } => keys_ordered(where_, keys),
        Track::Dissolve { keys, .. }
        | Track::Material { keys, .. }
        | Track::Post { keys, .. }
        | Track::Hud { keys, .. }
        | Track::PointCloud { keys, .. } => keys_ordered(where_, keys),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bar_is_twenty_sevenths_of_a_second_and_eighty_four_of_them_are_exactly_four_minutes() {
        let t = Tempo::default();
        assert!((t.bar_seconds() - 20.0 / 7.0).abs() < 1e-12);
        // The reason the canonical cut is 84 bars and not 80: 84 x 20/7 is
        // exactly 240, and nothing else nearby is.
        assert!((84.0 * t.bar_seconds() - 240.0).abs() < 1e-9);
    }

    fn shot(id: &str, bars: f64, scaling: Scaling) -> Shot {
        Shot {
            id: id.into(),
            title: id.into(),
            weight: 1.0,
            scaling,
            duration_bars: bars,
            min_bars: None,
            max_bars: None,
            tier: Tier::Always,
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
            version: FORMAT_VERSION,
            id: "t".into(),
            title: "t".into(),
            subtitle: None,
            archive_signature: "IA-2026-002".into(),
            tempo: Tempo::default(),
            base_duration_bars: 84.0,
            seed: 1,
            palette: BTreeMap::new(),
            scenes: vec![],
            movements: vec![Movement {
                id: "act-1".into(),
                title: "t".into(),
                roman_numeral: "I".into(),
                shots,
            }],
            credits: None,
        }
    }

    #[test]
    fn a_valid_document_reports_nothing() {
        assert!(validate(&show_of(vec![shot("A1-S01", 2.0, Scaling::Stretch)])).is_empty());
    }

    #[test]
    fn a_fixed_shot_may_not_also_be_stretchable() {
        // The Kick is two beats in every cut. Saying "fixed" and then giving
        // it a range is a contradiction the resolver would have to guess at.
        let mut s = shot("A4-S03", 0.5, Scaling::Fixed);
        s.max_bars = Some(4.0);
        let errs = validate(&show_of(vec![s]));
        assert_eq!(errs.len(), 1);
        assert!(errs[0].contains("fixed"), "{errs:?}");
    }

    #[test]
    fn a_repeat_shot_must_say_what_one_pass_is() {
        let errs = validate(&show_of(vec![shot("X", 4.0, Scaling::Repeat)]));
        assert_eq!(errs.len(), 1);
        assert!(errs[0].contains("repeat spec"), "{errs:?}");
    }

    #[test]
    fn duplicate_shot_ids_are_caught_because_everything_addresses_shots_by_id() {
        let errs = validate(&show_of(vec![
            shot("A1-S01", 2.0, Scaling::Stretch),
            shot("A1-S01", 2.0, Scaling::Stretch),
        ]));
        assert!(errs.iter().any(|e| e.contains("duplicate")), "{errs:?}");
    }

    #[test]
    fn a_camera_mode_without_its_own_block_is_an_error_not_a_silent_default() {
        let mut s = shot("X", 2.0, Scaling::Stretch);
        s.camera = CameraTrack {
            mode: CameraMode::Orbit,
            fov_deg: None,
            keys: None,
            orbit: None,
            dolly: None,
            exponential_zoom: None,
            motion_blur: None,
        };
        let errs = validate(&show_of(vec![s]));
        assert!(errs.iter().any(|e| e.contains("no `orbit` block")), "{errs:?}");
    }

    #[test]
    fn keyframes_must_not_go_backwards() {
        let mut s = shot("X", 2.0, Scaling::Stretch);
        s.tracks = vec![Track::Dissolve {
            target: "monolith/*".into(),
            keys: vec![
                Keyframe { t: 0.6, value: 0.0, easing: Easing::Linear, snap_to_beat: false },
                Keyframe { t: 0.2, value: 1.0, easing: Easing::Linear, snap_to_beat: false },
            ],
        }];
        let errs = validate(&show_of(vec![s]));
        assert!(errs.iter().any(|e| e.contains("goes backwards")), "{errs:?}");
    }

    #[test]
    fn a_shot_may_not_reference_a_scene_the_document_never_defines() {
        let mut s = shot("X", 2.0, Scaling::Stretch);
        s.scenes = vec!["monolith".into()];
        let errs = validate(&show_of(vec![s]));
        assert!(errs.iter().any(|e| e.contains("does not define")), "{errs:?}");
    }

    #[test]
    fn every_error_is_reported_in_one_pass() {
        // An author fixing a screenplay wants the whole list, not one build
        // per mistake.
        let mut a = shot("A", 0.0, Scaling::Repeat);
        a.scenes = vec!["nope".into()];
        let mut b = shot("B", 2.0, Scaling::Fixed);
        b.min_bars = Some(1.0);
        let errs = validate(&show_of(vec![a, b]));
        assert!(errs.len() >= 4, "expected several at once, got {errs:?}");
    }

    #[test]
    fn the_document_round_trips_through_json_unchanged() {
        let show = show_of(vec![shot("A1-S01", 2.0, Scaling::Stretch)]);
        let json = serde_json::to_string_pretty(&show).unwrap();
        let back: Show = serde_json::from_str(&json).unwrap();
        assert_eq!(show, back);
    }

    #[test]
    fn tier_is_one_two_or_three_and_nothing_else() {
        assert_eq!(serde_json::from_str::<Tier>("1").unwrap(), Tier::Always);
        assert_eq!(serde_json::to_string(&Tier::Full).unwrap(), "3");
        assert!(serde_json::from_str::<Tier>("4").is_err());
    }
}
