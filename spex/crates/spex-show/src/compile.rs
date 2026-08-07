//! M61 — binding globs to instance indices, once.
//!
//! A track says `"monolith/**"`. A mesh bundle says its instances are called
//! `3010/0`, `3010/1`, …, `3710/8`. This module is where those two facts meet
//! and become a list of integers.
//!
//! # Why this happens exactly once
//!
//! The obvious implementation matches the glob per frame. At Atlas scale that
//! is a string comparison per brick per track per frame — 40 sites of a few
//! thousand bricks each, at 60 Hz, is tens of millions of string matches a
//! second to compute an answer that never changes. So it happens at build
//! time, and the resolved document carries the indices.
//!
//! # `*` does not cross a separator
//!
//! A scene's addressable ids are `<prefix>/<part>/<n>` — `monolith/3010/0` —
//! which is **two** separators. `*` matches within one segment and `**` spans
//! any number, exactly as in every other glob dialect, which is what makes the
//! spec's own `flag/dk/tile-*` example mean something narrower than "all of
//! it". The consequence worth stating out loud: `monolith/*` matches *nothing*
//! and produces no error anywhere — the shot simply never animates. It is the
//! quiet failure this module's tests are mostly about.
//!
//! This module deliberately knows nothing about LDraw, meshes or files. It
//! takes instance ids as strings, because the WASM build (Phase 6) resolves
//! timelines without any of that, and a glob matcher that dragged the whole
//! part resolver behind it would be the reason the wasm bundle is megabytes.

use crate::resolve::{ResolvedShow, TargetBinding};
use std::collections::BTreeMap;

/// A scene's own instance ids, in bundle order — index *is* the instance
/// index, which is why this is a `Vec` and not a set.
pub type SceneInstances = BTreeMap<String, Vec<String>>;

/// Does `glob` match `path`?
///
/// Supports `*` (any run of characters inside one segment) and `**` (any run,
/// separators included). Everything else is literal. No character classes, no
/// braces: a show document is not a shell, and an unsupported construct that
/// silently matched nothing would be the same failure as `monolith/*`.
pub fn glob_matches(glob: &str, path: &str) -> bool {
    matches_from(glob.as_bytes(), path.as_bytes())
}

fn matches_from(g: &[u8], p: &[u8]) -> bool {
    if g.is_empty() {
        return p.is_empty();
    }
    if g[0] == b'*' {
        let (crosses, rest) = if g.len() > 1 && g[1] == b'*' { (true, &g[2..]) } else { (false, &g[1..]) };
        // Try every length this wildcard could consume, shortest first.
        for take in 0..=p.len() {
            if !crosses && p[..take].contains(&b'/') {
                break;
            }
            if matches_from(rest, &p[take..]) {
                return true;
            }
        }
        return false;
    }
    if p.is_empty() || g[0] != p[0] {
        return false;
    }
    matches_from(&g[1..], &p[1..])
}

/// What a glob turned out to select, and where.
#[derive(Debug, Clone, PartialEq)]
pub struct Binding {
    pub scene: Option<String>,
    pub instances: Vec<u32>,
}

/// Expands one glob against every scene's instance ids.
///
/// The scene is identified by the glob's first segment matching a scene's
/// `prefix`, so a target can only ever address one scene — which is the point.
/// A track that meant to address two scenes is two tracks, and one that
/// addresses none is a typo the caller should hear about.
pub fn bind(prefixes: &BTreeMap<String, String>, instances: &SceneInstances, glob: &str) -> Binding {
    let head = glob.split('/').next().unwrap_or("");
    let Some(scene_id) = prefixes.get(head) else {
        return Binding { scene: None, instances: Vec::new() };
    };
    let ids = match instances.get(scene_id) {
        Some(ids) => ids,
        None => return Binding { scene: Some(scene_id.clone()), instances: Vec::new() },
    };
    let matched = ids
        .iter()
        .enumerate()
        .filter(|(_, id)| glob_matches(glob, &format!("{head}/{id}")))
        .map(|(i, _)| i as u32)
        .collect();
    Binding { scene: Some(scene_id.clone()), instances: matched }
}

