//! Every real parametric primitive `spex-build` ships: `Wall`, `Column`,
//! `Arch`, `Stair`, `Ziggurat`, `Pyramid`, `Dome`, `Trilithon`, `Colonnade`,
//! `Mosaic`. Each returns real `Placement`s, nothing else — no rendering,
//! no I/O.
//!
//! **A shared building block.** LDraw's Y axis points down (see
//! `ldraw-scenes/monolith.ldr`'s own comment); this module's convention is
//! that world/local Y=0 is the ground plane and a course `c` counting up
//! from the ground (0-based) has its *bottom* at `-((c+1) * BRICK_PLATES)`
//! plates — i.e. structures grow toward negative Y, matching the one real
//! hand-authored `.ldr` this repo already has. Every primitive below
//! builds its own placements in **local** space (as if it were its own
//! world, origin at its own front-bottom-left corner) and then composes
//! them with the caller's `origin`/`orientation` via `transform_local`,
//! which is the only place translation/rotation composition happens — so
//! primitives can also legally embed other primitives (`Trilithon` embeds
//! two `Column`s, `Colonnade` embeds N `Column`s and a `Wall`) just by
//! calling `.emit()` with a shifted `origin`.
//!
//! **The real part set.** Every primitive here builds from `PartSet::Classic`
//! — 1x1 (`3005.dat`), 1x2 (`3004.dat`), 1x4 (`3010.dat`) and 1x6
//! (`3009.dat`) real bricks,
//! the only parts this milestone has measured real footprints for (see
//! `grid::FootprintTable::standard`'s doc comment for how). That constrains
//! every primitive to whole-brick (3-plate) courses for now — plate-
//! granularity coursing needs a real 1-stud plate part's footprint measured
//! and cited the same way, left for a later milestone.
//!
//! **Honest approximations, stated once here rather than scattered:**
//! - `Column`'s `diameter_studs` produces a **square** footprint, not a
//!   circle — a real round LDraw part's footprint is not yet in this
//!   milestone's cited table. Matches this milestone doc's own precedent
//!   for `Dome` ("corbelled approximation, documented as such").
//! - `Arch` is a real, historically-attested **post-and-lintel** span (the
//!   same technique `Trilithon` uses), not a voussoir/corbel arch closing
//!   to a point — a smaller, buildable, honestly-scoped first cut.
//! - `Pyramid { stepped: false }` is a real smooth-sided pyramid — real
//!   45-degree slope parts (`SLOPE_STRAIGHT`/`SLOPE_CORNER_PART`) replace
//!   each tier's exposed ledge instead of leaving it a flat step. Uses a
//!   real 2-stud setback (not `stepped: true`'s 1) because the real
//!   outside-corner slope this kit cites is itself a 2-stud part — see
//!   `smooth_ring`'s doc comment. `Dome`, which shares `Ziggurat`'s
//!   corbelling, does not get this treatment (a real round dome's smooth
//!   surface is a different, curved-part problem, not a square-pyramid
//!   one) — still corbelled/stepped regardless of any future `Dome` API.
//! - `Dome` is a **solid** corbelled square stack (see its own doc comment
//!   for why a hollow-ring first attempt was rejected: it was not a real
//!   self-supporting structure), not a hollow circular corbelled vault.

use crate::grid::{mat_mul, mat_vec, GridPos, Orientation, Placement, BRICK_PLATES, HALF_STUD_LDU, PLATE_LDU, STUD_LDU};

/// The kit's current working real part set. See the module doc comment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PartSet {
    Classic,
}

impl PartSet {
    /// Real bricks, longest first, for greedy tiling.
    fn bricks(self) -> &'static [(&'static str, u32)] {
        match self {
            PartSet::Classic => &[("3009.dat", 6), ("3010.dat", 4), ("3004.dat", 2), ("3005.dat", 1)],
        }
    }
}

/// Greedily tiles a run of `len_studs` studs, largest real brick first.
/// Exact for any `len_studs` because `PartSet::Classic` always has a
/// 1-stud brick to finish the remainder.
fn tile_length(parts: PartSet, len_studs: u32) -> Vec<(&'static str, u32)> {
    let mut remaining = len_studs;
    let mut out = Vec::new();
    for (part, w) in parts.bricks() {
        while remaining >= *w {
            out.push((*part, *w));
            remaining -= *w;
        }
    }
    debug_assert_eq!(remaining, 0, "PartSet::Classic's 1-stud brick must always finish the remainder");
    out
}

/// A plain run of real 1x2 stretchers, capped with a single 1x1 if
/// `width_studs` is odd. The "full", non-offset running-bond course.
fn stretcher_course(width_studs: u32) -> Vec<(&'static str, u32)> {
    let pairs = width_studs / 2;
    let mut out = vec![("3004.dat", 2); pairs as usize];
    if width_studs % 2 == 1 {
        out.push(("3005.dat", 1));
    }
    out
}

/// The real running-bond "half-brick offset" course: a single 1x1 at each
/// end (the half-brick), 1x2 stretchers in between. Hand-verifiable: for
/// `width_studs=20` this is exactly `1x1, 9x(1x2), 1x1` = 11 real bricks
/// covering `1 + 18 + 1 = 20` studs (acceptance criterion 3's own test
/// case — see `tests` below for the explicit expected translations).
fn offset_stretcher_course(width_studs: u32) -> Vec<(&'static str, u32)> {
    if width_studs == 0 {
        return Vec::new();
    }
    if width_studs == 1 {
        return vec![("3005.dat", 1)];
    }
    let mut out = vec![("3005.dat", 1)];
    let mut remaining = width_studs - 1;
    while remaining >= 2 {
        out.push(("3004.dat", 2));
        remaining -= 2;
    }
    if remaining == 1 {
        out.push(("3005.dat", 1));
    }
    out
}

/// A header course: real 1x1 bricks end to end across the full width.
/// `PartSet::Classic` has no distinct "header face" geometry (a true
/// header shows a rotated brick's short end; a 1x1 brick's footprint is
/// square either way) — documented placeholder until a header-shaped real
/// part's footprint is measured and cited.
fn header_course(width_studs: u32) -> Vec<(&'static str, u32)> {
    vec![("3005.dat", 1); width_studs as usize]
}

