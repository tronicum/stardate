//! `spex-show` — the show document: the screenplay as data.
//!
//! Phase 2's first crate. `model.rs` is the *authored* document — what a human
//! writes and what `spec/show.schema.json` describes. Later milestones add the
//! resolver (M61) that turns it into the *resolved* document
//! (`spec/show-resolved.schema.json`): globs expanded to index lists, bars
//! turned into absolute seconds, a chosen duration cut applied.
//!
//! The crate deliberately depends on nothing in the workspace. A show document
//! is text; loading one must not require a mesh bundle, a tileset or a
//! renderer, because the schema tests, the CLI and eventually the viewer all
//! need to read one in contexts where none of those exist.

pub mod choreography;
pub mod compile;
pub mod model;
pub mod resolve;

pub use choreography::{
    placement_seed, splitmix64, staggered_progress, start_offset_ldu, FLOAT_HEIGHT_LDU,
    SCATTER_RADIUS_LDU,
};
pub use compile::{bind_targets, glob_matches, SceneInstances};
pub use resolve::{
    resolve, ResolveOptions, ResolvedCameraTrack, ResolvedCue, ResolvedKey, ResolvedScene,
    ResolvedShot, ResolvedShow, ResolvedTempo, ResolvedTrack, TargetBinding,
    RESOLVED_FORMAT_VERSION,
};
pub use model::{
    validate, CameraMode, CameraTrack, CameraValue, Credits, Cue, CueKind, DollySpec,
    Easing, Keyframe, MaterialProperty, Movement, OrbitSpec, PostProperty, RepeatSpec, Scaling,
    SceneRef, SceneSource, Shot, Show, Tempo, Tier, Track, TransformValue, ZoomSpec,
    DEFAULT_BEATS_PER_BAR, DEFAULT_BPM, FORMAT_VERSION,
};

use std::path::Path;

/// Reads and parses a show document, then validates it.
///
/// Parsing and validation are one step on purpose: a `Show` that parsed but
/// does not validate is not a useful intermediate value for any caller, and
/// making the two separate invites code that forgets the second.
pub fn load(path: &Path) -> anyhow::Result<Show> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("reading show document {}: {e}", path.display()))?;
    from_str(&text).map_err(|e| anyhow::anyhow!("{}: {e}", path.display()))
}

/// Same, from a string already in memory.
pub fn from_str(text: &str) -> anyhow::Result<Show> {
    let show: Show = serde_json::from_str(text)?;
    let errors = validate(&show);
    if !errors.is_empty() {
        // Every error, not the first: an author fixing a document one message
        // at a time re-runs the tool once per mistake.
        anyhow::bail!("show document is invalid:\n  - {}", errors.join("\n  - "));
    }
    Ok(show)
}
