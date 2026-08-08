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
//! - [`counterpoint`] (M68) — the rules, as named predicates that return
//!   *where* they were broken, and the generator that realises a plan into
//!   four voices under them. Deterministic from `(seed, spec)`; what it cannot
//!   satisfy it relaxes in a fixed order and records.
//! - [`emit`] (M68) — a hand-rolled type-1 SMF writer, and a reader, because
//!   the acceptance criterion is about the *emitted* score rather than about
//!   what the generator was holding in memory.
//!
//! The score is one artefact: the browser plays the same file a person opens
//! in a DAW, so review and runtime cannot disagree.

pub mod counterpoint;
pub mod emit;
pub mod model;
pub mod theory;

pub use model::{
    act_one_countersubject, act_one_key, act_one_subject, subject_facts, FugueSpec, PulseSpec,
    Section, SubjectFacts,
};
pub use counterpoint::{realise, Placed, Realisation, Relaxation, Rule};
pub use emit::{read_smf, to_smf};
pub use theory::{Key, Line, Mode, Note};
