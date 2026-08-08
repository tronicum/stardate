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
pub fn read(path: &Path) -> Result<Vec<Point>> {
    let mut reader = las::Reader::from_path(path).with_context(|| format!("opening {} as LAS/LAZ", path.display()))?;
    let point_data = reader.read_all().with_context(|| format!("reading points from {}", path.display()))?;

    let mut points = Vec::new();
    for wrapped in point_data.points() {
        let p = wrapped.with_context(|| format!("reading a point from {}", path.display()))?;
        let color = match p.color {
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

/// Real ASPRS LAS/LAZ data (a projected/geographic CRS, e.g. UTM or a state
/// plane system) is conventionally X=easting, Y=northing, Z=elevation — a
/// right-handed, **Z-up** coordinate system. spex's renderers (the WebGL
/// viewer and `spex ascii`) both assume **Y-up** world space (three.js/
/// OrbitControls' `(0,1,0)`, see `ascii.rs`'s `default_camera` doc
/// comment). Passing LAS's raw axes straight through un-remapped puts a
/// real scan's northing (large, horizontal in reality) on spex's vertical
/// axis while real elevation (small) ends up on spex's depth axis — a real
/// geographic scan renders tipped over on its side (found via a real
/// committed fixture, `scripts/point-cloud-data/autzen-trim.las`, whose
/// `spex ascii` render was blank before this fix — the camera framing
/// logic breaks down for that resulting extreme, wrongly-oriented aspect
/// ratio).
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
