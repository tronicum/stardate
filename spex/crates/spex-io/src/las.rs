use anyhow::{Context, Result};
use spex_core::Point;
use std::path::Path;

/// Reads a real ASPRS LAS or LAZ point cloud file via the `las` crate (LAZ
/// decompression via its `laz` feature, backed by `laz-rs` — real LASzip
/// support, not a stub). Unlike `ply`/`xyz` this takes the path directly
/// rather than a generic `BufRead`: `las::Reader` needs `Seek` (LAS/LAZ are
/// not a simple line-oriented streaming format — the header records exact
/// byte offsets/counts read back out of order) plus `'static`, which
/// `Reader::from_path` already satisfies internally.
///
/// Real airborne LiDAR scans are overwhelmingly intensity-only (no RGB) —
/// photogrammetry-fused color is the exception, not the rule — so a point
/// with no real color falls back to a grayscale shade of its own real
/// `intensity` value (normalized `u16` -> `u8`) rather than a flat fabricated
/// gray, keeping every visible value tied to something the scanner actually
/// measured.
///
/// The ASPRS spec's RGB fields are 16-bit per channel, and compliant
/// producers scale an 8-bit source up (`value * 256`) before writing it —
/// see [`color_channels_look_8bit`] for why real-world producers don't
/// always do that, and why this reader detects and corrects for it rather
/// than trusting the spec blindly.
pub fn read(path: &Path) -> Result<Vec<Point>> {
    let mut reader = las::Reader::from_path(path).with_context(|| format!("opening {} as LAS/LAZ", path.display()))?;
    let point_data = reader.read_all().with_context(|| format!("reading points from {}", path.display()))?;

    // Scanning every point's raw color once, up front, to answer one
    // whole-file question ("is this producer's RGB actually 8-bit despite
    // the 16-bit field width?") is cheap next to the per-point Point
    // materialization below, and `PointData::points()` is a cheap
    // non-consuming iterator over data already fully resident in memory
    // (see `las::PointData`), so a second pass costs no extra I/O.
    let max_channel = point_data
        .points()
        .filter_map(|w| w.ok())
        .filter_map(|p| p.color)
        .flat_map(|c| [c.red, c.green, c.blue])
        .max()
        .unwrap_or(0);
    let color_is_8bit_range = color_channels_look_8bit(max_channel);

    let mut points = Vec::new();
    for wrapped in point_data.points() {
        let p = wrapped.with_context(|| format!("reading a point from {}", path.display()))?;
        let color = match p.color {
            Some(c) if color_is_8bit_range => [c.red as u8, c.green as u8, c.blue as u8],
            Some(c) => [(c.red >> 8) as u8, (c.green >> 8) as u8, (c.blue >> 8) as u8],
            None => {
                let gray = (p.intensity >> 8) as u8;
                [gray, gray, gray]
            }
        };
        points.push(Point {
            position: las_to_spex(p.x, p.y, p.z),
            color,
        });
    }
    Ok(points)
}

/// Whether a file's real RGB data — despite living in the LAS spec's 16-bit
/// fields — is actually already 8-bit-range (`0..=255`) and would be wiped
/// out to pure black by a blind `>> 8`.
///
/// The ASPRS LAS spec says an 8-bit source channel should be scaled up by
/// 256 before being stored in the 16-bit field (so pixel value 255 becomes
/// 65280, not 255) — precisely so a `>> 8` on read recovers the original
/// 8-bit value. Plenty of real-world LAS producers don't do that scale-up
/// and just write the raw 8-bit value into the low byte of the 16-bit
/// field instead. Reading that non-compliant-but-real data with a blind
/// `>> 8` doesn't get a slightly-wrong color — for any channel value under
/// 256 it becomes exactly `0`, so *every* point in such a file renders
/// pure black. That's what a real committed fixture
/// (`scripts/point-cloud-data/autzen-trim.las`, real Autzen Stadium
/// airborne LiDAR) turned out to hit: every one of its 6,002 points has
/// real, distinct RGB (confirmed by inspecting the raw values, e.g. a real
/// `Color { red: 84, green: 102, blue: 93 }`), but every value is under
/// 256, so the naive path decoded all 6,002 to `[0, 0, 0]` — which is what
/// made `spex ascii` (whose darkest glyph is a space) look completely
/// blank; issue #34 originally suspected the ASCII camera/FOV math, but
/// tracing real `(sx, sy)` values for this fixture found every point
/// correctly inside the camera's field of view — the points were being
/// projected just fine, they just had no visible color once decoded.
///
/// The detection heuristic: if the *whole file's* maximum real channel
/// value never exceeds 255, the data can't be legitimate spec-compliant
/// 16-bit color (which would only stay under 256 by having every channel
/// of every point sit in the bottom 0.4% of the 16-bit range — implausible
/// for real captured imagery with any color variation at all) — so treat
/// it as already-8-bit and use it directly instead of shifting it away.
/// `max_channel == 0` (a file with no real color, or all real color
/// legitimately black) is left on the spec-compliant `>> 8` path, since
/// there's nothing to lose either way.
fn color_channels_look_8bit(max_channel: u16) -> bool {
    (1..=255).contains(&max_channel)
}

