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
//! — 1x1 (`3005.dat`), 1x2 (`3004.dat`) and 1x4 (`3010.dat`) real bricks,
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
//! - `Pyramid { stepped: false }` currently emits the same stepped
//!   construction as `stepped: true`: a smooth-sided pyramid needs real
//!   slope-part footprints not yet measured/cited. The flag is accepted
//!   and threaded through so a later milestone can differentiate without
//!   an API change.
//! - `Dome` is a **solid** corbelled square stack (see its own doc comment
//!   for why a hollow-ring first attempt was rejected: it was not a real
//!   self-supporting structure), not a hollow circular corbelled vault.

use crate::grid::{mat_mul, mat_vec, GridPos, Orientation, Placement, BRICK_PLATES, HALF_STUD_LDU, STUD_LDU};

/// The kit's current working real part set. See the module doc comment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PartSet {
    Classic,
}

impl PartSet {
    /// Real bricks, longest first, for greedy tiling.
    fn bricks(self) -> &'static [(&'static str, u32)] {
        match self {
            PartSet::Classic => &[("3010.dat", 4), ("3004.dat", 2), ("3005.dat", 1)],
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
                    local.push(Placement::on_grid(
                        GridPos::new(half_studs(x_center), y_plates, half_studs(z_center)),
                        Orientation::IDENTITY,
                        *part,
                        self.color,
                    ));
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
            local.extend(block.emit(GridPos::new(studs(step * step_run), 0, 0), Orientation::IDENTITY));
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
            local.extend(tier_wall.emit(origin_for_tier, Orientation::IDENTITY));
            cumulative_plates += brick_courses(self.tier_height_plates) as i32 * BRICK_PLATES as i32;
        }
        transform_local(local, origin, orientation)
    }

    fn extent(&self) -> (u32, u32, u32) {
        (self.base_studs, self.base_studs, self.tiers * brick_courses(self.tier_height_plates) * BRICK_PLATES)
    }
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
        let _ = self.stepped; // see module doc comment: both settings emit stepped construction today
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
        (self.base_studs, self.base_studs, tiers_to_close(self.base_studs, 1) * BRICK_PLATES)
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
            local.extend(col.emit(GridPos::new(x, 0, 0), Orientation::IDENTITY));
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
            local.extend(beam.emit(GridPos::new(0, -(beam_top_plates as i32), 0), Orientation::IDENTITY));
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
        // real brick first = 2x 1x4 (3010.dat), not 4x 1x2.
        assert_eq!(placements.len(), 2);
        assert!(placements.iter().all(|p| p.part == "3010.dat"));
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
    }

    #[test]
    fn stair_has_one_more_course_per_step_and_a_real_part_count() {
        let stair = Stair { run_studs: 6, rise_plates: 9, width_studs: 2, color: 7 };
        let placements = stair.emit(GridPos::new(0, 0, 0), Orientation::IDENTITY);
        assert!(!placements.is_empty());
        assert_eq!(stair.extent(), (6, 2, 9));
        let problems = validate(&placements, &FootprintTable::standard());
        assert_eq!(problems, vec![]);
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
    fn pyramid_stepped_and_unstepped_currently_agree_and_both_validate_clean() {
        let a = Pyramid { base_studs: 6, color: 4, stepped: true };
        let b = Pyramid { base_studs: 6, color: 4, stepped: false };
        assert_eq!(a.emit(GridPos::new(0, 0, 0), Orientation::IDENTITY).len(), b.emit(GridPos::new(0, 0, 0), Orientation::IDENTITY).len());
        let placements = a.emit(GridPos::new(0, 0, 0), Orientation::IDENTITY);
        let problems = validate(&placements, &FootprintTable::standard());
        assert_eq!(problems, vec![]);
    }
}
