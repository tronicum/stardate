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
            position: [p.x, p.y, p.z],
            color,
        });
    }
    Ok(points)
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
        assert_eq!(points[0].position, [1.0, 2.0, 3.0]);
        assert_eq!(points[0].color, [255, 0, 128]);
    }

    #[test]
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
}
