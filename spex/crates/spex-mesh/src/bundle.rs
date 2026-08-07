//! The `spex-brick-mesh` bundle: a small JSON manifest plus tightly packed
//! little-endian binary buffers — the same shape as `tileset.json` +
//! `octree/*.bin`, which this project already knows how to write, serve and
//! parse.
//!
//! ```text
//! <bundle-dir>/
//!   mesh.json                 the manifest
//!   instances.bin             10 bytes per instance
//!   buffers/
//!     p<N>.pos.bin            f32 LE, 3 per vertex (mm, Y-up)
//!     p<N>.nrm.bin            f32 LE, 3 per vertex, unit length
//!     p<N>.idx.bin            u32 LE, 3 per triangle
//!     p<N>.edge.bin           f32 LE, 6 per hard edge
//!     p<N>.cond.bin           f32 LE, 12 per conditional edge (2 endpoints + 2 controls)
//! ```
//!
//! Two decisions here are load-bearing and were both corrected out of the
//! first draft of the spec, so they are spelled out rather than assumed.
//!
//! **Instances are binary, not JSON.** At Atlas scale an `instances[]` array
//! is roughly 37 MB of text, ~120 MB of parsed heap, and about a second of
//! main-thread parse time before the first frame. Ten bytes each is 2.5 MB.
//!
//! **Colour is stored linear.** three.js r152+ reads vertex colours and
//! material colours as linear, so handing it sRGB values straight from
//! `LDConfig.ldr` renders every material roughly 2.2x too bright. The
//! conversion happens here, once, and the manifest says so.

use anyhow::{bail, Context, Result};
use serde::Serialize;
use spex_ldraw::edges::EdgeKind;
use spex_ldraw::{Finish, FullTriangle, LdrawColor, PartGeometry};

use crate::material::PbrMaterial;

/// The mesh path reads the *full* colour table — finish, alpha and luminance
/// included — rather than the point pipeline's name+RGB tuple.
pub type FullColorTable = HashMap<u32, LdrawColor>;
use std::collections::HashMap;
use std::path::Path;

use crate::weld::{weld_and_smooth, WeldedMesh, DEFAULT_CREASE_DEGREES};

/// Bumped to 2 by M56.
///
/// M56 added `finish` and `pbr` to every material entry and made both
/// *required*. That is a breaking change to the format, and the first version
/// of it shipped without saying so — a bundle built before M56 then reached
/// the viewer, passed the `version === 1` check, and died on
/// `entry.pbr.opacity` with a TypeError pointing at minified three.js. A
/// format that changes what readers must find has to change its number, or
/// the check it offers them is worthless.
pub const FORMAT_VERSION: u32 = 2;
/// Instance translations are quantised to whole LDraw units, which is exact
/// for grid-legal geometry and 0.4 mm at worst for anything else. i16 gives
/// +/- 32767 LDU, i.e. +/- 13.1 m of local extent per bundle.
pub const TRANSLATION_UNIT_MM: f64 = spex_ldraw::LDU_TO_MM;

// --------------------------------------------------------------- frame ----

/// **The one place LDraw's coordinate frame becomes spex's** — LDU to
/// millimetres, and Y-down to Y-up.
///
/// Negating Y is a *mirror*. It flips handedness, so on its own it would
/// leave every triangle wound backwards relative to its own outward normal,
/// and a renderer that culls by winding — which is all of them — draws the
/// inside of the far wall instead of the outside of the near one. The object
/// renders as if it were transparent, and nothing about that symptom points
/// at a coordinate conversion. `to_output_triangles` therefore swaps two
/// vertices of every triangle, and `to_output_matrix` conjugates a rotation
/// by the same flip. Found the expensive way by
/// `scripts/mesh-vs-points-spike/`; see `spex_ldraw::bfc`'s module doc.
pub fn to_output_position(p: [f64; 3]) -> [f64; 3] {
    [
        p[0] * spex_ldraw::LDU_TO_MM,
        -p[1] * spex_ldraw::LDU_TO_MM,
        p[2] * spex_ldraw::LDU_TO_MM,
    ]
}

/// `F * M * F` with `F = diag(1, -1, 1)`: the same rotation, expressed in the
/// flipped frame. An axis-aligned matrix stays axis-aligned.
pub fn to_output_matrix(m: &[f64; 9]) -> [f64; 9] {
    let f = [1.0, -1.0, 1.0];
    let mut out = [0.0; 9];
    for r in 0..3 {
        for c in 0..3 {
            out[r * 3 + c] = f[r] * m[r * 3 + c] * f[c];
        }
    }
    out
}

/// Converts a resolved part's triangles into the output frame, **reversing
/// winding** to compensate for the mirror. See `to_output_position`.
pub fn to_output_triangles(geo: &PartGeometry) -> Vec<FullTriangle> {
    geo.triangles
        .iter()
        .map(|t| FullTriangle {
            vertices: [
                to_output_position(t.vertices[0]),
                to_output_position(t.vertices[2]),
                to_output_position(t.vertices[1]),
            ],
            color_code: t.color_code,
            source: t.source,
        })
        .collect()
}

/// sRGB 0..255 to linear 0..1, per the real piecewise sRGB transfer function.
pub fn srgb_to_linear(c: u8) -> f32 {
    let x = c as f32 / 255.0;
    if x <= 0.04045 {
        x / 12.92
    } else {
        ((x + 0.055) / 1.055).powf(2.4)
    }
}

// ------------------------------------------------------------ manifest ----