/// How many whole-brick (3-plate) courses fit in `total_plates`. This
/// milestone only builds whole-brick coursing (see module doc comment).
fn brick_courses(total_plates: u32) -> u32 {
    total_plates / BRICK_PLATES
}

/// Studs to half-studs, for `GridPos.x`/`GridPos.z`.
fn studs(n: u32) -> i32 {
    (n * 2) as i32
}

/// The bottom of course `c` (0-based from the ground), in plates, using
/// this module's "grows toward negative Y" convention.
fn course_bottom_plates(course: u32) -> i32 {
    -(((course + 1) * BRICK_PLATES) as i32)
}

/// Rounds an LDU value to the nearest half-stud grid index. Every call
/// site constructs an exact multiple arithmetically; `.round()` only
/// guards against float accumulation, never hides a real off-grid value.
fn half_studs(ldu: f64) -> i32 {
    (ldu / HALF_STUD_LDU).round() as i32
}

/// Composes a primitive's own local placements (built as if local space
/// were the whole world) with the caller's real grid origin/orientation —
/// the one place every primitive's translation/rotation composition
/// happens. `p_world = orientation.matrix() * p_local + origin`, matrices
/// compose the same way, matching LDraw's own `p' = M*p + T` convention.
fn transform_local(local: Vec<Placement>, origin: GridPos, orientation: Orientation) -> Vec<Placement> {
    let om = orientation.matrix();
    let ot = origin.to_ldu();
    local
        .into_iter()
        .map(|p| {
            let rotated_t = mat_vec(om, p.translation_ldu);
            Placement {
                translation_ldu: [rotated_t[0] + ot[0], rotated_t[1] + ot[1], rotated_t[2] + ot[2]],
                matrix: mat_mul(om, p.matrix),
                ..p
            }
        })
        .collect()
}

/// Every primitive emits real placements at a given grid origin/orientation,
/// and reports its own bounding footprint (studs W x studs D x plates H)
/// without emitting, so recipes can lay primitives out relative to each
/// other before generating geometry.
pub trait Primitive {
    fn emit(&self, origin: GridPos, orientation: Orientation) -> Vec<Placement>;
    fn extent(&self) -> (u32, u32, u32);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bond {
    Running,
    Stack,
    EnglishCross,
}

pub struct Wall {
    pub width_studs: u32,
    pub height_plates: u32,
    pub depth_studs: u32,
    pub bond: Bond,
    pub color: u32,
    pub part_set: PartSet,
}

impl Wall {
    fn course_segments(&self, course: u32) -> Vec<(&'static str, u32)> {
        match self.bond {
            // Stack has no offset to keep consistent between courses (every
            // course is identical), so unlike Running/EnglishCross below it
            // has no reason to stay within stretcher_course's real-1x2-only
            // set — real greedy tiling (1x4 first) gives a real build fewer,
            // bigger parts instead of a wall of uniform 1x2s. This is what
            // every Ziggurat/Pyramid/Dome tier actually builds from.
            Bond::Stack => tile_length(self.part_set, self.width_studs),
            Bond::Running => {
                if course % 2 == 0 {
                    stretcher_course(self.width_studs)
                } else {
                    offset_stretcher_course(self.width_studs)
                }
            }
            Bond::EnglishCross => {
                if course % 2 == 0 {
                    stretcher_course(self.width_studs)
                } else {
                    header_course(self.width_studs)
                }
            }
        }
    }
}

impl Primitive for Wall {
    fn emit(&self, origin: GridPos, orientation: Orientation) -> Vec<Placement> {
        let courses = brick_courses(self.height_plates);
        let depth = self.depth_studs.max(1);
        let mut local = Vec::new();
        for course in 0..courses {
            let y_plates = course_bottom_plates(course);
            let segments = self.course_segments(course);
            for wythe in 0..depth {
                let z_center = (wythe as f64 + 0.5) * STUD_LDU;
                let mut cursor_studs = 0u32;
                for (part, w) in &segments {
                    let x_center = (cursor_studs as f64 + *w as f64 / 2.0) * STUD_LDU;
                    // One real build stage per course — the real
                    // bricklaying stage (lay one course across the wall's
                    // full width and depth, then the next).
                    let mut p = Placement::on_grid(
                        GridPos::new(half_studs(x_center), y_plates, half_studs(z_center)),
                        Orientation::IDENTITY,
                        *part,
                        self.color,
                    );
                    p.build_step = course;
                    local.push(p);
                    cursor_studs += w;
                }
            }
        }
        transform_local(local, origin, orientation)
    }

    fn extent(&self) -> (u32, u32, u32) {
        (self.width_studs, self.depth_studs.max(1), self.height_plates)
    }
}

pub struct Column {
    pub height_plates: u32,
    pub diameter_studs: u32,
    pub color: u32,
}

impl Primitive for Column {
    fn emit(&self, origin: GridPos, orientation: Orientation) -> Vec<Placement> {
        let diameter = self.diameter_studs.max(1);
        let courses = brick_courses(self.height_plates);
        let mut local = Vec::new();
        for course in 0..courses {
            let y_plates = course_bottom_plates(course);
            for row in 0..diameter {
                let z_center = (row as f64 + 0.5) * STUD_LDU;
                let mut cursor_studs = 0u32;
                for (part, w) in tile_length(PartSet::Classic, diameter) {
                    let x_center = (cursor_studs as f64 + w as f64 / 2.0) * STUD_LDU;
                    local.push(Placement::on_grid(
                        GridPos::new(half_studs(x_center), y_plates, half_studs(z_center)),
                        Orientation::IDENTITY,
                        part,
                        self.color,
                    ));
                    cursor_studs += w;
                }
            }
        }
        transform_local(local, origin, orientation)
    }

