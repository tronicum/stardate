//! Real LDraw *model*/scene parsing: a flat sequence of a model file's own
//! real type-1 placement lines (plus real `0 STEP` build-stage markers) —
//! the "build instructions" half of this crate, as opposed to
//! `geometry.rs`'s single-part resolution. Deliberately does not recurse
//! into each placement's own part geometry (that's `geometry::resolve_part`'s
//! job, called once per *distinct* placement by the caller).
use crate::cache::LdrawCache;
use anyhow::{Context, Result};
use std::path::Path;

/// One real placement: which real part, what real color, and its real
/// translation + 3x3 rotation/scale matrix (LDraw's own type-1 line
/// format) in the scene's own root LDU frame.
#[derive(Clone, Debug, PartialEq)]
pub struct Placement {
    pub part_file: String,
    pub color_code: u32,
    pub translation: [f64; 3],
    pub matrix: [f64; 9],
    pub build_step: u32,
}

#[derive(Clone, Debug, Default)]
pub struct Scene {
    pub source_description: Option<String>,
    pub source_author: Option<String>,
    pub placements: Vec<Placement>,
}

/// Where a real `.ldr` model's text comes from: a named official model
/// fetched from ldraw.org's real `models/` folder (e.g. "car", "pyramid"),
/// or a local file already on disk (e.g. a hand-authored scene like
/// `ldraw-scenes/monolith.ldr`).
pub enum ModelSource<'a> {
    Named(&'a str),
    LocalFile(&'a Path),
}

pub fn parse_scene(cache: &LdrawCache, source: ModelSource) -> Result<Scene> {
    let text = match source {
        ModelSource::Named(name) => cache
            .fetch(&format!("models/{name}.ldr"))
            .with_context(|| format!("fetching real official model {name:?}"))?,
        ModelSource::LocalFile(path) => std::fs::read_to_string(path)
            .with_context(|| format!("reading local scene file {}", path.display()))?,
    };

    let mut description = None;
    let mut author = None;
    for line in text.lines() {
        let mut tokens = line.splitn(2, char::is_whitespace);
        if tokens.next() != Some("0") {
            continue;
        }
        let Some(rest) = tokens.next() else { continue };
        let rest = rest.trim();
        if rest.starts_with("//") {
            continue;
        }
        if let Some(value) = rest.strip_prefix("Author:") {
            author = Some(value.trim().to_string());
        } else if rest.starts_with("Name:") {
            // just restates the model's own filename, not useful beyond sourceModel
        } else if description.is_none() {
            description = Some(rest.to_string());
        }
    }

    let mut placements = Vec::new();
    let mut build_step = 0u32;
    for line in text.lines() {
        let tokens: Vec<&str> = line.split_whitespace().collect();
        let Some(&line_type) = tokens.first() else {
            continue;
        };
        if line_type == "0" {
            if tokens.get(1) == Some(&"STEP") {
                build_step += 1;
            }
            continue;
        }
        if line_type != "1" || tokens.len() < 15 {
            continue;
        }
        let Ok(color_code) = tokens[1].parse::<u32>() else {
            continue;
        };
        let nums: Result<Vec<f64>, _> = tokens[2..14].iter().map(|t| t.parse::<f64>()).collect();
        let Ok(nums) = nums else { continue };
        let translation = [nums[0], nums[1], nums[2]];
        let matrix: [f64; 9] = nums[3..12].try_into().unwrap();
        let part_file = tokens[14..].join(" ");
        placements.push(Placement {
            part_file,
            color_code,
            translation,
            matrix,
            build_step,
        });
    }

    Ok(Scene {
        source_description: description,
        source_author: author,
        placements,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_real_shaped_ldr_model_with_steps_and_header() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("test.ldr"),
            "0 Example Test Model\n\
             0 Name: test.ldr\n\
             0 Author: Test Author\n\
             0 // a comment, not a header field\n\
             1 4 0 0 0 1 0 0 0 1 0 0 0 1 3005.dat\n\
             \n\
             0 STEP\n\
             1 7 10 0 0 1 0 0 0 1 0 0 0 1 3010.dat\n",
        )
        .unwrap();
        let cache = LdrawCache::new(dir.path());
        let scene = parse_scene(&cache, ModelSource::LocalFile(&dir.path().join("test.ldr"))).unwrap();

        assert_eq!(scene.source_description, Some("Example Test Model".to_string()));
        assert_eq!(scene.source_author, Some("Test Author".to_string()));
        assert_eq!(scene.placements.len(), 2);
        assert_eq!(scene.placements[0].part_file, "3005.dat");
        assert_eq!(scene.placements[0].color_code, 4);
        assert_eq!(scene.placements[0].build_step, 0);
        assert_eq!(scene.placements[1].part_file, "3010.dat");
        assert_eq!(scene.placements[1].translation, [10.0, 0.0, 0.0]);
        assert_eq!(scene.placements[1].build_step, 1, "a real 0 STEP line must increment the build step for everything after it");
    }

    #[test]
    fn fetches_a_named_model_from_the_real_models_folder() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("models")).unwrap();
        std::fs::write(dir.path().join("models/pyramid.ldr"), "0 A Real Pyramid\n1 1 0 0 0 1 0 0 0 1 0 0 0 1 3001.dat\n").unwrap();
        let cache = LdrawCache::new(dir.path());
        let scene = parse_scene(&cache, ModelSource::Named("pyramid")).unwrap();
        assert_eq!(scene.source_description, Some("A Real Pyramid".to_string()));
        assert_eq!(scene.placements.len(), 1);
    }
}