#[derive(Serialize)]
pub struct Bounds {
    pub min: [f64; 3],
    pub max: [f64; 3],
}

#[derive(Serialize)]
pub struct SubmeshRange {
    /// `None` = LDraw colour 16, "inherit": take the instance's own material.
    /// A number is a fixed accent colour baked into the part itself.
    pub material: Option<usize>,
    #[serde(rename = "indexOffset")]
    pub index_offset: usize,
    #[serde(rename = "indexCount")]
    pub index_count: usize,
}

/// The five buffer paths, as a struct rather than a map — a `HashMap` here
/// serialises its keys in a different order on every run, which silently
/// breaks byte-for-byte reproducibility of the manifest. Determinism is an
/// acceptance criterion for this format and the foundation of M91's frame-hash
/// regression fixture, so nothing in it may iterate a hash map.
/// `skip_serializing_if` for a count that is only meaningful when non-zero.
fn is_zero(n: &usize) -> bool {
    *n == 0
}

#[derive(Serialize)]
pub struct PartBuffers {
    pub position: String,
    pub normal: String,
    pub index: String,
    #[serde(rename = "hardEdge")]
    pub hard_edge: String,
    #[serde(rename = "condEdge")]
    pub cond_edge: String,
    /// M65: the companion surface point cloud — position + normal, 24 bytes
    /// per point, **colour-neutral like the mesh it was sampled from**. Only
    /// on level 0: a point cloud of a simplified brick would be a cloud of a
    /// different object, and the crossfade's whole claim is that the two
    /// representations are the same thing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub points: Option<String>,
}

/// One coarser level of one part. Same shape as the part's own level-0
/// fields, because it is produced by exactly the same packing code.
#[derive(Serialize)]
pub struct LodEntry {
    pub level: u8,
    #[serde(rename = "vertexCount")]
    pub vertex_count: usize,
    #[serde(rename = "triangleCount")]
    pub triangle_count: usize,
    #[serde(rename = "hardEdgeCount")]
    pub hard_edge_count: usize,
    #[serde(rename = "conditionalEdgeCount")]
    pub conditional_edge_count: usize,
    pub buffers: PartBuffers,
    pub submeshes: Vec<SubmeshRange>,
}

#[derive(Serialize)]
pub struct PartEntry {
    pub index: usize,
    #[serde(rename = "partFile")]
    pub part_file: String,
    pub description: Option<String>,
    #[serde(rename = "vertexCount")]
    pub vertex_count: usize,
    #[serde(rename = "triangleCount")]
    pub triangle_count: usize,
    #[serde(rename = "hardEdgeCount")]
    pub hard_edge_count: usize,
    #[serde(rename = "conditionalEdgeCount")]
    pub conditional_edge_count: usize,
    pub bounds: Bounds,
    /// M65: how many points `buffers.points` holds. Absent (0) for a part
    /// with no triangles.
    #[serde(rename = "pointCount", default, skip_serializing_if = "is_zero")]
    pub point_count: usize,
    pub buffers: PartBuffers,
    pub submeshes: Vec<SubmeshRange>,
    /// Real LDraw **reference chains** this part's geometry came from — what
    /// M59's LOD pass gates stud/tube removal on. The chain, not the leaf:
    /// the same primitive file is a stud in one chain and a wall in another.
    pub sources: Vec<String>,
    /// Coarser levels, in ascending order: LOD1 (studs and tubes removed by
    /// reference chain) and LOD2 (the part's box, 12 triangles + 12 edges).
    /// **Level 0 is this entry's own `buffers`/counts**, not a member of this
    /// array — so a reader that ignores `lods` entirely still gets full
    /// geometry, which is why adding them did not need a format version bump.
    pub lods: Vec<LodEntry>,
    pub license: Option<String>,
    pub author: Option<String>,
}

#[derive(Serialize)]
pub struct MaterialEntry {
    #[serde(rename = "colorCode")]
    pub color_code: u32,
    pub name: String,
    /// **Linear**, not sRGB. See this module's header.
    #[serde(rename = "baseColor")]
    pub base_color: [f32; 3],
    /// The colour's own real `EDGE` value from `LDConfig.ldr`, linear.
    #[serde(rename = "edgeColor")]
    pub edge_color: [f32; 3],
    /// Real `LDConfig.ldr` finish keyword: `solid`, `chrome`, `pearlescent`,
    /// `rubber`, `matte_metallic`, `metal`, `speckle`, `glitter`.
    pub finish: &'static str,
    /// Everything a renderer needs to make that finish look like itself.
    /// The numbers are calibrated artistic choices, documented as such in
    /// `material.rs` — the bundle states them so a second renderer resolves
    /// the same brick to the same look without re-deriving the table.
    pub pbr: PbrMaterial,
}

#[derive(Serialize)]
pub struct InstanceEncoding {
    pub stride: usize,
    pub layout: Vec<&'static str>,
    #[serde(rename = "translationUnitMm")]
    pub translation_unit_mm: f64,
    #[serde(rename = "maxTranslationErrorMm")]
    pub max_translation_error_mm: f64,
    pub count: usize,
    pub file: &'static str,
}

#[derive(Serialize)]
pub struct Attribution {
    #[serde(rename = "geometrySource")]
    pub geometry_source: &'static str,
    #[serde(rename = "colorTable")]
    pub color_table: &'static str,
    pub note: &'static str,
}

