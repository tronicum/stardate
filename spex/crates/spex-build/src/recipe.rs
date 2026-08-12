//! The recipe JSON format (`spec/recipe.schema.json`) and the pipeline from
//! a recipe file to a real emitted `.ldr`: parse -> dispatch each step's
//! named primitive -> arrange `count` instances (optionally along an arc or
//! circle) -> collect real placements -> `validate()` -> serialise with
//! provenance (acceptance criterion 5).
use crate::grid::{self, FootprintTable, GridPos, Illegality, Orientation, Placement};
use crate::primitives::{Arch, Bond, Colonnade, Column, Dome, Mosaic, PartSet, Primitive, Pyramid, Stair, Trilithon, Wall, Ziggurat};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Recipe {
    pub version: u32,
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub scale: Option<Scale>,
    #[serde(default)]
    pub palette: HashMap<String, u32>,
    pub steps: Vec<Step>,
    /// Real, justified exceptions to `validate()`'s findings (acceptance
    /// criterion 2). Each entry names the placement(s) it excuses and why.
    #[serde(default, rename = "knownIllegal")]
    pub known_illegal: Vec<KnownIllegal>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Scale {
    #[serde(rename = "studsPerMetre")]
    pub studs_per_metre: f64,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Step {
    pub primitive: String,
    #[serde(default = "default_count")]
    pub count: u32,
    #[serde(default, rename = "arrangeOn")]
    pub arrange_on: Option<ArrangeOn>,
    /// A fixed single placement, for a step with no `arrangeOn` (default
    /// origin/orientation when absent). Not in `phase4-kit.md`'s own
    /// minimal example — that recipe's two steps both arrange around one
    /// shared circle center — but a real recipe combining differently
    /// positioned primitives (the common case from M74 on) needs a way to
    /// say where a single, non-arranged instance goes. Additive: every
    /// recipe that omits it keeps behaving exactly as documented.
    #[serde(default)]
    pub at: Option<At>,
    #[serde(default)]
    pub params: Value,
}

fn default_count() -> u32 {
    1
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, Default)]
pub struct At {
    #[serde(default, rename = "xStuds")]
    pub x_studs: f64,
    #[serde(default, rename = "zStuds")]
    pub z_studs: f64,
    #[serde(default, rename = "yPlates")]
    pub y_plates: i32,
    #[serde(default, rename = "orientationDeg")]
    pub orientation_deg: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum ArrangeOn {
    /// `count` instances along a real circular arc, evenly spaced from
    /// `startDeg` to `endDeg` inclusive (matching the recipe example in
    /// `docs/fugen/phase4-kit.md`'s M72 section — Stonehenge's trilithons).
    Arc {
        #[serde(rename = "radiusStuds")]
        radius_studs: f64,
        #[serde(rename = "startDeg")]
        start_deg: f64,
        #[serde(rename = "endDeg")]
        end_deg: f64,
    },
    /// `count` instances evenly spaced around a full real circle.
    Circle {
        #[serde(rename = "radiusStuds")]
        radius_studs: f64,
    },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct KnownIllegal {
    #[serde(rename = "placementIndex")]
    pub placement_index: usize,
    pub reason: String,
}

fn get_u32(v: &Value, key: &str) -> Result<u32> {
    v.get(key)
        .and_then(|x| x.as_u64())
        .map(|x| x as u32)
        .with_context(|| format!("params.{key} missing or not a non-negative integer"))
}

fn get_u32_or(v: &Value, key: &str, default: u32) -> u32 {
    v.get(key).and_then(|x| x.as_u64()).map(|x| x as u32).unwrap_or(default)
}

fn get_bool_or(v: &Value, key: &str, default: bool) -> bool {
    v.get(key).and_then(|x| x.as_bool()).unwrap_or(default)
}

/// A real LDraw color code, given directly as a number or by a name looked
/// up in the recipe's own `palette`.
fn get_color(v: &Value, key: &str, palette: &HashMap<String, u32>) -> Result<u32> {
    let raw = v.get(key).with_context(|| format!("params.{key} missing"))?;
    if let Some(n) = raw.as_u64() {
        return Ok(n as u32);
    }
    if let Some(name) = raw.as_str() {
        return palette
            .get(name)
            .copied()
            .with_context(|| format!("palette has no entry {name:?} needed by params.{key}"));
    }
    bail!("params.{key} must be a real LDraw color code (integer) or a palette name (string)")
}

fn parse_bond(v: &Value) -> Result<Bond> {
    match v.get("bond").and_then(|x| x.as_str()) {
        Some("Running") | None => Ok(Bond::Running),
        Some("Stack") => Ok(Bond::Stack),
        Some("EnglishCross") => Ok(Bond::EnglishCross),
        Some(other) => bail!("params.bond {other:?} is not a real bond ('Running', 'Stack', 'EnglishCross')"),
    }
}

/// Dispatches one step's `primitive` name + JSON `params` into the real
/// primitive it names. The only place that connects the recipe format's
/// string vocabulary to `primitives.rs`'s real Rust types.
fn build_primitive(name: &str, params: &Value, palette: &HashMap<String, u32>) -> Result<Box<dyn Primitive>> {
    let color = |key: &str| get_color(params, key, palette);
    Ok(match name {
        "Wall" => Box::new(Wall {
            width_studs: get_u32(params, "widthStuds")?,
            height_plates: get_u32(params, "heightPlates")?,
            depth_studs: get_u32_or(params, "depthStuds", 1),
            bond: parse_bond(params)?,
            color: color("color")?,
            part_set: PartSet::Classic,
        }),
        "Column" => Box::new(Column {
            height_plates: get_u32(params, "heightPlates")?,
            diameter_studs: get_u32(params, "diameterStuds")?,
            color: color("color")?,
        }),
        "Arch" => Box::new(Arch {
            span_studs: get_u32(params, "spanStuds")?,
            rise_plates: get_u32(params, "risePlates")?,
            thickness_studs: get_u32_or(params, "thicknessStuds", 1),
            color: color("color")?,
        }),
        "Stair" => Box::new(Stair {
            run_studs: get_u32(params, "runStuds")?,
            rise_plates: get_u32(params, "risePlates")?,
            width_studs: get_u32(params, "widthStuds")?,
            color: color("color")?,
        }),
        "Ziggurat" => Box::new(Ziggurat {
            base_studs: get_u32(params, "baseStuds")?,
            tiers: get_u32(params, "tiers")?,
            tier_height_plates: get_u32(params, "tierHeightPlates")?,
            setback_studs: get_u32(params, "setbackStuds")?,
            color: color("color")?,
        }),
        "Pyramid" => Box::new(Pyramid {
            base_studs: get_u32(params, "baseStuds")?,
            color: color("color")?,
            stepped: get_bool_or(params, "stepped", true),
        }),
        "Dome" => Box::new(Dome {
            radius_studs: get_u32(params, "radiusStuds")?,
            height_plates: params.get("heightPlates").and_then(|v| v.as_u64()).map(|v| v as u32),
            color: color("color")?,
        }),
        "Trilithon" => Box::new(Trilithon {
            post_height_plates: get_u32(params, "postHeightPlates")?,
            gap_studs: get_u32(params, "gapStuds")?,
            color: color("color")?,
        }),
        "Colonnade" => {
            let column_params = params.get("column").with_context(|| "params.column missing (Colonnade needs a nested Column spec)")?;
            Box::new(Colonnade {
                columns: get_u32(params, "columns")?,
                spacing_studs: get_u32(params, "spacingStuds")?,
                column: Column {
                    height_plates: get_u32(column_params, "heightPlates")?,
                    diameter_studs: get_u32(column_params, "diameterStuds")?,
                    color: get_color(column_params, "color", palette)?,
                },
                architrave: get_bool_or(params, "architrave", false),
            })
        }
        "Mosaic" => {
            let raw_cells = params.get("cells").with_context(|| "params.cells missing (Mosaic needs a 2D array of colors)")?;
            let rows = raw_cells.as_array().with_context(|| "params.cells must be an array of arrays")?;
            let mut cells = Vec::with_capacity(rows.len());
            for row in rows {
                let row_arr = row.as_array().with_context(|| "params.cells rows must each be an array")?;
                let mut resolved = Vec::with_capacity(row_arr.len());
                for cell in row_arr {
                    resolved.push(resolve_cell_color(cell, palette)?);
                }
                cells.push(resolved);
            }
            let tile_part = params
                .get("tilePart")
                .and_then(|x| x.as_str())
                .with_context(|| "params.tilePart missing")?
                .to_string();
            Box::new(Mosaic { cells, tile_part })
        }
        other => bail!("{other:?} is not a real spex-build primitive"),
    })
}

/// `null` is a real, empty cell — a hole in the mosaic. Anything else must
/// resolve to a real colour, so a typo in a palette name is still an error
/// rather than a quietly missing brick.
fn resolve_cell_color(v: &Value, palette: &HashMap<String, u32>) -> Result<Option<u32>> {
    if v.is_null() {
        return Ok(None);
    }
    if let Some(n) = v.as_u64() {
        return Ok(Some(n as u32));
    }
    if let Some(name) = v.as_str() {
        return palette
            .get(name)
            .copied()
            .map(Some)
            .with_context(|| format!("palette has no entry {name:?} needed by a mosaic cell"));
    }
    bail!("a mosaic cell must be a real LDraw color code (integer), a palette name (string), or null for an empty cell")
}

fn snap_half_studs(studs: f64) -> i32 {
    (studs * 2.0).round() as i32
}

/// The origin/orientation pairs `count` instances of one step land at.
/// `None` (no `arrangeOn`) with `count == 1` is the common case: one
/// instance at the recipe's own origin. `count > 1` with no `arrangeOn` is
/// almost always an authoring mistake (every instance would land in the
/// same place) and is rejected rather than silently overlapped.
fn step_origins(step: &Step) -> Result<Vec<(GridPos, Orientation)>> {
    let count = step.count.max(1);
    match &step.arrange_on {
        None => {
            if count > 1 {
                bail!("step {:?} has count={count} but no arrangeOn — every instance would land on the same placement", step.primitive);
            }
            let origin = match step.at {
                Some(at) => GridPos::new(snap_half_studs(at.x_studs), at.y_plates, snap_half_studs(at.z_studs)),
                None => GridPos::new(0, 0, 0),
            };
            let orientation = step.at.map(|at| Orientation::nearest_yaw(at.orientation_deg)).unwrap_or(Orientation::IDENTITY);
            Ok(vec![(origin, orientation)])
        }
        // An arrangement is RELATIVE TO `at`, and it did not used to be: the
        // two were exclusive, so a step with both had its `at` silently
        // dropped and the ring landed at the origin at ground level. M74's
        // Colosseum found it — three arcades authored at 0 m, 14 m and 28 m
        // came out stacked in the same place, 6 240 overlaps out of 6 240
        // placements. "A ring of columns at this point and this height" is
        // what an author means by writing both, and a silently ignored field
        // is worse than a rejected one.
        Some(ArrangeOn::Circle { radius_studs }) => Ok((0..count)
            .map(|i| {
                let deg = 360.0 * i as f64 / count as f64;
                offset_by_at(arc_point(*radius_studs, deg), step)
            })
            .collect()),
        Some(ArrangeOn::Arc { radius_studs, start_deg, end_deg }) => Ok((0..count)
            .map(|i| {
                let t = if count > 1 { i as f64 / (count as f64 - 1.0) } else { 0.0 };
                let deg = start_deg + (end_deg - start_deg) * t;
                offset_by_at(arc_point(*radius_studs, deg), step)
            })
            .collect()),
    }
}

/// Shifts an arranged instance by the step's own `at`.
///
/// The orientation the arrangement chose is kept: `arrangeOn` snaps each
/// instance to face outward along its radius, which is the whole reason to use
/// it, and `at.orientationDeg` on an arranged step would be asking for two
/// different things at once. It is ignored, and that is stated here rather
/// than left to be discovered.
fn offset_by_at(point: (GridPos, Orientation), step: &Step) -> (GridPos, Orientation) {
    let (pos, orientation) = point;
    let Some(at) = step.at else { return (pos, orientation) };
    (
        GridPos::new(
            pos.x + snap_half_studs(at.x_studs),
            pos.y + at.y_plates,
            pos.z + snap_half_studs(at.z_studs),
        ),
        orientation,
    )
}

/// A point on a real circle of `radius_studs`, snapped to the grid-legal
/// half-stud lattice, with the orientation snapped to whichever of the
/// real 24 orientations has the closest yaw to facing outward along the
/// radius. Snapping is the documented, honest cost of "count instances
/// around an arc" landing on a *legal* grid rather than a continuous one —
/// a recipe that genuinely needs continuous rotation is off-grid by
/// construction (`Placement::declared_off_grid`), not this helper's job.
///
/// **A placement's origin is its own front-bottom-left corner, not its
/// center** — every primitive in `primitives.rs` builds outward from
/// `(0,0,0)` local, and `at`/`arrangeOn` place that same local `(0,0,0)`.
/// A ring of columns is therefore each offset from the ideal circle by
/// about half its own footprint (radially and tangentially) — visible at
/// small radius, negligible at real Atlas scale. A "center-anchored"
/// variant is a real, deferred polish item for M74, not attempted here.
fn arc_point(radius_studs: f64, deg: f64) -> (GridPos, Orientation) {
    let rad = deg.to_radians();
    let x = radius_studs * rad.cos();
    let z = radius_studs * rad.sin();
    (GridPos::new(snap_half_studs(x), 0, snap_half_studs(z)), Orientation::nearest_yaw(deg))
}

pub fn build_recipe(recipe: &Recipe) -> Result<Vec<Placement>> {
    let mut all = Vec::new();
    // Real build-stage numbering: each emitted instance's own internal
    // `build_step` (e.g. `Ziggurat` numbers its own tiers from 0) is offset
    // by how many stages every earlier instance in this recipe already
    // used, so a `count`+`arrangeOn` step (e.g. 5 real Trilithons) gets
    // real, distinct, staggerable stages across all of them — the same
    // real grain `ldraw-scenes/stonehenge.ldr` uses by hand (one stage per
    // real structural instance). Placement order is untouched by this —
    // `validate()`'s `Illegality`/`recipe.known_illegal` indices depend on
    // placements staying in emission order.
    let mut next_step: u32 = 0;
    for step in &recipe.steps {
        let origins = step_origins(step).with_context(|| format!("step {:?}", step.primitive))?;
        for (origin, orientation) in origins {
            let prim = build_primitive(&step.primitive, &step.params, &recipe.palette).with_context(|| format!("step {:?}", step.primitive))?;
            let mut emitted = prim.emit(origin, orientation);
            let local_max = emitted.iter().map(|p| p.build_step).max().unwrap_or(0);
            for p in &mut emitted {
                p.build_step += next_step;
            }
            next_step += local_max + 1;
            all.extend(emitted);
        }
    }
    Ok(all)
}

/// FNV-1a 64-bit (Fowler/Noll/Vo, 1991) — a real, standard, public
/// non-cryptographic hash, not invented for this project. Deterministic
/// content fingerprint over the recipe's own raw bytes, the same
/// provenance discipline `spex-brick-mesh`'s resolve-once cache already
/// established (`BRICKs.md`).
pub fn content_hash(bytes: &[u8]) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn fmt_ldu(v: f64) -> String {
    let s = format!("{v:.6}");
    let trimmed = s.trim_end_matches('0').trim_end_matches('.');
    if trimmed.is_empty() || trimmed == "-" {
        "0".to_string()
    } else {
        trimmed.to_string()
    }
}

/// Serialises real placements as a real `.ldr`, with the provenance
/// acceptance criterion 5 requires: a `0 Author:` line naming this as
/// machine-generated by `spex-build` from a named recipe, and a `0 !SPEX`
/// comment carrying the recipe's own content hash.
pub fn write_ldr(recipe: &Recipe, recipe_hash: &str, placements: &[Placement]) -> String {
    let mut out = String::new();
    out.push_str(&format!("0 {}\n", recipe.title));
    out.push_str(&format!("0 Name: {}.ldr\n", recipe.id));
    out.push_str(&format!("0 Author: machine-generated by spex-build from recipe {:?}\n", recipe.id));
    out.push_str(&format!("0 !SPEX recipe-hash {recipe_hash}\n"));
    out.push_str(&format!("0 !SPEX recipe-version {}\n", recipe.version));
    // Real `0 STEP` lines, one per real build-stage boundary — the exact
    // token `spex_ldraw::scene::parse_scene` already increments its own
    // `Placement.build_step` counter on, so this round-trips into the
    // already-shipped `instanceBuildSteps`/`AssemblyChoreography` pipeline
    // with zero changes outside this crate. `placements` is in emission
    // order with non-decreasing `build_step` by construction
    // (`build_recipe`'s own offsetting never reorders), but a `while`
    // rather than a single `if` keeps this correct even if that numbering
    // is ever non-contiguous, not just when it increments by exactly 1.
    let mut current_step: u32 = 0;
    for p in placements {
        while current_step < p.build_step {
            out.push_str("0 STEP\n");
            current_step += 1;
        }
        let [x, y, z] = p.translation_ldu;
        let m = p.matrix;
        out.push_str(&format!(
            "1 {} {} {} {} {} {} {} {} {} {} {} {} {} {}\n",
            p.color,
            fmt_ldu(x),
            fmt_ldu(y),
            fmt_ldu(z),
            fmt_ldu(m[0]),
            fmt_ldu(m[1]),
            fmt_ldu(m[2]),
            fmt_ldu(m[3]),
            fmt_ldu(m[4]),
            fmt_ldu(m[5]),
            fmt_ldu(m[6]),
            fmt_ldu(m[7]),
            fmt_ldu(m[8]),
            p.part
        ));
    }
    out
}

/// A real problem `validate()` found, split into what a recipe has
/// explicitly declared and justified (acceptance criterion 2's
/// `"knownIllegal"`) versus what it has not.
pub struct BuildOutput {
    pub recipe: Recipe,
    pub recipe_hash: String,
    pub placements: Vec<Placement>,
    pub declared: Vec<(Illegality, String)>,
    pub undeclared: Vec<Illegality>,
    pub ldr_text: String,
}

fn illegality_placement_indices(problem: &Illegality) -> Vec<usize> {
    match *problem {
        Illegality::OffGridTranslation { placement_index, .. } => vec![placement_index],
        Illegality::NonAxisRotation { placement_index } => vec![placement_index],
        Illegality::Overlap { a, b, .. } => vec![a, b],
        Illegality::Floating { placement_index } => vec![placement_index],
    }
}

/// The whole M72 pipeline: read a recipe file, build it, validate it,
/// serialise it with provenance. What `spex build` calls.
pub fn build(recipe_path: &Path) -> Result<BuildOutput> {
    let bytes = std::fs::read(recipe_path).with_context(|| format!("reading recipe {}", recipe_path.display()))?;
    let recipe: Recipe = serde_json::from_slice(&bytes).with_context(|| format!("parsing recipe {} as JSON", recipe_path.display()))?;
    if recipe.version != 1 {
        bail!("recipe {} declares version {}, only version 1 is understood", recipe_path.display(), recipe.version);
    }
    let recipe_hash = content_hash(&bytes);
    let placements = build_recipe(&recipe)?;
    let problems = grid::validate(&placements, &FootprintTable::standard());

    let mut declared = Vec::new();
    let mut undeclared = Vec::new();
    for problem in problems {
        let indices = illegality_placement_indices(&problem);
        let excuse = recipe.known_illegal.iter().find(|k| indices.contains(&k.placement_index));
        match excuse {
            Some(k) => declared.push((problem, k.reason.clone())),
            None => undeclared.push(problem),
        }
    }

    let ldr_text = write_ldr(&recipe, &recipe_hash, &placements);
    Ok(BuildOutput { recipe, recipe_hash, placements, declared, undeclared, ldr_text })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wall_recipe_json() -> &'static str {
        r#"{
            "version": 1,
            "id": "test-wall",
            "title": "Test Wall",
            "palette": { "stone": 71 },
            "steps": [
                { "primitive": "Wall", "params": { "widthStuds": 8, "heightPlates": 3, "bond": "Stack", "color": "stone" } }
            ]
        }"#
    }

    #[test]
    fn parses_a_real_recipe_and_builds_real_placements() {
        let recipe: Recipe = serde_json::from_str(wall_recipe_json()).unwrap();
        let placements = build_recipe(&recipe).unwrap();
        assert_eq!(placements.len(), 2, "8 studs, stack bond -> real greedy tiling, 2x 1x4");
        assert!(placements.iter().all(|p| p.color == 71));
    }

    /// `at` and `arrangeOn` compose. They did not: a step with both had its
    /// `at` silently dropped, so three arcades authored at three heights came
    /// out in the same place — 6 240 overlaps out of 6 240 placements, and no
    /// error anywhere.
    #[test]
    fn an_arrangement_is_relative_to_the_step_s_own_at() {
        let with_at: Recipe = serde_json::from_str(
            r#"{
                "version": 1, "id": "ring", "title": "Ring",
                "palette": { "stone": 71 },
                "steps": [
                    { "primitive": "Column", "count": 4, "at": { "xStuds": 100, "yPlates": -30, "zStuds": 200 },
                      "arrangeOn": { "kind": "circle", "radiusStuds": 10 },
                      "params": { "heightPlates": 3, "diameterStuds": 1, "color": "stone" } }
                ]
            }"#,
        )
        .unwrap();
        let moved = build_recipe(&with_at).unwrap();
        assert_eq!(moved.len(), 4);
        // 100 studs = 2000 LDU, 200 studs = 4000 LDU, -30 plates = -240 LDU.
        for p in &moved {
            assert!(p.translation_ldu[0] > 1700.0 && p.translation_ldu[0] < 2300.0, "x {:?}", p.translation_ldu);
            assert!(p.translation_ldu[2] > 3700.0 && p.translation_ldu[2] < 4300.0, "z {:?}", p.translation_ldu);
        }
        let lowest = moved.iter().map(|p| p.translation_ldu[1]).fold(f64::MAX, f64::min);
        assert!(lowest <= -240.0, "the ring did not rise with at.yPlates: {lowest}");
    }

    #[test]
    fn content_hash_is_deterministic_and_sensitive_to_real_bytes() {
        let h1 = content_hash(wall_recipe_json().as_bytes());
        let h2 = content_hash(wall_recipe_json().as_bytes());
        assert_eq!(h1, h2);
        let h3 = content_hash(b"different bytes");
        assert_ne!(h1, h3);
        assert_eq!(h1.len(), 16, "16 real hex digits for a 64-bit FNV-1a hash");
    }

    #[test]
    fn write_ldr_carries_real_provenance() {
        let recipe: Recipe = serde_json::from_str(wall_recipe_json()).unwrap();
        let placements = build_recipe(&recipe).unwrap();
        let text = write_ldr(&recipe, "deadbeef01234567", &placements);
        assert!(text.contains("0 Author: machine-generated by spex-build from recipe \"test-wall\""));
        assert!(text.contains("0 !SPEX recipe-hash deadbeef01234567"));
        assert_eq!(text.lines().filter(|l| l.starts_with("1 ")).count(), 2);
    }

    #[test]
    fn count_greater_than_one_without_arrange_on_is_rejected() {
        let mut recipe: Recipe = serde_json::from_str(wall_recipe_json()).unwrap();
        recipe.steps[0].count = 3;
        let err = build_recipe(&recipe).unwrap_err();
        // anyhow's Display only shows the outermost context; the real
        // message is further down the chain.
        let full = err.chain().map(|c| c.to_string()).collect::<Vec<_>>().join(" / ");
        assert!(full.contains("no arrangeOn"), "{full}");
    }

    #[test]
    fn arrange_on_circle_places_count_instances_at_the_real_snapped_angles() {
        let recipe: Recipe = serde_json::from_str(
            r#"{
                "version": 1, "id": "ring", "title": "Ring",
                "palette": { "stone": 71 },
                "steps": [
                    { "primitive": "Column", "count": 4, "arrangeOn": { "kind": "circle", "radiusStuds": 10 },
                      "params": { "heightPlates": 3, "diameterStuds": 1, "color": "stone" } }
                ]
            }"#,
        )
        .unwrap();
        let placements = build_recipe(&recipe).unwrap();
        assert_eq!(placements.len(), 4, "4 columns, 1 course each");
        // 4 columns around a circle at 0/90/180/270 degrees, radius 10
        // studs (200 LDU), each offset by its own +10 LDU corner-to-center
        // local offset (a 1-stud column's local center — see arc_point's
        // doc comment on the corner-anchored convention).
        let origins: Vec<(i32, i32)> = placements.iter().map(|p| (p.translation_ldu[0].round() as i32, p.translation_ldu[2].round() as i32)).collect();
        assert_eq!(origins[0], (210, 10), "0 degrees: radius on +X (200) plus the column's own +10 local center, identity orientation");
        // At 90 degrees the instance is also *rotated* (arrangeOn snaps
        // each instance's orientation to face outward), so its local
        // (+10,+10) center offset is itself rotated before being added —
        // it does not simply repeat the 0-degree case on the other axis.
        assert_eq!(origins[1], (10, 190), "90 degrees: radius on +Z (200), rotated local center subtracts on this axis");
        // Each of the 4 real Column instances gets its own real build
        // stage — `Column` itself doesn't stage its own courses, so each
        // instance's local build_step stays 0, and build_recipe's own
        // cross-instance offsetting is what gives them 0/1/2/3 here, not
        // any change inside the primitive.
        let steps: Vec<u32> = placements.iter().map(|p| p.build_step).collect();
        assert_eq!(steps, vec![0, 1, 2, 3], "one real build stage per column instance, offset by build_recipe");
    }

    #[test]
    fn write_ldr_emits_one_real_step_line_per_build_stage_boundary() {
        let recipe: Recipe = serde_json::from_str(
            r#"{
                "version": 1, "id": "ring", "title": "Ring",
                "palette": { "stone": 71 },
                "steps": [
                    { "primitive": "Column", "count": 3, "arrangeOn": { "kind": "circle", "radiusStuds": 10 },
                      "params": { "heightPlates": 3, "diameterStuds": 1, "color": "stone" } }
                ]
            }"#,
        )
        .unwrap();
        let placements = build_recipe(&recipe).unwrap();
        let text = write_ldr(&recipe, "deadbeef01234567", &placements);
        // 3 real build stages (one per column) means exactly 2 real
        // boundaries between them — a STEP line before the last stage
        // starts and one before the middle one, none before the first.
        assert_eq!(text.matches("0 STEP\n").count(), 2);
        // The STEP lines land between real "1 " placement lines, not
        // before the very first one (nothing to stage before that).
        let first_step = text.find("0 STEP\n").unwrap();
        let first_placement = text.find("\n1 ").unwrap();
        assert!(first_placement < first_step, "at least one real placement line before the first STEP");
    }

    #[test]
    fn at_places_a_single_instance_at_a_real_fixed_offset() {
        let recipe: Recipe = serde_json::from_str(
            r#"{
                "version": 1, "id": "offset", "title": "Offset",
                "palette": { "stone": 71 },
                "steps": [
                    { "primitive": "Column", "params": { "heightPlates": 3, "diameterStuds": 1, "color": "stone" },
                      "at": { "xStuds": 5, "zStuds": 2 } }
                ]
            }"#,
        )
        .unwrap();
        let placements = build_recipe(&recipe).unwrap();
        // "at" places the primitive's own local (0,0,0) corner at 5/2
        // studs (100/40 LDU); a 1-stud column's own local center is a
        // further +10 LDU on each axis (see arc_point's doc comment).
        assert_eq!(placements[0].translation_ldu[0].round() as i32, 110, "5 studs = 100 LDU + 10 LDU local center");
        assert_eq!(placements[0].translation_ldu[2].round() as i32, 50, "2 studs = 40 LDU + 10 LDU local center");
    }

    #[test]
    fn build_reports_a_known_illegal_exception_separately_from_a_real_undeclared_one() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("two-overlapping-columns.json");
        // Two identical, unarranged Column steps land at the exact same
        // origin by construction (both default to (0,0,0)) — a real,
        // deliberate overlap to exercise the declared/undeclared split.
        std::fs::write(
            &path,
            r#"{
                "version": 1, "id": "overlap-test", "title": "Overlap Test",
                "palette": { "stone": 71 },
                "steps": [
                    { "primitive": "Column", "params": { "heightPlates": 3, "diameterStuds": 1, "color": "stone" } },
                    { "primitive": "Column", "params": { "heightPlates": 3, "diameterStuds": 1, "color": "stone" } }
                ],
                "knownIllegal": [ { "placementIndex": 1, "reason": "test fixture: deliberately duplicated placement" } ]
            }"#,
        )
        .unwrap();

        let output = build(&path).unwrap();
        assert_eq!(output.declared.len(), 1, "the overlap involving placement 1 is declared and excused");
        assert!(output.undeclared.is_empty(), "no other real problem should be left over: {:?}", output.undeclared);
    }
}