    fn extent(&self) -> (u32, u32, u32) {
        let d = self.diameter_studs.max(1);
        (d, d, self.height_plates)
    }
}

pub struct Arch {
    pub span_studs: u32,
    pub rise_plates: u32,
    pub thickness_studs: u32,
    pub color: u32,
}

impl Primitive for Arch {
    fn emit(&self, origin: GridPos, orientation: Orientation) -> Vec<Placement> {
        let thickness = self.thickness_studs.max(1);
        let mut local = Vec::new();

        let left = Column { height_plates: self.rise_plates, diameter_studs: thickness, color: self.color };
        let right = Column { height_plates: self.rise_plates, diameter_studs: thickness, color: self.color };
        local.extend(left.emit(GridPos::new(0, 0, 0), Orientation::IDENTITY));
        local.extend(right.emit(GridPos::new(studs(self.span_studs + thickness), 0, 0), Orientation::IDENTITY));

        let total_width = self.span_studs + 2 * thickness;
        let lintel = Wall {
            width_studs: total_width,
            height_plates: BRICK_PLATES,
            depth_studs: thickness,
            bond: Bond::Stack,
            color: self.color,
            part_set: PartSet::Classic,
        };
        // The lintel sits directly on top of the jambs: its bottom must
        // land exactly at the jambs' top, `-rise_plates` (rounded down to
        // a whole brick course, this module's coursing granularity).
        let lintel_top_plates = brick_courses(self.rise_plates) * BRICK_PLATES;
        local.extend(lintel.emit(GridPos::new(0, -(lintel_top_plates as i32), 0), Orientation::IDENTITY));

        transform_local(local, origin, orientation)
    }

    fn extent(&self) -> (u32, u32, u32) {
        let thickness = self.thickness_studs.max(1);
        (self.span_studs + 2 * thickness, thickness, self.rise_plates + BRICK_PLATES)
    }
}

pub struct Stair {
    pub run_studs: u32,
    pub rise_plates: u32,
    pub width_studs: u32,
    pub color: u32,
}

impl Primitive for Stair {
    fn emit(&self, origin: GridPos, orientation: Orientation) -> Vec<Placement> {
        let num_steps = (self.rise_plates / BRICK_PLATES).max(1);
        let step_run = (self.run_studs / num_steps).max(1);
        let mut local = Vec::new();
        for step in 0..num_steps {
            let block = Wall {
                width_studs: step_run,
                height_plates: (step + 1) * BRICK_PLATES,
                depth_studs: self.width_studs,
                bond: Bond::Stack,
                color: self.color,
                part_set: PartSet::Classic,
            };
            let mut block_placements = block.emit(GridPos::new(studs(step * step_run), 0, 0), Orientation::IDENTITY);
            // One real build stage per physical stair step — overwrites
            // the embedded Wall's per-course numbering, same pattern as
            // Ziggurat's per-tier override.
            for p in &mut block_placements {
                p.build_step = step;
            }
            local.extend(block_placements);
        }
        transform_local(local, origin, orientation)
    }

    fn extent(&self) -> (u32, u32, u32) {
        let num_steps = (self.rise_plates / BRICK_PLATES).max(1);
        let step_run = (self.run_studs / num_steps).max(1);
        (step_run * num_steps, self.width_studs, num_steps * BRICK_PLATES)
    }
}

pub struct Ziggurat {
    pub base_studs: u32,
    pub tiers: u32,
    pub tier_height_plates: u32,
    pub setback_studs: u32,
    pub color: u32,
}

impl Primitive for Ziggurat {
    fn emit(&self, origin: GridPos, orientation: Orientation) -> Vec<Placement> {
        let mut local = Vec::new();
        let mut cumulative_plates = 0i32;
        for tier in 0..self.tiers {
            let inset = self.setback_studs * tier;
            let size = self.base_studs.saturating_sub(2 * inset).max(1);
            let tier_wall = Wall {
                width_studs: size,
                height_plates: self.tier_height_plates,
                depth_studs: size,
                bond: Bond::Stack,
                color: self.color,
                part_set: PartSet::Classic,
            };
            let origin_for_tier = GridPos::new(studs(inset), -cumulative_plates, studs(inset));
            let mut tier_placements = tier_wall.emit(origin_for_tier, Orientation::IDENTITY);
            // One real build stage per tier (the real structural unit a
            // Ziggurat/Pyramid/Dome rises by), not per brick or per course
            // — overwrites whatever `Wall::emit` set internally, matching
            // the real Stonehenge precedent's grain (one stage per real
            // structural instance).
            for p in &mut tier_placements {
                p.build_step = tier;
            }
            local.extend(tier_placements);
            cumulative_plates += brick_courses(self.tier_height_plates) as i32 * BRICK_PLATES as i32;
        }
        transform_local(local, origin, orientation)
    }