#[derive(Serialize)]
pub struct Manifest {
    pub version: u32,
    pub generator: String,
    pub unit: &'static str,
    #[serde(rename = "upAxis")]
    pub up_axis: &'static str,
    #[serde(rename = "colorSpace")]
    pub color_space: &'static str,
    #[serde(rename = "creaseDegrees")]
    pub crease_degrees: f64,
    pub bounds: Bounds,
    pub parts: Vec<PartEntry>,
    pub materials: Vec<MaterialEntry>,
    /// Distinct 3x3 orientations, row-major, already in the output frame.
    /// An instance references one by index, which is what makes a 10-byte
    /// instance record possible.
    pub orientations: Vec<[f64; 9]>,
    #[serde(rename = "instanceEncoding")]
    pub instance_encoding: InstanceEncoding,
    /// Stable per-instance ids, in the same order as `instances.bin`. Used to
    /// resolve a show's target globs once, at load time.
    #[serde(rename = "instanceIds")]
    pub instance_ids: Vec<String>,
    /// M64: the real `0 STEP` build stage each instance belongs to, in the
    /// same order as `instanceIds`. Present only for scenes whose source
    /// actually has step markers — which is the point: an assembly staggered
    /// by *real* build order looks like something being built, and one
    /// staggered by array index looks like a list being iterated.
    ///
    /// Optional and additive, so a reader that ignores it still gets a
    /// complete bundle and no version bump is needed. (Contrast M56, which
    /// added *required* fields and did need one.)
    #[serde(default, rename = "instanceBuildSteps", skip_serializing_if = "Option::is_none")]
    pub instance_build_steps: Option<Vec<u32>>,
    pub attribution: Attribution,
}


/// Packs one level of one part: vertex/normal/index/edge buffers written to
/// disk, plus the counts and submesh ranges that describe them.
///
/// M59 made this a function rather than the inside of a loop. Every level —
/// LOD0's real geometry, LOD1 with its studs removed, LOD2's box — goes
/// through exactly the same packing, colour grouping, winding reversal and
/// mirror handling. Anything a level did differently would be a bug, and the
/// only way to be sure of that is for there to be one copy of the code.
struct PackedLevel {
    buffers: PartBuffers,
    submeshes: Vec<SubmeshRange>,
    vertex_count: usize,
    triangle_count: usize,
    hard_edge_count: usize,
    conditional_edge_count: usize,
    min: [f64; 3],
    max: [f64; 3],
    bytes: u64,
    point_count: usize,
}

fn pack_level(
    out_dir: &Path,
    prefix: &str,
    geometry: &PartGeometry,
    welded: &WeldedMesh,
    material_index: &HashMap<u32, usize>,
) -> Result<PackedLevel> {
    let w = welded;
    let mut pos = Vec::with_capacity(w.positions.len() * 12);
    let mut nrm = Vec::with_capacity(w.normals.len() * 12);
    let (mut mn, mut mx) = ([f64::INFINITY; 3], [f64::NEG_INFINITY; 3]);
    for (v, n) in w.positions.iter().zip(w.normals.iter()) {
        for k in 0..3 {
            pos.extend_from_slice(&v[k].to_le_bytes());
            nrm.extend_from_slice(&n[k].to_le_bytes());
            mn[k] = mn[k].min(v[k] as f64);
            mx[k] = mx[k].max(v[k] as f64);
        }
    }
    if !mn[0].is_finite() {
        mn = [0.0; 3];
        mx = [0.0; 3];
    }

    // Triangles are grouped by colour so a submesh is one contiguous index
    // range — a renderer draws each range with one material and never
    // re-binds mid-buffer.
    let mut by_color: Vec<(Option<u32>, Vec<u32>)> = Vec::new();
    for (t, color) in w.triangle_colors.iter().enumerate() {
        let slot = match by_color.iter().position(|(c, _)| c == color) {
            Some(s) => s,
            None => {
                by_color.push((*color, Vec::new()));
                by_color.len() - 1
            }
        };
        by_color[slot].1.extend_from_slice(&w.indices[t * 3..t * 3 + 3]);
    }
    let mut idx = Vec::new();
    let mut submeshes = Vec::new();
    let mut offset = 0usize;
    for (color, ids) in &by_color {
        for id in ids {
            idx.extend_from_slice(&id.to_le_bytes());
        }
        submeshes.push(SubmeshRange {
            material: color.map(|c| material_index.get(&c).copied().unwrap_or(0)),
            index_offset: offset,
            index_count: ids.len(),
        });
        offset += ids.len();
    }

    let mut edge = Vec::new();
    let mut cond = Vec::new();
    let (mut hard_n, mut cond_n) = (0usize, 0usize);
    for e in &geometry.edges {
        let a = to_output_position(e.vertices[0]);
        let b = to_output_position(e.vertices[1]);
        match &e.kind {
            EdgeKind::Hard => {
                for v in [a, b] {
                    for k in 0..3 {
                        edge.extend_from_slice(&(v[k] as f32).to_le_bytes());
                    }
                }
                hard_n += 1;
            }
            EdgeKind::Conditional { control } => {
                let c0 = to_output_position(control[0]);
                let c1 = to_output_position(control[1]);
                for v in [a, b, c0, c1] {
                    for k in 0..3 {
                        cond.extend_from_slice(&(v[k] as f32).to_le_bytes());
                    }
                }
                cond_n += 1;
            }
        }
    }

    // M65: the point cloud, level 0 only. `prefix` carries the level, so this
    // is the one place that has to know the difference.
    let mut point_count = 0usize;
    let mut points_rel: Option<String> = None;
    if !prefix.contains(".l") {
        let sampled = crate::points::sample_surface(w);
        if !sampled.is_empty() {
            let rel = format!("buffers/{prefix}.pts.bin");
            let data = crate::points::to_bytes(&sampled);
            std::fs::write(out_dir.join(&rel), &data).with_context(|| format!("writing {rel}"))?;
            point_count = sampled.len();
            points_rel = Some(rel);
        }
    }

    let mut bytes = 0u64;
    let mut written = Vec::new();
    for (name, data) in [("pos", &pos), ("nrm", &nrm), ("idx", &idx), ("edge", &edge), ("cond", &cond)] {
        let rel = format!("buffers/{prefix}.{name}.bin");
        std::fs::write(out_dir.join(&rel), data).with_context(|| format!("writing {rel}"))?;
        bytes += data.len() as u64;
        written.push(rel);
    }

    Ok(PackedLevel {
        buffers: PartBuffers {
            position: written[0].clone(),
            normal: written[1].clone(),
            index: written[2].clone(),
            hard_edge: written[3].clone(),
            cond_edge: written[4].clone(),
            points: points_rel,
        },
        submeshes,
        point_count,
        vertex_count: w.vertex_count(),
        triangle_count: w.triangle_count(),
        hard_edge_count: hard_n,
        conditional_edge_count: cond_n,
        min: mn,
        max: mx,
        bytes,
    })
}

