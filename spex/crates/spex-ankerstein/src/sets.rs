//! Real Ankerstein set/inventory catalog — which stones, and how many of
//! each, make up a given historical or modern set (e.g. "Nr. 5A", "GK 2").
//! Distinct from `catalog.rs` (the stone/part library itself,
//! set-independent) and from `scene.rs` (one specific build) — mirrors
//! LDraw/Rebrickable's own part-vs-set-vs-model split (see `BRICKs.md`'s
//! "Set number" entry). Formal schema: `spec/ankerstein-sets.schema.json`.
//!
//! **Deliberately empty for now** (`data/ankerstein-sets.json` is `[]`).
//! Scaffolded ahead of any real set data, at the user's own request, so
//! the shape exists before it's forgotten — not filled with a placeholder
//! entry, since a fabricated-looking "example" set would violate this
//! project's "real data only" rule (`docs/agents/working-mode.md`) just as
//! much as a fabricated shape would. The first real entry should come from
//! an actual CVA catalog record, a manufacturer's page, or a real
//! Bauanleitung/box in hand — see `docs/ANKERSTEIN-ENGINE.md`.
use crate::catalog::Caliber;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// One (shape, quantity) pair within a set's real contents.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SetContent {
    pub shape_id: String,
    pub quantity: u32,
}

/// One real historical or modern Ankerstein set/inventory record.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AnkersteinSet {
    pub set_id: String,
    pub display_name: String,
    pub caliber: Caliber,
    #[serde(default)]
    pub is_supplement_set: Option<bool>,
    pub contents: Vec<SetContent>,
    /// Mandatory, same rule as `AnkersteinShape::source_citation`.
    pub source_citation: String,
}

/// Loads the set catalog from a JSON file (see
/// `data/ankerstein-sets.json`). An empty array is a valid, expected
/// result right now — not an error — since the catalog is intentionally
/// unseeded (see this module's own doc comment).
pub fn load_sets(path: &std::path::Path) -> Result<Vec<AnkersteinSet>> {
    let text = std::fs::read_to_string(path).with_context(|| format!("reading Ankerstein set catalog at {}", path.display()))?;
    let sets: Vec<AnkersteinSet> =
        serde_json::from_str(&text).with_context(|| format!("parsing Ankerstein set catalog at {}", path.display()))?;
    Ok(sets)
}

/// Checks that every `shapeId` referenced by a set's contents actually
/// exists in the given shape catalog — the cross-file consistency check
/// the two schemas' own doc comments note isn't (and shouldn't be)
/// enforced by JSON Schema alone (no cross-file `$ref`, per this
/// project's self-contained-schema convention).
pub fn validate_against_catalog(sets: &[AnkersteinSet], shapes: &[crate::catalog::AnkersteinShape]) -> Result<()> {
    let known_ids: std::collections::HashSet<&str> = shapes.iter().map(|s| s.id.as_str()).collect();
    for set in sets {
        for content in &set.contents {
            if !known_ids.contains(content.shape_id.as_str()) {
                anyhow::bail!(
                    "set {:?} references unknown shape id {:?} (not in the shape catalog)",
                    set.set_id,
                    content.shape_id
                );
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{Caliber as ShapeCaliber, ShapeType};

    #[test]
    fn loads_an_empty_catalog_without_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sets.json");
        std::fs::write(&path, "[]").unwrap();
        let sets = load_sets(&path).unwrap();
        assert!(sets.is_empty());
    }

    #[test]
    fn round_trips_a_real_shaped_set_through_json() {
        let set = AnkersteinSet {
            set_id: "test-5A".to_string(),
            display_name: "Test Ergänzungskasten 5A".to_string(),
            caliber: Caliber::Gk,
            is_supplement_set: Some(true),
            contents: vec![SetContent { shape_id: "gk-cube-full".to_string(), quantity: 10 }],
            source_citation: "test fixture, not a real set record".to_string(),
        };
        let json = serde_json::to_string(&vec![set.clone()]).unwrap();
        let parsed: Vec<AnkersteinSet> = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed[0], set);
    }

    #[test]
    fn validate_against_catalog_catches_an_unknown_shape_id() {
        let shape = crate::catalog::AnkersteinShape {
            id: "gk-cube-full".to_string(),
            shape_type: ShapeType::Block,
            dimensions_mm: [25.0, 25.0, 25.0],
            caliber: ShapeCaliber::Gk,
            source_citation: "test fixture".to_string(),
        };
        let set = AnkersteinSet {
            set_id: "test-set".to_string(),
            display_name: "Test Set".to_string(),
            caliber: Caliber::Gk,
            is_supplement_set: None,
            contents: vec![SetContent { shape_id: "does-not-exist".to_string(), quantity: 1 }],
            source_citation: "test fixture".to_string(),
        };
        let result = validate_against_catalog(&[set], &[shape]);
        assert!(result.is_err(), "should reject a set referencing an unknown shape id");
    }
}