    fn extent(&self) -> (u32, u32, u32) {
        (self.base_studs, self.base_studs, self.tiers * brick_courses(self.tier_height_plates) * BRICK_PLATES)
    }
}

/// Real 45-degree slope parts for a smooth pyramid's outer ring, measured
/// against the real LDraw cache the same way `grid::FootprintTable` is
/// (bounding box / `PLATE_LDU`, real center = `(min+max)/2` per axis) —
/// not typed from memory. All three straight slopes share the same real
/// cross-section (a real 2-stud taper run, 3-plate height), differing
/// only in real tileable length; the corner is a real *outside* (convex)
/// corner, the shape a pyramid's real corner needs — confirmed by an
/// actual rendered screenshot, not inferred from raw vertex winding
/// (BFC-uncorrected triangle normals came out ambiguous in sign).
///
/// | part | real name | studs (tileable x taper-run) | real local center (LDU) |
/// |---|---|---|---|
/// | `3037.dat` | Slope 45 4 x 2 | 4 x 2 | (0, -10) |
/// | `3039.dat` | Slope 45 2 x 2 | 2 x 2 | (0, -10) |
/// | `3040.dat` | Slope 45 1 x 2 | 1 x 2 | (0, -10) |
/// | `3045.dat` | Slope 45 2 x 2 Double Convex Corner | 2 x 2 | (10, -10) |
///
/// The real corner-slope footprint (2 studs) is *why* a smooth pyramid
/// uses a real 2-stud setback, not the stepped variant's 1 — no real
/// 1-stud outside-corner slope exists in this kit's part set.
const SLOPE_STRAIGHT: &[(&str, u32)] = &[("3037.dat", 4), ("3039.dat", 2), ("3040.dat", 1)];
const SLOPE_STRAIGHT_CENTER_LDU: [f64; 2] = [0.0, -10.0];
const SLOPE_CORNER_PART: &str = "3045.dat";
const SLOPE_CORNER_CENTER_LDU: [f64; 2] = [10.0, -10.0];
const SLOPE_RING_WIDTH_STUDS: u32 = 2;

/// Greedily tiles a run of `len_studs` studs with real slope parts,
/// longest first — same real pattern as `tile_length`, using
/// `SLOPE_STRAIGHT` instead of a `PartSet`'s bricks.
fn tile_length_slopes(len_studs: u32) -> Vec<(&'static str, u32)> {
    let mut remaining = len_studs;
    let mut out = Vec::new();
    for (part, w) in SLOPE_STRAIGHT {
        while remaining >= *w {
            out.push((*part, *w));
            remaining -= *w;
        }
    }
    debug_assert_eq!(remaining, 0, "SLOPE_STRAIGHT's real 1-stud slope must always finish the remainder");
    out
}

/// Places one real slope part so its real geometric center lands at
/// `world_center_studs` (in the ring's own local X/Z, studs from the
/// tier's front-bottom-left corner) — the part's own real local center
/// (`local_center_ldu`, see `SLOPE_STRAIGHT`'s doc comment) is rotated by
/// `orientation` before being subtracted back out, the same real
/// local-to-world relationship `transform_local` uses everywhere else in
/// this module (`translation = target - rotated(local_offset)`).
fn place_slope(part: &str, orientation: Orientation, world_center_studs: [f64; 2], local_center_ldu: [f64; 2], y_plates: i32, color: u32) -> Placement {
    let m = orientation.matrix();
    let rotated = mat_vec(m, [local_center_ldu[0], 0.0, local_center_ldu[1]]);
    Placement {
        part: part.to_string(),
        color,
        translation_ldu: [
            world_center_studs[0] * STUD_LDU - rotated[0],
            y_plates as f64 * PLATE_LDU,
            world_center_studs[1] * STUD_LDU - rotated[2],
        ],
        matrix: m,
        declared_off_grid: false,
        build_step: 0,
    }
}

/// Real smooth outer ring for one tier of a smooth-sided pyramid: the
/// exposed real 2-stud ledge that would otherwise be flat bricks (see
/// `Ziggurat`), replaced with real slope parts so the tier-to-tier
/// transition reads as one continuous taper instead of a stepped shelf.
/// `size` is the tier's own full real footprint in studs. Local space:
/// origin at the tier's own front-bottom-left corner (matching every
/// other primitive in this module), ring at course 0 (the tier's own
/// single course, since every `Pyramid` tier is exactly one real brick
/// tall — `tier_height_plates: BRICK_PLATES`).
fn smooth_ring(size: u32, color: u32) -> Vec<Placement> {
    let rw = SLOPE_RING_WIDTH_STUDS;
    let run = size.saturating_sub(2 * rw);
    let y_plates = course_bottom_plates(0);
    let mut out = Vec::new();

    // Real yaw per edge: the direction the slope's real tall/stud end
    // faces (outward, away from the pyramid's own center) — found by an
    // actual rendered screenshot at identity orientation, then composing
    // the real 90-degree-yaw members of the 24 orientations
    // (`Orientation::nearest_yaw`) for the other three edges, not derived
    // from raw vertex winding.
    let half = size as f64 / 2.0;
    let edge_specs: [(f64, [f64; 2]); 4] = [
        (0.0, [half, rw as f64 / 2.0]),                       // south: taper faces -Z (outward)
        (180.0, [half, size as f64 - rw as f64 / 2.0]),        // north: taper faces +Z (outward)
        (90.0, [rw as f64 / 2.0, half]),                       // west: taper faces -X (outward)
        (270.0, [size as f64 - rw as f64 / 2.0, half]),        // east: taper faces +X (outward)
    ];
    for (yaw, fixed_center) in edge_specs {
        let orientation = Orientation::nearest_yaw(yaw);
        // South/north tile along real X (the run axis); west/east tile
        // along real Z instead — a 90-degree yaw swaps which world axis a
        // part's own local (X-tileable) axis lands on.
        let along_x = yaw == 0.0 || yaw == 180.0;
        let mut cursor = rw as f64;
        for (part, w) in tile_length_slopes(run) {
            let center = cursor + w as f64 / 2.0;
            let world_center = if along_x { [center, fixed_center[1]] } else { [fixed_center[0], center] };
            out.push(place_slope(part, orientation, world_center, SLOPE_STRAIGHT_CENTER_LDU, y_plates, color));
            cursor += w as f64;
        }
    }

    // Real corners: one `SLOPE_CORNER_PART` each, same real yaw-rotation
    // technique as the edges above.
    let corner_specs: [(f64, [f64; 2]); 4] = [
        (0.0, [rw as f64 / 2.0, rw as f64 / 2.0]),                                   // SW
        (270.0, [size as f64 - rw as f64 / 2.0, rw as f64 / 2.0]),                   // SE
        (180.0, [size as f64 - rw as f64 / 2.0, size as f64 - rw as f64 / 2.0]),     // NE
        (90.0, [rw as f64 / 2.0, size as f64 - rw as f64 / 2.0]),                    // NW
    ];
    for (yaw, world_center) in corner_specs {
        let orientation = Orientation::nearest_yaw(yaw);
        out.push(place_slope(SLOPE_CORNER_PART, orientation, world_center, SLOPE_CORNER_CENTER_LDU, y_plates, color));
    }

    out
}

/// How many 1-brick-tall tiers it takes a `base_studs` square footprint to
/// shrink to a closed point, losing `2*setback_studs` per tier (inset on
/// every side). Shared by `Pyramid` and `Dome`, both of which corbel a
/// square footprint down to a cap — see each type's own doc comment for
/// why this milestone approximates them the same way.
fn tiers_to_close(base_studs: u32, setback_studs: u32) -> u32 {
    let setback = setback_studs.max(1);
    let mut tiers = 0u32;
    let mut size = base_studs.max(1);
    loop {
        tiers += 1;
        if size <= 2 * setback {
            break;
        }
        size -= 2 * setback;
    }
    tiers
}

pub struct Pyramid {
    pub base_studs: u32,
    pub color: u32,
    pub stepped: bool,
}

impl Primitive for Pyramid {
    fn emit(&self, origin: GridPos, orientation: Orientation) -> Vec<Placement> {
        if !self.stepped {
            return self.emit_smooth(origin, orientation);
        }
        let zig = Ziggurat {
            base_studs: self.base_studs,
            tiers: tiers_to_close(self.base_studs, 1),
            tier_height_plates: BRICK_PLATES,
            setback_studs: 1,
            color: self.color,
        };
        zig.emit(origin, orientation)
    }