// ------------------------------------------------------------- builder ----

struct PendingPart {
    part_file: String,
    geometry: PartGeometry,
    welded: WeldedMesh,
}

struct PendingInstance {
    part: usize,
    material: usize,
    translation_ldu: [i16; 3],
    orientation: u8,
    id: String,
}

#[derive(Debug, Default, PartialEq)]
pub struct MeshBundleStats {
    /// M59: what the coarser levels would cost, measured on this bundle's own
    /// parts and gated on the real LDraw reference chain. Reported rather
    /// than written — the bundle still carries LOD0 only, and a format that
    /// advertises levels no reader selects would be a promise, not a feature.
    pub lod1_triangles: usize,
    pub lod2_triangles: usize,
    pub part_count: usize,
    pub instance_count: usize,
    pub material_count: usize,
    pub orientation_count: usize,
    pub total_vertices: usize,
    pub total_triangles: usize,
    pub total_hard_edges: usize,
    pub total_conditional_edges: usize,
    pub bytes_written: u64,
    pub max_translation_error_mm: f64,
}

pub struct MeshBundleBuilder {
    crease_degrees: f64,
    parts: Vec<PendingPart>,
    part_index: HashMap<String, usize>,
    materials: Vec<MaterialEntry>,
    material_index: HashMap<u32, usize>,
    orientations: Vec<[f64; 9]>,
    instances: Vec<PendingInstance>,
    build_steps: Option<Vec<u32>>,
    max_translation_error_mm: f64,
}

impl Default for MeshBundleBuilder {
    fn default() -> Self {
        Self::new(DEFAULT_CREASE_DEGREES)
    }
}

impl MeshBundleBuilder {
    pub fn new(crease_degrees: f64) -> Self {
        MeshBundleBuilder {
            crease_degrees,
            parts: Vec::new(),
            part_index: HashMap::new(),
            materials: Vec::new(),
            material_index: HashMap::new(),
            orientations: Vec::new(),
            instances: Vec::new(),
            build_steps: None,
            max_translation_error_mm: 0.0,
        }
    }

    /// Whether this part is already in the bundle, and where — so a caller
    /// can skip resolving it a second time. Resolution is the expensive half
    /// (car.ldr's 61 placements are 26 distinct parts, each a whole
    /// subpart/primitive tree), and `add_part` would deduplicate anyway, but
    /// only after the work had already been done.
    pub fn part_index(&self, part_file: &str) -> Option<usize> {
        self.part_index.get(part_file).copied()
    }

    /// Adds a distinct real part's geometry exactly once, keyed on the part
    /// file **alone** — deliberately not on (part, colour), because the
    /// geometry carries no colour. Two colours of the same part share one
    /// mesh, which is the whole basis of instancing.
    pub fn add_part(&mut self, part_file: &str, geometry: &PartGeometry) -> usize {
        if let Some(i) = self.part_index.get(part_file) {
            return *i;
        }
        let welded = weld_and_smooth(&to_output_triangles(geometry), self.crease_degrees);
        let i = self.parts.len();
        self.parts.push(PendingPart {
            part_file: part_file.to_string(),
            geometry: geometry.clone(),
            welded,
        });
        self.part_index.insert(part_file.to_string(), i);
        i
    }

    /// Resolves one real LDraw colour code into the bundle's material table.
    ///
    /// Takes the *full* colour table (`load_colors_full`), not the point
    /// pipeline's name+RGB tuple: a brick's finish, alpha and luminance are
    /// as much a part of what it looks like as its RGB, and dropping them was
    /// what made every transparent and chrome part render as flat plastic.
    pub fn add_material(&mut self, colors: &FullColorTable, color_code: u32) -> usize {
        if let Some(i) = self.material_index.get(&color_code) {
            return *i;
        }
        // An unknown code is a real possibility (third-party parts reference
        // codes the official table has never had). A neutral grey solid is a
        // visible, honest stand-in; failing the whole build is not.
        let fallback = LdrawColor {
            code: color_code,
            name: format!("Unknown {color_code}"),
            value: [200, 200, 200],
            edge: [0x59, 0x59, 0x59],
            alpha: 255,
            luminance: 0,
            finish: Finish::Solid,
        };
        let color = colors.get(&color_code).unwrap_or(&fallback);
        let lin = |c: [u8; 3]| [srgb_to_linear(c[0]), srgb_to_linear(c[1]), srgb_to_linear(c[2])];
        let i = self.materials.len();
        self.materials.push(MaterialEntry {
            color_code,
            name: color.name.clone(),
            base_color: lin(color.value),
            edge_color: lin(color.edge),
            finish: color.finish.key(),
            pbr: crate::material::from_ldraw(color),
        });
        self.material_index.insert(color_code, i);
        i
    }