/// Real ASPRS LAS/LAZ data (a projected/geographic CRS, e.g. UTM or a state
/// plane system) is conventionally X=easting, Y=northing, Z=elevation — a
/// right-handed, **Z-up** coordinate system. spex's renderers (the WebGL
/// viewer and `spex ascii`) both assume **Y-up** world space (three.js/
/// OrbitControls' `(0,1,0)`, see `ascii.rs`'s `default_camera` doc
/// comment). Passing LAS's raw axes straight through un-remapped puts a
/// real scan's northing (large, horizontal in reality) on spex's vertical
/// axis while real elevation (small) ends up on spex's depth axis — a real
/// geographic scan renders tipped over on its side (found via a real
/// committed fixture, `scripts/point-cloud-data/autzen-trim.las`). That
/// fixture's `spex ascii` render was *also* blank before this fix, which
/// this rotation alone doesn't explain — see [`color_channels_look_8bit`]
/// for the actual (separate, since-fixed) cause: every point in that file
/// decoded to pure black due to an unrelated RGB bit-depth bug, not
/// anything about camera framing or axis orientation.
///
/// The standard Z-up -> Y-up conversion — `(x, y, z) -> (x, z, -y)`, a real
/// -90-degree rotation about the X axis — is used here rather than a plain
/// Y/Z swap specifically because a swap alone is a *reflection* (mirrors
/// the scene, flips handedness); a rotation preserves it, so real
/// structures (e.g. text, asymmetric buildings) don't come out
/// mirror-imaged.
fn las_to_spex(x: f64, y: f64, z: f64) -> [f64; 3] {
    [x, z, -y]
}

#[cfg(test)]
mod tests {
    use super::*;
    use las::{Builder, Color as LasColor, Point as LasPoint, Writer};

    fn write_las(path: &Path, points: Vec<LasPoint>, with_color: bool) {
        let mut builder = Builder::default();
        if with_color {
            builder.point_format.has_color = true;
        }
        let header = builder.into_header().unwrap();
        let mut writer = Writer::from_path(path, header).unwrap();
        for p in points {
            writer.write_point(p).unwrap();
        }
        writer.close().unwrap();
    }

    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn reads_real_color_when_present() {
        let mut p = LasPoint::default();
        p.x = 1.0;
        p.y = 2.0;
        p.z = 3.0;
        p.color = Some(LasColor::new(65535, 0, 32768));

        let tmp = std::env::temp_dir().join("spex-las-test-color.las");
        write_las(&tmp, vec![p], true);
        let points = read(&tmp).unwrap();
        std::fs::remove_file(&tmp).ok();

        assert_eq!(points.len(), 1);
        // Z-up -> Y-up: (x, y, z) -> (x, z, -y).
        assert_eq!(points[0].position, [1.0, 3.0, -2.0]);
        assert_eq!(points[0].color, [255, 0, 128]);
    }

    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn falls_back_to_intensity_grayscale_when_no_color() {
        let mut p = LasPoint::default();
        p.x = 10.0;
        p.y = 20.0;
        p.z = 30.0;
        p.intensity = 65535;

        let tmp = std::env::temp_dir().join("spex-las-test-nocolor.las");
        write_las(&tmp, vec![p], false);
        let points = read(&tmp).unwrap();
        std::fs::remove_file(&tmp).ok();

        assert_eq!(points.len(), 1);
        assert_eq!(points[0].color, [255, 255, 255]);
    }

    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn recovers_real_8bit_range_color_instead_of_zeroing_it_out() {
        // The real-world quirk this guards against: a producer writes an
        // already-8-bit channel value (0..=255) directly into LAS's 16-bit
        // RGB field instead of scaling it up by 256 as the spec expects.
        // A blind `>> 8` on a value like 84 (as seen in the real
        // autzen-trim.las fixture) yields exactly 0 for every such point.
        let mut points_in = Vec::new();
        for (r, g, b) in [(84u16, 102u16, 93u16), (58, 72, 70), (164, 153, 126)] {
            let mut p = LasPoint::default();
            p.x = 0.0;
            p.y = 0.0;
            p.z = 0.0;
            p.color = Some(LasColor::new(r, g, b));
            points_in.push(p);
        }

        let tmp = std::env::temp_dir().join("spex-las-test-8bit-color.las");
        write_las(&tmp, points_in, true);
        let points = read(&tmp).unwrap();
        std::fs::remove_file(&tmp).ok();

        assert_eq!(points.len(), 3);
        assert_eq!(points[0].color, [84, 102, 93]);
        assert_eq!(points[1].color, [58, 72, 70]);
        assert_eq!(points[2].color, [164, 153, 126]);
        assert!(points.iter().all(|p| p.color != [0, 0, 0]), "real distinct color should never decode to pure black");
    }