    fn extent(&self) -> (u32, u32, u32) {
        let setback = if self.stepped { 1 } else { SLOPE_RING_WIDTH_STUDS };
        (self.base_studs, self.base_studs, tiers_to_close(self.base_studs, setback) * BRICK_PLATES)
    }
}

impl Pyramid {
    /// A real smooth-sided pyramid: each tier's exposed real ledge (the
    /// ring `smooth_ring` builds) is real slope parts instead of flat
    /// bricks, so the structure reads as one continuous taper instead of
    /// stepped shelves. Real setback is `SLOPE_RING_WIDTH_STUDS` (2), not
    /// `stepped: true`'s 1 — see `SLOPE_STRAIGHT`'s doc comment for why.
    /// The topmost tier (nothing above it to smooth into) stays a plain
    /// solid cap, same real choice `stepped: true`'s own top tier makes.
    fn emit_smooth(&self, origin: GridPos, orientation: Orientation) -> Vec<Placement> {
        let setback = SLOPE_RING_WIDTH_STUDS;
        let mut local = Vec::new();
        let mut cumulative_plates = 0i32;
        let mut size = self.base_studs.max(1);
        let mut tier = 0u32;
        loop {
            let is_last = size <= 2 * setback;
            let inner = size.saturating_sub(2 * setback).max(1);
            let mut tier_placements = if is_last {
                let wall = Wall { width_studs: size, height_plates: BRICK_PLATES, depth_studs: size, bond: Bond::Stack, color: self.color, part_set: PartSet::Classic };
                wall.emit(GridPos::new(0, -cumulative_plates, 0), Orientation::IDENTITY)
            } else {
                let core_origin = GridPos::new(studs(setback), -cumulative_plates, studs(setback));
                let core = Wall { width_studs: inner, height_plates: BRICK_PLATES, depth_studs: inner, bond: Bond::Stack, color: self.color, part_set: PartSet::Classic };
                let mut placements = core.emit(core_origin, Orientation::IDENTITY);
                let ring = smooth_ring(size, self.color);
                let mut ring = transform_local(ring, GridPos::new(0, -cumulative_plates, 0), Orientation::IDENTITY);
                placements.append(&mut ring);
                placements
            };
            for p in &mut tier_placements {
                p.build_step = tier;
            }
            local.extend(tier_placements);
            cumulative_plates += BRICK_PLATES as i32;
            if is_last {
                break;
            }
            size = inner;
            tier += 1;
        }
        transform_local(local, origin, orientation)
    }
}

pub struct Dome {
    pub radius_studs: u32,
    pub color: u32,
}

impl Primitive for Dome {
    /// A corbelled approximation: solid square courses, each inset by one
    /// stud on every side from the course below, closing to a single small
    /// cap course. **Solid, not a hollow vault** — an earlier hollow-ring
    /// (four short walls per course in a pinwheel) version turned out not
    /// to be a real self-supporting structure: consecutive rings only
    /// touch at the corners, well under this kit's real footprint-overlap
    /// threshold for vertical support, so `validate()` correctly reported
    /// most of it as `Floating`. That is a real, useful finding: a true
    /// hollow corbelled vault needs each course's *inner* edge to overhang
    /// no more than the course below can carry — real voussoir/corbel
    /// engineering this milestone does not attempt. A solid mass is the
    /// honest, self-supporting first cut (every course's footprint is
    /// fully contained in the one below it by construction, the same
    /// mechanism `Ziggurat` already proves correct), at the cost of no
    /// hollow interior. See the module doc comment.
    fn emit(&self, origin: GridPos, orientation: Orientation) -> Vec<Placement> {
        let outer0 = (self.radius_studs * 2).max(1);
        let zig = Ziggurat {
            base_studs: outer0,
            tiers: tiers_to_close(outer0, 1),
            tier_height_plates: BRICK_PLATES,
            setback_studs: 1,
            color: self.color,
        };
        zig.emit(origin, orientation)
    }

    fn extent(&self) -> (u32, u32, u32) {
        let outer0 = (self.radius_studs * 2).max(1);
        (outer0, outer0, tiers_to_close(outer0, 1) * BRICK_PLATES)
    }
}

pub struct Trilithon {
    pub post_height_plates: u32,
    pub gap_studs: u32,
    pub color: u32,
}

impl Primitive for Trilithon {
    fn emit(&self, origin: GridPos, orientation: Orientation) -> Vec<Placement> {
        let mut local = Vec::new();
        let left = Column { height_plates: self.post_height_plates, diameter_studs: 1, color: self.color };
        let right = Column { height_plates: self.post_height_plates, diameter_studs: 1, color: self.color };
        local.extend(left.emit(GridPos::new(0, 0, 0), Orientation::IDENTITY));
        local.extend(right.emit(GridPos::new(studs(self.gap_studs + 1), 0, 0), Orientation::IDENTITY));

        let lintel = Wall {
            width_studs: self.gap_studs + 2,
            height_plates: BRICK_PLATES,
            depth_studs: 1,
            bond: Bond::Stack,
            color: self.color,
            part_set: PartSet::Classic,
        };
        let lintel_top_plates = brick_courses(self.post_height_plates) * BRICK_PLATES;
        local.extend(lintel.emit(GridPos::new(0, -(lintel_top_plates as i32), 0), Orientation::IDENTITY));

        transform_local(local, origin, orientation)
    }