    fn orientation_index(&mut self, matrix: &[f64; 9]) -> Result<u8> {
        let m = to_output_matrix(matrix);
        for (i, o) in self.orientations.iter().enumerate() {
            if o.iter().zip(m.iter()).all(|(a, b)| (a - b).abs() < 1e-9) {
                return Ok(i as u8);
            }
        }
        if self.orientations.len() >= 256 {
            bail!(
                "more than 256 distinct orientations in one bundle — the 10-byte instance \
                 record indexes them with a u8. Split the scene, or add the float-matrix \
                 fallback stream (deliberately not built yet: no real scene has needed it)."
            );
        }
        self.orientations.push(m);
        Ok((self.orientations.len() - 1) as u8)
    }

    /// Adds one placement. `translation`/`matrix` are in LDraw's own frame,
    /// exactly as `spex_ldraw::Placement` carries them; the conversion to the
    /// output frame happens here.
    /// The real `0 STEP` stage of each instance, in `add_instance` order.
    /// Ignored — and omitted from the manifest — when every value is the
    /// same, because a scene with no step markers has nothing to say here and
    /// a column of zeroes is not information.
    pub fn set_build_steps(&mut self, steps: Vec<u32>) {
        let distinct = steps.iter().collect::<std::collections::BTreeSet<_>>().len();
        self.build_steps = if distinct > 1 { Some(steps) } else { None };
    }

    pub fn add_instance(
        &mut self,
        part: usize,
        material: usize,
        translation: [f64; 3],
        matrix: &[f64; 9],
        id: impl Into<String>,
    ) -> Result<()> {
        let orientation = self.orientation_index(matrix)?;
        let p = to_output_position(translation);
        let mut q = [0i16; 3];
        for k in 0..3 {
            let in_units = p[k] / TRANSLATION_UNIT_MM;
            if !(-32768.0..=32767.0).contains(&in_units) {
                bail!(
                    "instance {:?} sits {:.1} mm from the bundle origin, outside the \
                     +/-13.1 m an i16 LDU translation can address",
                    id.into(),
                    p[k]
                );
            }
            let r = in_units.round();
            let err = ((in_units - r).abs()) * TRANSLATION_UNIT_MM;
            let err = if err < 1e-9 { 0.0 } else { err };
            if err > self.max_translation_error_mm {
                self.max_translation_error_mm = err;
            }
            q[k] = r as i16;
        }
        self.instances.push(PendingInstance {
            part,
            material,
            translation_ldu: q,
            orientation,
            id: id.into(),
        });
        Ok(())
    }