/// Fills in every track's `TargetBinding` in place, and reports the globs that
/// matched nothing.
///
/// Returns the empty matches rather than failing, because a partially built
/// show is still worth looking at and the caller (the CLI) is the right place
/// to decide whether an unmatched glob is fatal. What it must not do is stay
/// quiet: a target that matches nothing is invisible at runtime.
pub fn bind_targets(show: &mut ResolvedShow, instances: &SceneInstances) -> Vec<String> {
    let prefixes: BTreeMap<String, String> =
        show.scenes.iter().map(|s| (s.prefix.clone(), s.id.clone())).collect();

    let mut empty = Vec::new();
    for shot in &mut show.shots {
        let shot_id = shot.id.clone();
        for track in &mut shot.tracks {
            let Some(target): Option<&mut TargetBinding> = track.target_mut() else { continue };
            let b = bind(&prefixes, instances, &target.glob);
            if b.instances.is_empty() {
                empty.push(format!("{shot_id}: {:?}", target.glob));
            }
            target.scene = b.scene;
            target.instances = Some(b.instances);
        }
    }
    for scene in &mut show.scenes {
        if let Some(ids) = instances.get(&scene.id) {
            scene.instance_count = Some(ids.len() as u32);
        }
    }
    empty
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_single_star_stays_inside_one_segment() {
        assert!(glob_matches("monolith/*", "monolith/3010"));
        // The one that matters: a real bundle id has a second separator.
        assert!(!glob_matches("monolith/*", "monolith/3010/0"));
        assert!(glob_matches("monolith/**", "monolith/3010/0"));
        assert!(glob_matches("monolith/**", "monolith/3010"));
    }

    #[test]
    fn a_partial_segment_pattern_works_the_way_the_spec_advertises() {
        assert!(glob_matches("flag/dk/tile-*", "flag/dk/tile-07"));
        assert!(!glob_matches("flag/dk/tile-*", "flag/dk/other-07"));
        assert!(!glob_matches("flag/dk/tile-*", "flag/dk/tile-07/2"));
        assert!(glob_matches("atlas/site-07/**", "atlas/site-07/3001/12"));
        assert!(!glob_matches("atlas/site-07/**", "atlas/site-08/3001/12"));
    }

    #[test]
    fn a_literal_glob_matches_exactly_one_instance() {
        assert!(glob_matches("brick/3005/0", "brick/3005/0"));
        assert!(!glob_matches("brick/3005/0", "brick/3005/1"));
    }

    fn fixture() -> (BTreeMap<String, String>, SceneInstances) {
        let prefixes = BTreeMap::from([
            ("monolith".to_string(), "monolith".to_string()),
            ("brick".to_string(), "brick".to_string()),
        ]);
        let instances = SceneInstances::from([
            (
                "monolith".to_string(),
                vec![
                    "3010/0".into(),
                    "3010/1".into(),
                    "3010/2".into(),
                    "3710/3".into(),
                ],
            ),
            ("brick".to_string(), vec!["3005/0".into()]),
        ]);
        (prefixes, instances)
    }

    #[test]
    fn binding_returns_indices_into_the_bundle_not_names() {
        let (p, i) = fixture();
        let b = bind(&p, &i, "monolith/**");
        assert_eq!(b.scene.as_deref(), Some("monolith"));
        assert_eq!(b.instances, vec![0, 1, 2, 3]);

        let only_bricks = bind(&p, &i, "monolith/3010/**");
        assert_eq!(only_bricks.instances, vec![0, 1, 2], "the two plates are not 3010s");
    }

    /// The whole reason this module has tests.
    #[test]
    fn the_single_star_mistake_binds_to_nothing_and_is_visible() {
        let (p, i) = fixture();
        let b = bind(&p, &i, "monolith/*");
        assert!(b.instances.is_empty());
        assert_eq!(b.scene.as_deref(), Some("monolith"), "the scene is still identified, so a caller can say which glob went wrong");
    }

    #[test]
    fn a_glob_whose_head_is_no_scenes_prefix_binds_to_no_scene_at_all() {
        let (p, i) = fixture();
        let b = bind(&p, &i, "monolth/**");
        assert_eq!(b.scene, None);
        assert!(b.instances.is_empty());
    }
}
