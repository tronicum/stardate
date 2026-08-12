//! Turning a construction sheet into cells.
//!
//! Analytic and unfiltered on purpose: a cell takes the colour of whichever
//! element covers its **centre**, and nothing is blended. A 1×1 tile is one
//! colour — there is no such thing as a half-red stud — so anti-aliasing here
//! would only invent colours the mosaic cannot build, and then the quantiser
//! would have to map those invented colours onto real bricks. Sampling the
//! centre is both the simplest rule and the only one that answers the
//! question the mosaic actually asks.
//!
//! The consequence, stated rather than discovered later: a diagonal comes out
//! as a staircase, and at 48 studs wide the Union Flag's 1/30-height bands are
//! about 1.2 studs. That is a real property of building a flag out of studs,
//! not an artefact of this code, and `--width-studs` is the dial for it.
use crate::model::{Counterchange, Element, FlagSpec};
use anyhow::Result;

/// Rasterises the sheet at `width_studs` across. The height follows from the
/// flag's own ratio, rounded to a whole stud — a flag is built out of a whole
/// number of bricks in both directions.
///
/// `cells[row][col]`, row 0 at the top, col 0 at the hoist — the same
/// orientation `spex_build::Mosaic` reads.
pub fn rasterize(spec: &FlagSpec, width_studs: u32) -> Result<Vec<Vec<[u8; 3]>>> {
    anyhow::ensure!(width_studs >= 4, "a flag narrower than 4 studs is not a flag");
    let aspect = spec.aspect();
    let height_studs = ((width_studs as f64 / aspect).round() as u32).max(1);

    let mut cells = vec![vec![[0u8; 3]; width_studs as usize]; height_studs as usize];
    for row in 0..height_studs {
        // The centre of the cell, in normalised flag coordinates.
        let v = (row as f64 + 0.5) / height_studs as f64;
        for col in 0..width_studs {
            let u = (col as f64 + 0.5) / width_studs as f64;
            let mut rgb = None;
            for el in &spec.elements {
                if let Some(name) = covers(el, u, v, aspect) {
                    rgb = Some(spec.color(name)?);
                }
            }
            cells[row as usize][col as usize] =
                rgb.ok_or_else(|| anyhow::anyhow!("cell ({row},{col}) is covered by no element — the first element must be a field"))?;
        }
    }
    Ok(cells)
}

/// Which colour of `el`, if any, covers the point `(u, v)`.
///
/// Every width arrives as a fraction of the flag's HEIGHT (see `model.rs`), so
/// the only place the aspect ratio appears is where a height-width has to be
/// compared against a distance measured along `u`.
fn covers<'a>(el: &'a Element, u: f64, v: f64, aspect: f64) -> Option<&'a str> {
    match el {
        Element::StripesVertical { colors } => {
            let i = ((u * colors.len() as f64).floor() as usize).min(colors.len() - 1);
            Some(&colors[i])
        }
        Element::StripesHorizontal { colors } => {
            let i = ((v * colors.len() as f64).floor() as usize).min(colors.len() - 1);
            Some(&colors[i])
        }
        Element::Cross { color, arm_width_fraction, center_x_fraction, center_y_fraction } => {
            let half = arm_width_fraction / 2.0;
            // The horizontal arm is `half` of the height above and below its
            // centre; the vertical arm is the same physical width, which along
            // `u` is that much divided by the aspect ratio.
            let in_horizontal = (v - center_y_fraction).abs() <= half;
            let in_vertical = (u - center_x_fraction).abs() <= half / aspect;
            (in_horizontal || in_vertical).then_some(color.as_str())
        }
        Element::Saltire { color, arm_width_fraction, offset_fraction, counterchange } => {
            let half = arm_width_fraction / 2.0;
            let offset = offset_fraction.unwrap_or(0.0);
            // On the hoist half the band sits below each centreline, on the
            // fly half above. One rule, four arms — the flag is its own
            // half-turn about the centre, and this is that symmetry written
            // down. See `Counterchange::HoistBroadAbove`.
            let sign = match counterchange {
                Some(Counterchange::HoistBroadAbove) => {
                    if u < 0.5 {
                        1.0
                    } else {
                        -1.0
                    }
                }
                None => 1.0,
            };
            let centre = offset * sign;
            // Signed vertical distance from each diagonal, in units of the
            // flag's height. Vertical rather than perpendicular because the
            // published widths are fractions of the height and the same sheet
            // serves 1:2 and 3:5 — a perpendicular measure would make the
            // bands change width when the ratio changed.
            let d1 = v - u; // corner (hoist, top) to (fly, bottom)
            let d2 = v - (1.0 - u); // corner (hoist, bottom) to (fly, top)
            let hit = |d: f64| (d - centre).abs() <= half;
            (hit(d1) || hit(d2)).then_some(color.as_str())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::FlagColor;
    use std::collections::BTreeMap;

    fn palette(pairs: &[(&str, [u8; 3])]) -> BTreeMap<String, FlagColor> {
        pairs
            .iter()
            .map(|(n, c)| ((*n).to_string(), FlagColor { srgb: *c, specified_as: None }))
            .collect()
    }

    /// A square flag with one centred cross: the arms must be the same number
    /// of cells thick in both directions. This is the property the whole
    /// "widths are fractions of the height" convention exists to guarantee,
    /// and it is worth one test on a shape where the answer is obvious.
    #[test]
    fn cross_arms_are_equally_thick_in_both_directions() {
        let spec = FlagSpec {
            version: 1,
            iso2: "XX".into(),
            name: "test".into(),
            ratio: [1, 2],
            source: "test".into(),
            colors: palette(&[("bg", [0, 0, 0]), ("fg", [255, 255, 255])]),
            elements: vec![
                Element::StripesVertical { colors: vec!["bg".into()] },
                Element::Cross {
                    color: "fg".into(),
                    arm_width_fraction: 0.2,
                    center_x_fraction: 0.5,
                    center_y_fraction: 0.5,
                },
            ],
            unsupported: false,
            note: None,
        };
        let cells = rasterize(&spec, 60).unwrap();
        assert_eq!(cells.len(), 30, "1:2 at 60 studs wide is 30 tall");
        // Count the horizontal arm down the hoist column, where only it can
        // reach; count the vertical arm along the top row, for the same
        // reason. Sampling the middle of either would just be measuring the
        // crossing.
        let horizontal_arm = cells.iter().filter(|r| r[0] == [255, 255, 255]).count();
        let vertical_arm = cells[0].iter().filter(|c| **c == [255, 255, 255]).count();
        // 0.2 of the height = 6 cells of 30 — and 6 cells in the other
        // direction too, not 12, because the arm is as wide as it is thick.
        assert_eq!(horizontal_arm, 6, "horizontal arm");
        assert_eq!(vertical_arm, 6, "vertical arm");
    }
}
