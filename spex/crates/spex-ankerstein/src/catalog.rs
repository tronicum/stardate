//! Real, hand-authored Ankerstein shape catalog — no network fetch (unlike
//! `spex-ldraw::cache`), because no equivalent open parts library exists
//! for Ankerstein. Every entry's `source_citation` is mandatory: the
//! concrete mechanism (not just a doc convention) that keeps this catalog
//! honest to the "real data only" rule `docs/agents/working-mode.md`
//! holds every adapter in this project to. See `docs/ANKERSTEIN-ENGINE.md`
//! §2 for the research pass that produced the seed entries below, and §1
//! for why AnkerCAD's/AnkerPlan's own stone definitions are not a source
//! here.
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Which historical grid a shape belongs to — GK and KK are separate,
/// non-interchangeable scales (25mm vs. 20mm base cube), never mixed in
/// one real Ankerstein set.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Caliber {
    /// Großes Kaliber — 25mm base cube. The better-documented, larger
    /// caliber; the default for this crate's scenes (see
    /// `docs/ANKERSTEIN-ENGINE.md` §6).
    Gk,
    /// Kleines Kaliber — 20mm base cube.
    Kk,
}

/// Real shape families, matching the historical set-supplement vocabulary
/// (see `docs/ANKERSTEIN-ENGINE.md` §2/§3) rather than an invented taxonomy.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShapeType {
    /// A plain rectangular block (a full cube is the degenerate 1x1x1 case).
    Block,
    /// A sloped/triangular block, from the historical 5th-set supplement.
    Prism,
    /// One voussoir of a Roman or Gothic arch — see M101 in
    /// `docs/ANKERSTEIN-ENGINE.md`; not yet seeded with real dimensions.
    ArchVoussoir,
}

/// One real catalog entry. `dimensions_mm` is always `[width, height, depth]`
/// in the shape's own local frame, matching `spex_ldraw::geometry::Triangle`'s
/// coordinate convention (Y up after `to_point_cloud`'s flip) so a
/// generated shape composes with `spex-ldraw`'s reused sampling/shading
/// functions without an extra conversion step.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AnkersteinShape {
    pub id: String,
    pub shape_type: ShapeType,
    pub dimensions_mm: [f64; 3],
    pub caliber: Caliber,
    /// Mandatory: where this shape's real dimensions were sourced from.
    /// Never leave this as a placeholder — an unsourced entry does not
    /// belong in this catalog (see this module's own doc comment).
    pub source_citation: String,
}

/// The real seed catalog described in `docs/ANKERSTEIN-ENGINE.md` §2 — four
/// shapes, each independently cross-checked across the research pass
/// (ankerstein.ch, Fred Hartjes' summary page, the CVA Book PDF). Not the
/// full historical catalog (1000+ shapes existed) — grows milestone by
/// milestone, each new entry citing its own real source, per M98's own
/// acceptance criteria.
pub fn seed_shapes() -> Vec<AnkersteinShape> {
    vec![
        AnkersteinShape {
            id: "gk-cube-full".to_string(),
            shape_type: ShapeType::Block,
            dimensions_mm: [25.0, 25.0, 25.0],
            caliber: Caliber::Gk,
            source_citation: "Fred Hartjes, fredhartjes-home.nl/Anker.html — GK base cube unit".to_string(),
        },
        AnkersteinShape {
            id: "gk-half-height".to_string(),
            shape_type: ShapeType::Block,
            dimensions_mm: [25.0, 12.5, 25.0],
            caliber: Caliber::Gk,
            source_citation: "Fred Hartjes, fredhartjes-home.nl/Anker.html — \"a typical Anker stone of 25 x 25 x 12.5 mm\"".to_string(),
        },
        AnkersteinShape {
            id: "kk-cube-full".to_string(),
            shape_type: ShapeType::Block,
            dimensions_mm: [20.0, 20.0, 20.0],
            caliber: Caliber::Kk,
            source_citation: "ankerstein.ch / Grokipedia summary of the CVA Book — KK base cube unit".to_string(),
        },
        AnkersteinShape {
            id: "gk-brick-1x2x4".to_string(),
            shape_type: ShapeType::Block,
            dimensions_mm: [25.0, 50.0, 100.0],
            caliber: Caliber::Gk,
            source_citation: "Grokipedia summary of the CVA Book — 4th historical set's \"1 by 2 by 4 inch\" brick-shaped block, converted to the GK 25mm unit".to_string(),
        },
    ]
}

/// Loads the catalog from a JSON file on disk (see
/// `data/ankerstein-shapes.json` at the repo root) — the real, editable
/// source of truth once M98 lands; falls back to nothing here, callers
/// should use `seed_shapes()` directly until that file exists.
pub fn load_catalog(path: &std::path::Path) -> Result<Vec<AnkersteinShape>> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("reading Ankerstein catalog at {}", path.display()))?;
    let shapes: Vec<AnkersteinShape> =
        serde_json::from_str(&text).with_context(|| format!("parsing Ankerstein catalog at {}", path.display()))?;
    Ok(shapes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_seed_shape_has_a_non_empty_citation() {
        for shape in seed_shapes() {
            assert!(
                !shape.source_citation.trim().is_empty(),
                "shape {:?} is missing a real source citation",
                shape.id
            );
        }
    }

    #[test]
    fn seed_shapes_do_not_mix_calibers_within_a_single_entry() {
        // Each entry declares one caliber; this test exists as the seam
        // M106 (caliber correctness pass) should extend once the catalog
        // has enough entries for cross-entry consistency checks to be
        // meaningful (e.g. no KK-caliber id accidentally reusing a GK
        // dimension verbatim).
        for shape in seed_shapes() {
            match shape.caliber {
                Caliber::Gk => assert!(shape.id.starts_with("gk-"), "{} should be gk-prefixed", shape.id),
                Caliber::Kk => assert!(shape.id.starts_with("kk-"), "{} should be kk-prefixed", shape.id),
            }
        }
    }

    #[test]
    fn round_trips_through_json() {
        let shapes = seed_shapes();
        let json = serde_json::to_string(&shapes).unwrap();
        let parsed: Vec<AnkersteinShape> = serde_json::from_str(&json).unwrap();
        assert_eq!(shapes, parsed);
    }
}
