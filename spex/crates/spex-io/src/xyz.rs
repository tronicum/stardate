use anyhow::{Context, Result};
use spex_core::Point;
use std::io::BufRead;

/// Reads a whitespace- or comma-delimited text point cloud: `x y z [r g b]` per line.
/// Color is optional; missing color defaults to light gray so points are visible
/// against both light and dark viewer backgrounds.
pub fn read<R: BufRead>(reader: R) -> Result<Vec<Point>> {
    let mut points = Vec::new();
    for (line_no, line) in reader.lines().enumerate() {
        let line = line.with_context(|| format!("reading line {}", line_no + 1))?;
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let tokens: Vec<&str> = if line.contains(',') {
            line.split(',').map(str::trim).collect()
        } else {
            line.split_whitespace().collect()
        };
        if tokens.len() < 3 {
            anyhow::bail!("line {}: expected at least 3 fields (x y z), got {}", line_no + 1, tokens.len());
        }
        let x: f64 = tokens[0]
            .parse()
            .with_context(|| format!("line {}: invalid x '{}'", line_no + 1, tokens[0]))?;
        let y: f64 = tokens[1]
            .parse()
            .with_context(|| format!("line {}: invalid y '{}'", line_no + 1, tokens[1]))?;
        let z: f64 = tokens[2]
            .parse()
            .with_context(|| format!("line {}: invalid z '{}'", line_no + 1, tokens[2]))?;

        let color = if tokens.len() >= 6 {
            let parse_channel = |s: &str| -> Result<u8> {
                if let Ok(v) = s.parse::<u8>() {
                    Ok(v)
                } else {
                    // Some exporters write normalized 0..1 floats for color.
                    let f: f64 = s.parse().with_context(|| format!("invalid color channel '{s}'"))?;
                    Ok((f.clamp(0.0, 1.0) * 255.0).round() as u8)
                }
            };
            [
                parse_channel(tokens[3])?,
                parse_channel(tokens[4])?,
                parse_channel(tokens[5])?,
            ]
        } else {
            [200, 200, 200]
        };

        points.push(Point {
            position: [x, y, z],
            color,
        });
    }
    Ok(points)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn parses_whitespace_with_color() {
        let data = "1.0 2.0 3.0 255 0 0\n4 5 6\n";
        let pts = read(Cursor::new(data)).unwrap();
        assert_eq!(pts.len(), 2);
        assert_eq!(pts[0].position, [1.0, 2.0, 3.0]);
        assert_eq!(pts[0].color, [255, 0, 0]);
        assert_eq!(pts[1].color, [200, 200, 200]);
    }

    #[test]
    fn parses_csv() {
        let data = "1,2,3,255,255,255\n";
        let pts = read(Cursor::new(data)).unwrap();
        assert_eq!(pts[0].position, [1.0, 2.0, 3.0]);
        assert_eq!(pts[0].color, [255, 255, 255]);
    }
}