    fn extent(&self) -> (u32, u32, u32) {
        (self.gap_studs + 2, 1, self.post_height_plates + BRICK_PLATES)
    }
}

pub struct Colonnade {
    pub columns: u32,
    pub spacing_studs: u32,
    pub column: Column,
    pub architrave: bool,
}

impl Primitive for Colonnade {
    fn emit(&self, origin: GridPos, orientation: Orientation) -> Vec<Placement> {
        let n = self.columns.max(1);
        let diameter = self.column.diameter_studs.max(1);
        let mut local = Vec::new();
        for i in 0..n {
            let x = studs(i * self.spacing_studs);
            let col = Column { height_plates: self.column.height_plates, diameter_studs: diameter, color: self.column.color };
            // One real build stage per real column instance — the same
            // "one upright, one stage" grain the real Stonehenge file
            // uses. Column doesn't stage its own courses, so this is the
            // whole of its numbering.
            let mut col_placements = col.emit(GridPos::new(x, 0, 0), Orientation::IDENTITY);
            for p in &mut col_placements {
                p.build_step = i;
            }
            local.extend(col_placements);
        }
        if self.architrave && n > 0 {
            let span_studs = (n - 1) * self.spacing_studs + diameter;
            let beam = Wall {
                width_studs: span_studs,
                height_plates: BRICK_PLATES,
                depth_studs: diameter,
                bond: Bond::Stack,
                color: self.column.color,
                part_set: PartSet::Classic,
            };
            let beam_top_plates = brick_courses(self.column.height_plates) * BRICK_PLATES;
            // The architrave is one final real stage, raised only after
            // every real column below it — mirrors the real Stonehenge
            // file's "all uprights, then the lintel ring" sequencing.
            let mut beam_placements = beam.emit(GridPos::new(0, -(beam_top_plates as i32), 0), Orientation::IDENTITY);
            for p in &mut beam_placements {
                p.build_step = n;
            }
            local.extend(beam_placements);
        }
        transform_local(local, origin, orientation)
    }

    fn extent(&self) -> (u32, u32, u32) {
        let n = self.columns.max(1);
        let diameter = self.column.diameter_studs.max(1);
        let span_studs = (n - 1) * self.spacing_studs + diameter;
        let height = self.column.height_plates + if self.architrave { BRICK_PLATES } else { 0 };
        (span_studs, diameter, height)
    }
}

pub struct Mosaic {
    pub cells: Vec<Vec<u32>>,
    pub tile_part: String,
}

impl Primitive for Mosaic {
    /// One `tile_part` per cell, colored by the cell's own real LDraw
    /// color code, laid flat (a single 1-plate-tall course) on a 1-stud
    /// grid. `cells[row][col]`: row runs along local Z, col along local X.
    fn emit(&self, origin: GridPos, orientation: Orientation) -> Vec<Placement> {
        let mut local = Vec::new();
        for (row, cols) in self.cells.iter().enumerate() {
            for (col, &color) in cols.iter().enumerate() {
                let x_center = (col as f64 + 0.5) * STUD_LDU;
                let z_center = (row as f64 + 0.5) * STUD_LDU;
                local.push(Placement::on_grid(
                    GridPos::new(half_studs(x_center), 0, half_studs(z_center)),
                    Orientation::IDENTITY,
                    self.tile_part.clone(),
                    color,
                ));
            }
        }
        transform_local(local, origin, orientation)
    }

