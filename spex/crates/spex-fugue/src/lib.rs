//! M67 — the fugue's plan, and the music theory it is written in.
//!
//! Two modules and no dependencies beyond serde, for the same reason
//! `spex-show` has none: M69's WebAudio engine and M68's constraint solver
//! both need this, and neither should have to bring a renderer, a mesh
//! bundle or a file format along to get it.
//!
//! - [`theory`] — modes, degrees, diatonic transposition, inversion, the
//!   tonal answer, and the vertical analysis the counterpoint rules are
//!   stated in. Pure integer arithmetic, no time, no I/O, no randomness.
//! - [`model`] — [`model::FugueSpec`], the plan carried in `show.json`'s
//!   `audio` field, and the two authored lines: the subject and the
//!   countersubject.
//!
//! The score itself is not here. M68 emits a standard MIDI file, and that
//! file is both what the browser plays and what a person opens in a DAW —
//! one artefact, so review and runtime cannot disagree.

pub mod model;
pub mod theory;

pub use model::{
    act_one_countersubject, act_one_key, act_one_subject, subject_facts, FugueSpec, PulseSpec,
    Section, SubjectFacts,
};
pub use theory::{Key, Line, Mode, Note};