    #[test]
    fn real_committed_geographic_fixture_has_real_non_black_color() {
        // The actual root cause traced for issue #34 ("spex ascii renders
        // blank for this fixture"): every point in this real file has real,
        // distinct RGB, but the naive 16-bit `>> 8` decode zeroed all of it
        // out because the file's real values are 8-bit-range. Confirm the
        // fix recovers real color on the real fixture, not just a
        // synthetic one.
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/point-cloud-data/autzen-trim.las");
        let points = read(&fixture).expect("reading the real committed autzen-trim.las fixture");
        assert!(!points.is_empty());
        assert!(points.iter().any(|p| p.color != [0, 0, 0]), "at least some real points should have non-black color");
        // Every real point in this fixture carries real color (confirmed by
        // direct inspection) -- this used to be 100% [0,0,0] before the fix.
        let black_count = points.iter().filter(|p| p.color == [0, 0, 0]).count();
        assert!(black_count < points.len() / 2, "most real points should now decode to non-black color, got {black_count}/{} black", points.len());
    }

    #[test]
    fn las_to_spex_rotates_z_up_to_y_up_without_mirroring() {
        // A real -90-degree rotation about X, not a Y/Z swap (which would
        // be a reflection): a positive-Z (real "up") input must land on
        // spex's positive-Y (also "up"), and a positive-Y (real "north")
        // input must land on spex's *negative* Z, not positive — that's
        // the part a naive swap would get backwards/mirrored.
        assert_eq!(las_to_spex(1.0, 0.0, 0.0), [1.0, 0.0, 0.0]);
        assert_eq!(las_to_spex(0.0, 1.0, 0.0), [0.0, 0.0, -1.0]);
        assert_eq!(las_to_spex(0.0, 0.0, 1.0), [0.0, 1.0, 0.0]);
    }

    #[test]
    fn real_committed_geographic_fixture_ends_up_with_elevation_as_the_smallest_extent() {
        // scripts/point-cloud-data/autzen-trim.las is real airborne LiDAR
        // (see its own README.md) — real terrain, so its real elevation
        // range is genuinely tiny next to its real horizontal footprint.
        // Before the Z-up -> Y-up fix, this fixture's real northing (huge)
        // landed on spex's Y (vertical) axis instead — this is the exact
        // real bug (issue #18) this fix resolves, checked against the
        // real committed file, not a synthetic one.
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/point-cloud-data/autzen-trim.las");
        let points = read(&fixture).expect("reading the real committed autzen-trim.las fixture");
        assert!(!points.is_empty());

        let mut min = [f64::INFINITY; 3];
        let mut max = [f64::NEG_INFINITY; 3];
        for p in &points {
            for axis in 0..3 {
                min[axis] = min[axis].min(p.position[axis]);
                max[axis] = max[axis].max(p.position[axis]);
            }
        }
        let extent: Vec<f64> = (0..3).map(|axis| max[axis] - min[axis]).collect();

        assert!(
            extent[1] < extent[0] && extent[1] < extent[2],
            "real elevation (Y after remap) should be the smallest extent: {extent:?}"
        );
    }
}