    fn extent(&self) -> (u32, u32, u32) {
        let rows = self.cells.len() as u32;
        let cols = self.cells.iter().map(|r| r.len()).max().unwrap_or(0) as u32;
        (cols, rows, 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::{validate, FootprintTable};

    #[test]
    fn wall_running_bond_produces_the_real_hand_computed_translations() {
        // Acceptance criterion 3's own test case: Wall{width: 20, height: 9,
        // bond: Running}. 9 plates = 3 brick courses. Even courses (0, 2):
        // 10x 1x2 stretcher, centers at studs 1,3,5,...,19 -> x_ldu
        // 20,60,100,...,380 -> half-studs 2,6,10,...,38.
        // Odd course (1): 1x1 at stud [0,1) center stud 0.5 -> x_ldu 10 ->
        // half-studs 1; then 9x 1x2 at studs 1..19, centers 2,4,...,18 ->
        // x_ldu 40,80,...,360 -> half-studs 4,8,...,36; then 1x1 at stud
        // [19,20) center stud 19.5 -> x_ldu 390 -> half-studs 39.
        let wall = Wall { width_studs: 20, height_plates: 9, depth_studs: 1, bond: Bond::Running, color: 15, part_set: PartSet::Classic };
        let placements = wall.emit(GridPos::new(0, 0, 0), Orientation::IDENTITY);

        let course0: Vec<i32> = placements[0..10].iter().map(|p| half_studs(p.translation_ldu[0])).collect();
        let expected_course0: Vec<i32> = (0..10).map(|i| 2 + 4 * i).collect();
        assert_eq!(course0, expected_course0, "even course: 10 real 1x2 stretchers, no offset");
        assert!(placements[0..10].iter().all(|p| p.part == "3004.dat"));

        let course1 = &placements[10..21];
        assert_eq!(course1.len(), 11, "odd course: 1x1 + 9x 1x2 + 1x1 = 11 real bricks");
        assert_eq!(course1[0].part, "3005.dat");
        assert_eq!(half_studs(course1[0].translation_ldu[0]), 1);
        for (i, p) in course1[1..10].iter().enumerate() {
            assert_eq!(p.part, "3004.dat");
            assert_eq!(half_studs(p.translation_ldu[0]), 4 + 4 * i as i32);
        }
        assert_eq!(course1[10].part, "3005.dat");
        assert_eq!(half_studs(course1[10].translation_ldu[0]), 39);

        let course2: Vec<i32> = placements[21..31].iter().map(|p| half_studs(p.translation_ldu[0])).collect();
        assert_eq!(course2, expected_course0, "course 2 (even again) repeats course 0's pattern");

        assert_eq!(placements.len(), 31, "10 + 11 + 10 real bricks across 3 courses");

        // One real build stage per course.
        assert!(placements[0..10].iter().all(|p| p.build_step == 0), "course 0");
        assert!(placements[10..21].iter().all(|p| p.build_step == 1), "course 1");
        assert!(placements[21..31].iter().all(|p| p.build_step == 2), "course 2");

        // And every one of them is grid-legal.
        let build_placements: Vec<crate::grid::Placement> = placements;
        let problems = validate(&build_placements, &FootprintTable::standard());
        assert_eq!(problems, vec![], "a real running-bond wall must validate with zero illegality");
    }

    #[test]
    fn wall_reports_real_part_counts_and_extent() {
        let wall = Wall { width_studs: 8, height_plates: 3, depth_studs: 1, bond: Bond::Stack, color: 15, part_set: PartSet::Classic };
        let placements = wall.emit(GridPos::new(0, 0, 0), Orientation::IDENTITY);
        // 8 studs, stack bond, single course: real greedy tiling, longest
        // real brick first = 1x 1x6 (3009.dat) + 1x 1x2 (3004.dat), not
        // 2x 1x4 or 4x 1x2.
        assert_eq!(placements.len(), 2);
        assert_eq!(placements[0].part, "3009.dat");
        assert_eq!(placements[1].part, "3004.dat");
        assert_eq!(wall.extent(), (8, 1, 3));
    }

    #[test]
    fn column_diameter_one_is_a_real_single_stud_stack() {
        let col = Column { height_plates: 9, diameter_studs: 1, color: 71 };
        let placements = col.emit(GridPos::new(0, 0, 0), Orientation::IDENTITY);
        assert_eq!(placements.len(), 3, "3 courses of a real 1x1 brick");
        assert!(placements.iter().all(|p| p.part == "3005.dat"));
        assert_eq!(col.extent(), (1, 1, 9));
        let problems = validate(&placements, &FootprintTable::standard());
        assert_eq!(problems, vec![]);
    }

    #[test]
    fn column_diameter_four_tiles_each_course_with_a_single_real_1x4_row() {
        let col = Column { height_plates: 3, diameter_studs: 4, color: 71 };
        let placements = col.emit(GridPos::new(0, 0, 0), Orientation::IDENTITY);
        assert_eq!(placements.len(), 4, "4 real 1x4 rows (one per row of the 4x4 square footprint)");
        assert!(placements.iter().all(|p| p.part == "3010.dat"));
    }

    #[test]
    fn trilithon_emits_two_real_posts_and_a_real_lintel_with_no_illegality() {
        let tri = Trilithon { post_height_plates: 9, gap_studs: 3, color: 72 };
        let placements = tri.emit(GridPos::new(0, 0, 0), Orientation::IDENTITY);
        // 2 posts * 3 courses each (real 1x1 bricks) + 1 lintel course, real
        // greedy tiling of 5 studs (gap 3 + 2 posts) = 1x 1x4 + 1x 1x1 =
        // 2 real bricks.
        assert_eq!(placements.len(), 2 * 3 + 2);
        assert_eq!(tri.extent(), (5, 1, 12));
        let problems = validate(&placements, &FootprintTable::standard());
        assert_eq!(problems, vec![]);
    }

    #[test]
    fn colonnade_places_real_columns_at_the_real_spacing() {
        let col = Colonnade {
            columns: 3,
            spacing_studs: 4,
            column: Column { height_plates: 3, diameter_studs: 1, color: 71 },
            architrave: false,
        };
        let placements = col.emit(GridPos::new(0, 0, 0), Orientation::IDENTITY);
        assert_eq!(placements.len(), 3, "3 columns, 1 course each");
        let xs: Vec<i32> = placements.iter().map(|p| half_studs(p.translation_ldu[0])).collect();
        assert_eq!(xs, vec![1, 9, 17], "spaced 4 studs = 8 half-studs apart, each centered on its own stud");
        assert_eq!(col.extent(), (9, 1, 3));
        // One real build stage per real column instance.
        let steps: Vec<u32> = placements.iter().map(|p| p.build_step).collect();
        assert_eq!(steps, vec![0, 1, 2]);
    }

    #[test]
    fn colonnade_architrave_is_one_real_final_stage_after_every_column() {
        let col = Colonnade {
            columns: 2,
            spacing_studs: 4,
            column: Column { height_plates: 3, diameter_studs: 1, color: 71 },
            architrave: true,
        };
        let placements = col.emit(GridPos::new(0, 0, 0), Orientation::IDENTITY);
        // 2 columns (1 real placement each) + 1 real architrave course.
        let column_steps: Vec<u32> = placements[0..2].iter().map(|p| p.build_step).collect();
        assert_eq!(column_steps, vec![0, 1]);
        assert!(placements[2..].iter().all(|p| p.build_step == 2), "the architrave is one final real stage, after both real columns");
    }

    #[test]
    fn mosaic_places_one_real_tile_per_cell_with_the_cells_own_color() {
        let mosaic = Mosaic { cells: vec![vec![4, 1], vec![15, 71]], tile_part: "3005.dat".to_string() };
        let placements = mosaic.emit(GridPos::new(0, 0, 0), Orientation::IDENTITY);
        assert_eq!(placements.len(), 4);
        assert_eq!(placements[0].color, 4);
        assert_eq!(placements[3].color, 71);
        assert_eq!(mosaic.extent(), (2, 2, 1));
    }

    #[test]
    fn ziggurat_tiers_step_inward_by_the_real_setback_and_stack_without_illegality() {
        let zig = Ziggurat { base_studs: 6, tiers: 2, tier_height_plates: 3, setback_studs: 1, color: 72 };
        let placements = zig.emit(GridPos::new(0, 0, 0), Orientation::IDENTITY);
        assert!(!placements.is_empty());
        assert_eq!(zig.extent(), (6, 6, 6));
        let problems = validate(&placements, &FootprintTable::standard());
        assert_eq!(problems, vec![]);
        // One real build stage per tier — every placement of tier 0 (the
        // real structural unit) shares one stage, tier 1 the next, not one
        // stage per brick/course.
        // tier_height_plates: 3 is exactly one real brick course (24 LDU);
        // this module's own "course c's bottom is at -((c+1)*BRICK_PLATES)
        // plates" convention puts tier 0's placements at real Y=-24 and
        // tier 1's at real Y=-48 — -36 cleanly bisects them.
        let tier0_steps: Vec<u32> = placements.iter().filter(|p| p.translation_ldu[1] > -36.0).map(|p| p.build_step).collect();
        let tier1_steps: Vec<u32> = placements.iter().filter(|p| p.translation_ldu[1] <= -36.0).map(|p| p.build_step).collect();
        assert!(!tier0_steps.is_empty() && tier0_steps.iter().all(|&s| s == 0), "{tier0_steps:?}");
        assert!(!tier1_steps.is_empty() && tier1_steps.iter().all(|&s| s == 1), "{tier1_steps:?}");
    }

    #[test]
    fn stair_has_one_more_course_per_step_and_a_real_part_count() {
        let stair = Stair { run_studs: 6, rise_plates: 9, width_studs: 2, color: 7 };
        let placements = stair.emit(GridPos::new(0, 0, 0), Orientation::IDENTITY);
        assert!(!placements.is_empty());
        assert_eq!(stair.extent(), (6, 2, 9));
        let problems = validate(&placements, &FootprintTable::standard());
        assert_eq!(problems, vec![]);
        // One real build stage per physical step, and each later step has
        // strictly more real placements than the one before it (a taller
        // block has more real courses) — real, hand-verifiable structure
        // without hardcoding the exact greedy-tiling part counts.
        let mut counts = [0u32; 3];
        for p in &placements {
            assert!(p.build_step < 3, "3 steps here, got build_step {}", p.build_step);
            counts[p.build_step as usize] += 1;
        }
        assert!(counts[0] > 0 && counts[1] > counts[0] && counts[2] > counts[1], "{counts:?}");
        // And build_step is emitted in non-decreasing order (step 0's real
        // placements all come before step 1's, etc.) — required for
        // write_ldr's real "0 STEP" emission to be correct.
        assert!(placements.windows(2).all(|w| w[0].build_step <= w[1].build_step));
    }

    #[test]
    fn arch_posts_and_lintel_validate_clean() {
        let arch = Arch { span_studs: 4, rise_plates: 6, thickness_studs: 1, color: 71 };
        let placements = arch.emit(GridPos::new(0, 0, 0), Orientation::IDENTITY);
        assert!(!placements.is_empty());
        let problems = validate(&placements, &FootprintTable::standard());
        assert_eq!(problems, vec![]);
    }

    #[test]
    fn dome_closes_to_a_cap_and_validates_clean() {
        let dome = Dome { radius_studs: 3, color: 1 };
        let placements = dome.emit(GridPos::new(0, 0, 0), Orientation::IDENTITY);
        assert!(!placements.is_empty());
        let problems = validate(&placements, &FootprintTable::standard());
        assert_eq!(problems, vec![]);
    }

    #[test]
    fn pyramid_stepped_validates_clean() {
        let a = Pyramid { base_studs: 6, color: 4, stepped: true };
        let placements = a.emit(GridPos::new(0, 0, 0), Orientation::IDENTITY);
        let problems = validate(&placements, &FootprintTable::standard());
        assert_eq!(problems, vec![]);
    }

    #[test]
    fn pyramid_smooth_uses_real_slope_parts_and_validates_clean() {
        let b = Pyramid { base_studs: 6, color: 4, stepped: false };
        let placements = b.emit(GridPos::new(0, 0, 0), Orientation::IDENTITY);
        // Real proof this is actually the smooth path, not a silent
        // no-op: real slope parts (straight + the real outside corner)
        // must be present, not just the flat PartSet::Classic bricks.
        assert!(placements.iter().any(|p| p.part == "3045.dat"), "expected the real corner slope");
        assert!(
            placements.iter().any(|p| SLOPE_STRAIGHT.iter().any(|(sp, _)| *sp == p.part)),
            "expected at least one real straight slope part"
        );
        let problems = validate(&placements, &FootprintTable::standard());
        assert_eq!(problems, vec![], "{problems:?}");
    }

    #[test]
    fn pyramid_smooth_has_exactly_four_real_corner_slopes_per_non_apex_tier() {
        // base_studs=10: setback 2 gives tiers 10 -> inner 6 (real ring) ->
        // is_last (6 <= 4? no -> inner 2, another real ring) -> is_last (2
        // <= 4, real solid cap). 2 real rings, so 2 * 4 = 8 real corners.
        let p = Pyramid { base_studs: 10, color: 4, stepped: false };
        let placements = p.emit(GridPos::new(0, 0, 0), Orientation::IDENTITY);
        let corners = placements.iter().filter(|p| p.part == "3045.dat").count();
        assert_eq!(corners, 8, "2 real rings x 4 real corners each");
    }

    #[test]
    fn pyramid_inherits_ziggurats_real_per_tier_build_staging_for_free() {
        // Pyramid delegates straight to Ziggurat::emit — no separate
        // staging code, this just proves that delegation actually carries
        // real, multi-valued build_step through rather than losing it.
        let pyramid = Pyramid { base_studs: 6, color: 4, stepped: true };
        let placements = pyramid.emit(GridPos::new(0, 0, 0), Orientation::IDENTITY);
        let distinct: std::collections::BTreeSet<u32> = placements.iter().map(|p| p.build_step).collect();
        assert!(distinct.len() > 1, "a multi-tier pyramid must have more than one real build stage: {distinct:?}");
        assert_eq!(*distinct.iter().min().unwrap(), 0, "stages are 0-based");
    }
}