    pub fn write(self, out_dir: &Path) -> Result<MeshBundleStats> {
        std::fs::create_dir_all(out_dir.join("buffers"))
            .with_context(|| format!("creating {}", out_dir.display()))?;
        let mut stats = MeshBundleStats {
            part_count: self.parts.len(),
            instance_count: self.instances.len(),
            material_count: self.materials.len(),
            orientation_count: self.orientations.len(),
            max_translation_error_mm: self.max_translation_error_mm,
            ..Default::default()
        };
        let mut bytes = 0u64;
        let mut parts = Vec::new();
        let mut part_local_bounds: Vec<([f64; 3], [f64; 3])> = Vec::new();
        let mut global = ([f64::INFINITY; 3], [f64::NEG_INFINITY; 3]);

        for (i, p) in self.parts.iter().enumerate() {
            // LOD0 is the part as resolved; LOD1 drops studs and tubes by
            // real reference chain; LOD2 is its box. All three are packed by
            // the same function, so none of them can quietly differ.
            let lod0 = pack_level(out_dir, &format!("p{i}"), &p.geometry, &p.welded, &self.material_index)?;
            bytes += lod0.bytes;

            let mut lods = Vec::new();
            for (level, geo) in [(1u8, crate::lod::lod1(&p.geometry)), (2u8, crate::lod::lod2(&p.geometry))] {
                let welded = weld_and_smooth(&to_output_triangles(&geo), self.crease_degrees);
                let packed = pack_level(out_dir, &format!("p{i}.l{level}"), &geo, &welded, &self.material_index)?;
                bytes += packed.bytes;
                if level == 1 {
                    stats.lod1_triangles += packed.triangle_count;
                } else {
                    stats.lod2_triangles += packed.triangle_count;
                }
                lods.push(LodEntry {
                    level,
                    vertex_count: packed.vertex_count,
                    triangle_count: packed.triangle_count,
                    hard_edge_count: packed.hard_edge_count,
                    conditional_edge_count: packed.conditional_edge_count,
                    buffers: packed.buffers,
                    submeshes: packed.submeshes,
                });
            }

            let (mn, mx) = (lod0.min, lod0.max);
            let buffers = lod0.buffers;
            let submeshes = lod0.submeshes;
            let (hard_n, cond_n) = (lod0.hard_edge_count, lod0.conditional_edge_count);
            let vertex_count = lod0.vertex_count;
            let point_count = lod0.point_count;
            let triangle_count = lod0.triangle_count;

            stats.total_vertices += vertex_count;
            stats.total_triangles += triangle_count;
            stats.total_hard_edges += hard_n;
            stats.total_conditional_edges += cond_n;


            part_local_bounds.push(([mn[0], mn[1], mn[2]], [mx[0], mx[1], mx[2]]));
            parts.push(PartEntry {
                index: i,
                part_file: p.part_file.clone(),
                description: p.geometry.description.clone(),
                vertex_count,
                triangle_count,
                hard_edge_count: hard_n,
                conditional_edge_count: cond_n,
                bounds: Bounds { min: mn, max: mx },
                point_count,
                buffers,
                submeshes,
                sources: p.geometry.sources.clone(),
                lods,
                license: p.geometry.license.clone(),
                author: p.geometry.author.clone(),
            });
        }

        let mut inst = Vec::with_capacity(self.instances.len() * 10);
        let mut ids = Vec::with_capacity(self.instances.len());
        for it in &self.instances {
            for k in 0..3 {
                inst.extend_from_slice(&it.translation_ldu[k].to_le_bytes());
            }
            inst.push(it.orientation);
            inst.push(u8::try_from(it.material).unwrap_or(0));
            inst.extend_from_slice(&u16::try_from(it.part).unwrap_or(0).to_le_bytes());
            ids.push(it.id.clone());
        }
        std::fs::write(out_dir.join("instances.bin"), &inst)?;
        bytes += inst.len() as u64;

        // Scene bounds are the ASSEMBLED extent — each part's local box put
        // where its instances actually place it — not the union of parts sitting
        // at the origin. A viewer frames its camera from this, and a nine-brick
        // stack whose bounds were one brick tall would frame the wrong object.
        for it in &self.instances {
            let (mn, mx) = part_local_bounds[it.part];
            let m = &self.orientations[it.orientation as usize];
            for cx in [mn[0], mx[0]] {
                for cy in [mn[1], mx[1]] {
                    for cz in [mn[2], mx[2]] {
                        for k in 0..3 {
                            let v = m[k * 3] * cx + m[k * 3 + 1] * cy + m[k * 3 + 2] * cz
                                + it.translation_ldu[k] as f64 * TRANSLATION_UNIT_MM;
                            global.0[k] = global.0[k].min(v);
                            global.1[k] = global.1[k].max(v);
                        }
                    }
                }
            }
        }
        if self.instances.is_empty() {
            global = ([0.0; 3], [0.0; 3]);
        }
        let manifest = Manifest {
            version: FORMAT_VERSION,
            generator: format!("spex-mesh {}", env!("CARGO_PKG_VERSION")),
            unit: "mm",
            up_axis: "+Y",
            color_space: "linear",
            crease_degrees: self.crease_degrees,
            bounds: Bounds { min: global.0, max: global.1 },
            parts,
            materials: self.materials,
            orientations: self.orientations,
            instance_encoding: InstanceEncoding {
                stride: 10,
                layout: vec!["i16 x", "i16 y", "i16 z", "u8 orientation", "u8 material", "u16 part"],
                translation_unit_mm: TRANSLATION_UNIT_MM,
                max_translation_error_mm: self.max_translation_error_mm,
                count: self.instances.len(),
                file: "instances.bin",
            },
            instance_ids: ids,
            instance_build_steps: self.build_steps.clone(),
            attribution: Attribution {
                geometry_source: "LDraw Parts Library (ldraw.org), CCAL 2.0",
                color_table: "LDConfig.ldr",
                note: "see docs/fugen/licensing.md",
            },
        };
        let json = serde_json::to_vec_pretty(&manifest)?;
        std::fs::write(out_dir.join("mesh.json"), &json)?;
        bytes += json.len() as u64;
        stats.bytes_written = bytes;
        Ok(stats)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use spex_ldraw::edges::Edge;
    use spex_ldraw::LdrawCache;

    fn synthetic_cache(files: &[(&str, &str)]) -> (tempfile::TempDir, LdrawCache) {
        let dir = tempfile::tempdir().unwrap();
        for (path, body) in files {
            let full = dir.path().join(path);
            std::fs::create_dir_all(full.parent().unwrap()).unwrap();
            std::fs::write(full, body).unwrap();
        }
        let cache = LdrawCache::new(dir.path());
        (dir, cache)
    }

    fn color(code: u32, name: &str, value: [u8; 3], alpha: u8, finish: Finish) -> LdrawColor {
        LdrawColor {
            code,
            name: name.into(),
            value,
            edge: [0x59, 0x59, 0x59],
            alpha,
            luminance: 0,
            finish,
        }
    }

    fn colors() -> FullColorTable {
        // Real codes, real values, real finishes, out of LDConfig.ldr.
        let mut c = FullColorTable::new();
        c.insert(0, color(0, "Black", [0x1B, 0x2A, 0x34], 255, Finish::Solid));
        c.insert(4, color(4, "Red", [0xC9, 0x1A, 0x09], 255, Finish::Solid));
        c.insert(47, color(47, "Trans_Clear", [0xFC, 0xFC, 0xFC], 128, Finish::Solid));
        c.insert(383, color(383, "Chrome_Silver", [0xCE, 0xCE, 0xCE], 255, Finish::Chrome));
        c
    }

    #[test]
    fn a_materials_finish_and_pbr_travel_with_it_into_the_manifest() {
        let mut b = MeshBundleBuilder::new(DEFAULT_CREASE_DEGREES);
        let table = colors();
        let chrome = b.add_material(&table, 383);
        let trans = b.add_material(&table, 47);
        let black = b.add_material(&table, 0);
        assert_eq!(b.materials[chrome].finish, "chrome");
        assert_eq!(b.materials[chrome].pbr.metalness, 1.0);
        assert_eq!(b.materials[trans].finish, "solid");
        assert!(b.materials[trans].pbr.transmission > 0.0, "Trans_Clear must transmit");
        assert!((b.materials[trans].pbr.opacity - 128.0 / 255.0).abs() < 1e-6);
        assert_eq!(b.materials[black].pbr.transmission, 0.0);
        // The real EDGE value, not the hardcoded default the pre-M56 writer used.
        assert!((b.materials[black].edge_color[0] - srgb_to_linear(0x59)).abs() < 1e-6);
    }

    #[test]
    fn an_unknown_colour_code_is_a_visible_grey_solid_not_a_build_failure() {
        let mut b = MeshBundleBuilder::new(DEFAULT_CREASE_DEGREES);
        let i = b.add_material(&colors(), 9999);
        assert_eq!(b.materials[i].name, "Unknown 9999");
        assert_eq!(b.materials[i].finish, "solid");
    }

    #[test]
    fn positions_convert_ldu_to_mm_and_flip_y() {
        assert_eq!(to_output_position([10.0, 20.0, 30.0]), [4.0, -8.0, 12.0]);
    }

    #[test]
    fn the_mirror_is_compensated_so_outward_stays_outward() {
        // A face whose LDraw normal points along +Y (which is *down* in
        // LDraw) must come out pointing along -Y in a Y-up frame. If the
        // winding swap were missing, it would come out along +Y — the face
        // would be inside-out, and backface culling would show the interior.
        let geo = PartGeometry {
            triangles: vec![FullTriangle {
                vertices: [[0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]],
                color_code: None,
                source: 0,
            }],
            ..Default::default()
        };
        let n_in = spex_ldraw::full_triangle_normal(&geo.triangles[0]);
        assert!(n_in[1] > 0.9, "fixture sanity: LDraw normal is +Y, got {n_in:?}");
        let out = to_output_triangles(&geo);
        let n_out = spex_ldraw::full_triangle_normal(&out[0]);
        assert!(n_out[1] < -0.9, "must flip to -Y, got {n_out:?}");
    }

    #[test]
    fn a_matrix_conjugated_by_the_flip_stays_a_rotation() {
        let m = spex_ldraw::rotation_y(0.7);
        let out = to_output_matrix(&m);
        assert!(
            (spex_ldraw::determinant3(&out) - 1.0).abs() < 1e-12,
            "conjugation preserves determinant +1, got {}",
            spex_ldraw::determinant3(&out)
        );
        assert!((to_output_matrix(&spex_ldraw::IDENTITY)[4] - 1.0).abs() < 1e-12);
    }

    #[test]
    fn srgb_converts_to_linear_and_darkens_the_midtones() {
        assert!(srgb_to_linear(0) < 1e-9);
        assert!((srgb_to_linear(255) - 1.0).abs() < 1e-6);
        assert!(srgb_to_linear(128) < 0.25, "the whole point: 0.5 sRGB is ~0.22 linear");
    }

    #[test]
    fn a_part_is_stored_once_regardless_of_how_many_colours_use_it() {
        let (_d, cache) = synthetic_cache(&[(
            "parts/t.dat",
            "0 T\n0 BFC CERTIFY CCW\n3 16 0 0 0 1 0 0 0 1 0\n",
        )]);
        let geo = spex_ldraw::resolve_part_full(&cache, "t.dat").unwrap();
        let mut b = MeshBundleBuilder::default();
        let a = b.add_part("t.dat", &geo);
        let c = b.add_part("t.dat", &geo);
        assert_eq!(a, c, "geometry carries no colour, so one mesh serves every colour");
        assert_eq!(b.parts.len(), 1);
    }

    #[test]
    fn identical_orientations_are_deduplicated_into_the_table() {
        let (_d, cache) = synthetic_cache(&[(
            "parts/t.dat",
            "0 T\n0 BFC CERTIFY CCW\n3 16 0 0 0 1 0 0 0 1 0\n",
        )]);
        let geo = spex_ldraw::resolve_part_full(&cache, "t.dat").unwrap();
        let mut b = MeshBundleBuilder::default();
        let p = b.add_part("t.dat", &geo);
        let m = b.add_material(&colors(), 0);
        for i in 0..5 {
            b.add_instance(p, m, [0.0, -24.0 * i as f64, 0.0], &spex_ldraw::IDENTITY, format!("i{i}"))
                .unwrap();
        }
        assert_eq!(b.orientations.len(), 1, "one shared orientation for five instances");
    }

    #[test]
    fn instance_translations_quantise_exactly_on_the_ldraw_grid() {
        let (_d, cache) = synthetic_cache(&[(
            "parts/t.dat",
            "0 T\n0 BFC CERTIFY CCW\n3 16 0 0 0 1 0 0 0 1 0\n",
        )]);
        let geo = spex_ldraw::resolve_part_full(&cache, "t.dat").unwrap();
        let mut b = MeshBundleBuilder::default();
        let p = b.add_part("t.dat", &geo);
        let m = b.add_material(&colors(), 0);
        // The real monolith's own stacking: whole LDU, so exact.
        b.add_instance(p, m, [0.0, -184.0, 0.0], &spex_ldraw::IDENTITY, "exact").unwrap();
        assert_eq!(b.max_translation_error_mm, 0.0);
        assert_eq!(b.instances[0].translation_ldu, [0, 184, 0], "and Y is flipped");
        // Something off-grid records its real error rather than hiding it.
        b.add_instance(p, m, [0.5, 0.0, 0.0], &spex_ldraw::IDENTITY, "off-grid").unwrap();
        assert!((b.max_translation_error_mm - 0.2).abs() < 1e-9, "0.5 LDU rounds to 0.2 mm");
    }

    #[test]
    fn buffer_sizes_match_the_counts_the_manifest_claims() {
        let (_d, cache) = synthetic_cache(&[(
            "parts/t.dat",
            "0 T\n0 BFC CERTIFY CCW\n\
             4 16 0 0 0 1 0 0 1 1 0 0 1 0\n\
             2 24 0 0 0 1 0 0\n\
             5 24 0 0 0 1 0 0 0 1 0 0 -1 0\n",
        )]);
        let geo = spex_ldraw::resolve_part_full(&cache, "t.dat").unwrap();
        let mut b = MeshBundleBuilder::default();
        let p = b.add_part("t.dat", &geo);
        let m = b.add_material(&colors(), 4);
        b.add_instance(p, m, [0.0; 3], &spex_ldraw::IDENTITY, "only").unwrap();

        let out = tempfile::tempdir().unwrap();
        let stats = b.write(out.path()).unwrap();
        assert_eq!(stats.part_count, 1);
        assert_eq!(stats.instance_count, 1);
        assert_eq!(stats.total_triangles, 2, "a quad is two triangles");
        assert_eq!(stats.total_hard_edges, 1);
        assert_eq!(stats.total_conditional_edges, 1);

        let json: serde_json::Value =
            serde_json::from_slice(&std::fs::read(out.path().join("mesh.json")).unwrap()).unwrap();
        let part = &json["parts"][0];
        let size = |k: &str| std::fs::metadata(out.path().join(part["buffers"][k].as_str().unwrap()))
            .unwrap()
            .len() as usize;
        let vc = part["vertexCount"].as_u64().unwrap() as usize;
        let tc = part["triangleCount"].as_u64().unwrap() as usize;
        assert_eq!(size("position"), vc * 12, "3 f32 per vertex");
        assert_eq!(size("normal"), vc * 12);
        assert_eq!(size("index"), tc * 12, "3 u32 per triangle");
        assert_eq!(size("hardEdge"), 1 * 24, "6 f32 per hard edge");
        assert_eq!(size("condEdge"), 1 * 48, "12 f32 per conditional edge");
        assert_eq!(
            std::fs::metadata(out.path().join("instances.bin")).unwrap().len(),
            10,
            "ten bytes per instance — the whole reason instances are not JSON"
        );
        assert_eq!(json["colorSpace"], "linear");
        assert_eq!(json["unit"], "mm");
        assert_eq!(json["upAxis"], "+Y");
    }

    #[test]
    fn welding_a_quad_beats_the_naive_three_vertices_per_triangle() {
        let (_d, cache) = synthetic_cache(&[(
            "parts/q.dat",
            "0 Q\n0 BFC CERTIFY CCW\n4 16 0 0 0 1 0 0 1 1 0 0 1 0\n",
        )]);
        let geo = spex_ldraw::resolve_part_full(&cache, "q.dat").unwrap();
        let mut b = MeshBundleBuilder::default();
        b.add_part("q.dat", &geo);
        assert_eq!(b.parts[0].welded.vertex_count(), 4, "not 6");
    }

    #[test]
    fn an_inherited_colour_submesh_defers_to_the_instance() {
        let (_d, cache) = synthetic_cache(&[(
            "parts/m.dat",
            "0 M\n0 BFC CERTIFY CCW\n3 16 0 0 0 1 0 0 0 1 0\n3 0 0 0 0 1 0 0 0 1 0\n",
        )]);
        let geo = spex_ldraw::resolve_part_full(&cache, "m.dat").unwrap();
        let mut b = MeshBundleBuilder::default();
        b.add_material(&colors(), 0);
        b.add_part("m.dat", &geo);
        let out = tempfile::tempdir().unwrap();
        b.write(out.path()).unwrap();
        let json: serde_json::Value =
            serde_json::from_slice(&std::fs::read(out.path().join("mesh.json")).unwrap()).unwrap();
        let sub = &json["parts"][0]["submeshes"];
        assert_eq!(sub.as_array().unwrap().len(), 2);
        assert!(sub[0]["material"].is_null(), "code 16 defers to the instance");
        assert!(sub[1]["material"].is_number(), "a fixed accent colour does not");
    }

    #[test]
    fn edge_endpoints_land_in_the_output_frame() {
        let (_d, cache) = synthetic_cache(&[(
            "parts/e.dat",
            "0 E\n0 BFC CERTIFY CCW\n2 24 0 20 0 0 40 0\n",
        )]);
        let geo = spex_ldraw::resolve_part_full(&cache, "e.dat").unwrap();
        assert_eq!(geo.edges.len(), 1);
        let a = to_output_position(geo.edges[0].vertices[0]);
        assert_eq!(a, [0.0, -8.0, 0.0], "20 LDU down becomes 8 mm down");
        let _ = Edge { ..geo.edges[0].clone() };
    }
}
