//! Real Ankerstein assembly/scene format — deliberately simpler than
//! `spex-ldraw::scene`'s full LDraw `.ldr` model parsing (no LDraw file to
//! parse; a plain JSON placement list instead). Only translation + a
//! single Y-axis rotation are supported for now (see
//! `docs/ANKERSTEIN-ENGINE.md` §3) — add full 3x3-matrix rotation only
//! when a real design (e.g. a sloped roof stone) actually needs it, not
//! speculatively.
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// One real placement: which catalog shape, where, and how it's rotated
/// about the vertical (Y) axis.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Placement {
    pub shape_id: String,
    pub translation_mm: [f64; 3],
    #[serde(default)]
    pub rotation_y_degrees: f64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Scene {
    #[serde(default)]
    pub title: Option<String>,
    pub placements: Vec<Placement>,
}

pub fn parse_scene(path: &std::path::Path) -> Result<Scene> {
    let text = std::fs::read_to_string(path).with_context(|| format!("reading scene file {}", path.display()))?;
    let scene: Scene = serde_json::from_str(&text).with_context(|| format!("parsing scene file {}", path.display()))?;
    Ok(scene)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_real_shaped_scene_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test-scene.json");
        std::fs::write(
            &path,
            r#"{
                "title": "A small test wall",
                "placements": [
                    {"shape_id": "gk-cube-full", "translation_mm": [0.0, 0.0, 0.0]},
                    {"shape_id": "gk-cube-full", "translation_mm": [25.0, 0.0, 0.0], "rotation_y_degrees": 90.0}
                ]
            }"#,
        )
        .unwrap();
        let scene = parse_scene(&path).unwrap();
        assert_eq!(scene.title, Some("A small test wall".to_string()));
        assert_eq!(scene.placements.len(), 2);
        assert_eq!(scene.placements[0].rotation_y_degrees, 0.0, "default rotation should be 0");
        assert_eq!(scene.placements[1].rotation_y_degrees, 90.0);
    }
}
