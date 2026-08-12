//! Mapping a flag's own colours onto real bricks, and reporting how far it had
//! to move them.
//!
//! CIEDE2000 rather than a distance in sRGB, because sRGB distance is not a
//! distance: the same numeric gap is a different amount of visible difference
//! depending on where in the cube you stand, and a flag's red and a brick's
//! red are exactly the case where that matters. This is the third time this
//! project has needed it — the terminal green (ΔE 11.9) and the Batima creme
//! (ΔE 10.1) were both decided this way — so it lives in a crate now rather
//! than in a script.
//!
//! The report is not decoration. A flag whose worst cell is ΔE 20 away from
//! anything LEGO ever moulded is a flag this piece should *say* it cannot
//! build, because "a flag that cannot be built in the available palette is
//! this thesis's best counter-evidence, and showing it costs nothing."

/// sRGB 0..255 to CIELAB, through linear RGB and CIE XYZ under D65.
pub fn srgb_to_lab(rgb: [u8; 3]) -> [f64; 3] {
    let lin = |c: u8| {
        let c = c as f64 / 255.0;
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    let (r, g, b) = (lin(rgb[0]), lin(rgb[1]), lin(rgb[2]));
    // sRGB primaries under D65, the standard matrix.
    let x = 0.4124564 * r + 0.3575761 * g + 0.1804375 * b;
    let y = 0.2126729 * r + 0.7151522 * g + 0.0721750 * b;
    let z = 0.0193339 * r + 0.1191920 * g + 0.9503041 * b;
    // D65 white point.
    let (xn, yn, zn) = (0.9504559, 1.0, 1.0890578);
    let f = |t: f64| {
        const D: f64 = 6.0 / 29.0;
        if t > D * D * D {
            t.cbrt()
        } else {
            t / (3.0 * D * D) + 4.0 / 29.0
        }
    };
    let (fx, fy, fz) = (f(x / xn), f(y / yn), f(z / zn));
    [116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz)]
}

/// CIEDE2000, the full formula including the hue-rotation term.
///
/// Written out rather than pulled in as a dependency: it is forty lines of
/// arithmetic from a published paper, and this crate's whole point is that
/// its numbers can be checked against the paper.
pub fn ciede2000(lab1: [f64; 3], lab2: [f64; 3]) -> f64 {
    let (l1, a1, b1) = (lab1[0], lab1[1], lab1[2]);
    let (l2, a2, b2) = (lab2[0], lab2[1], lab2[2]);
    let c1 = (a1 * a1 + b1 * b1).sqrt();
    let c2 = (a2 * a2 + b2 * b2).sqrt();
    let c_bar = (c1 + c2) / 2.0;
    let c7 = c_bar.powi(7);
    let g = 0.5 * (1.0 - (c7 / (c7 + 25f64.powi(7))).sqrt());
    let a1p = (1.0 + g) * a1;
    let a2p = (1.0 + g) * a2;
    let c1p = (a1p * a1p + b1 * b1).sqrt();
    let c2p = (a2p * a2p + b2 * b2).sqrt();

    let hp = |ap: f64, b: f64| {
        if ap == 0.0 && b == 0.0 {
            0.0
        } else {
            let h = b.atan2(ap).to_degrees();
            if h < 0.0 {
                h + 360.0
            } else {
                h
            }
        }
    };
    let h1p = hp(a1p, b1);
    let h2p = hp(a2p, b2);

    let dlp = l2 - l1;
    let dcp = c2p - c1p;
    let dhp = if c1p * c2p == 0.0 {
        0.0
    } else if (h2p - h1p).abs() <= 180.0 {
        h2p - h1p
    } else if h2p - h1p > 180.0 {
        h2p - h1p - 360.0
    } else {
        h2p - h1p + 360.0
    };
    let dhp_big = 2.0 * (c1p * c2p).sqrt() * (dhp.to_radians() / 2.0).sin();

    let lp_bar = (l1 + l2) / 2.0;
    let cp_bar = (c1p + c2p) / 2.0;
    let hp_bar = if c1p * c2p == 0.0 {
        h1p + h2p
    } else if (h1p - h2p).abs() <= 180.0 {
        (h1p + h2p) / 2.0
    } else if h1p + h2p < 360.0 {
        (h1p + h2p + 360.0) / 2.0
    } else {
        (h1p + h2p - 360.0) / 2.0
    };

    let t = 1.0 - 0.17 * ((hp_bar - 30.0).to_radians()).cos()
        + 0.24 * ((2.0 * hp_bar).to_radians()).cos()
        + 0.32 * ((3.0 * hp_bar + 6.0).to_radians()).cos()
        - 0.20 * ((4.0 * hp_bar - 63.0).to_radians()).cos();
    let d_theta = 30.0 * (-(((hp_bar - 275.0) / 25.0).powi(2))).exp();
    let cp7 = cp_bar.powi(7);
    let rc = 2.0 * (cp7 / (cp7 + 25f64.powi(7))).sqrt();
    let sl = 1.0 + (0.015 * (lp_bar - 50.0).powi(2)) / (20.0 + (lp_bar - 50.0).powi(2)).sqrt();
    let sc = 1.0 + 0.045 * cp_bar;
    let sh = 1.0 + 0.015 * cp_bar * t;
    let rt = -(2.0 * d_theta.to_radians()).sin() * rc;

    ((dlp / sl).powi(2) + (dcp / sc).powi(2) + (dhp_big / sh).powi(2) + rt * (dcp / sc) * (dhp_big / sh)).sqrt()
}

/// What the mapping cost, in the units a person can argue about.
#[derive(Clone, Debug, Default)]
pub struct QuantizeReport {
    pub max_delta_e: f64,
    pub mean_delta_e: f64,
    /// The real LDraw codes the flag ended up using, sorted.
    pub used_colors: Vec<u32>,
    /// The distinct source colours and where each one landed, for the note.
    pub mapping: Vec<([u8; 3], u32, f64)>,
}

/// Maps every cell to the nearest permitted real LDraw colour.
///
/// `palette` is the codes this flag is allowed to use; the caller decides what
/// "permitted" means (opaque, solid finish, currently produced) because that
/// is a policy about a brick collection and not a fact about colour.
///
/// Distinct source colours are resolved once and reused — a flag has three or
/// four colours and tens of thousands of cells, and running CIEDE2000 against
/// the whole palette per cell would be arithmetic nobody reads.
pub fn quantize(
    cells: &[Vec<[u8; 3]>],
    colors: &std::collections::HashMap<u32, [u8; 3]>,
    palette: &[u32],
) -> anyhow::Result<(Vec<Vec<u32>>, QuantizeReport)> {
    anyhow::ensure!(!palette.is_empty(), "an empty palette can express no flag");
    let lab_palette: Vec<(u32, [f64; 3])> = palette
        .iter()
        .map(|code| {
            let rgb = colors
                .get(code)
                .ok_or_else(|| anyhow::anyhow!("palette lists LDraw colour {code}, which the real colour table does not contain"))?;
            Ok((*code, srgb_to_lab(*rgb)))
        })
        .collect::<anyhow::Result<_>>()?;

    let mut resolved: std::collections::HashMap<[u8; 3], (u32, f64)> = Default::default();
    let mut out = Vec::with_capacity(cells.len());
    let mut sum = 0.0;
    let mut count = 0usize;
    let mut max = 0.0f64;
    for row in cells {
        let mut orow = Vec::with_capacity(row.len());
        for &rgb in row {
            let (code, de) = *resolved.entry(rgb).or_insert_with(|| {
                let lab = srgb_to_lab(rgb);
                lab_palette
                    .iter()
                    .map(|(code, plab)| (*code, ciede2000(lab, *plab)))
                    .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
                    .expect("palette is non-empty")
            });
            orow.push(code);
            sum += de;
            count += 1;
            max = max.max(de);
        }
        out.push(orow);
    }

    let mut used: Vec<u32> = resolved.values().map(|(c, _)| *c).collect();
    used.sort_unstable();
    used.dedup();
    let mut mapping: Vec<_> = resolved.iter().map(|(rgb, (c, de))| (*rgb, *c, *de)).collect();
    mapping.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap());

    Ok((
        out,
        QuantizeReport {
            max_delta_e: max,
            mean_delta_e: if count == 0 { 0.0 } else { sum / count as f64 },
            used_colors: used,
            mapping,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The reference pairs from Sharma, Wu & Dalal's supplementary table for
    /// "The CIEDE2000 Color-Difference Formula" (2005) — the table that exists
    /// precisely because implementations of this formula get the hue-rotation
    /// term wrong. Checking against the paper is the whole reason the formula
    /// is written out here instead of imported.
    #[test]
    fn matches_the_published_reference_pairs() {
        let cases: &[([f64; 3], [f64; 3], f64)] = &[
            ([50.0000, 2.6772, -79.7751], [50.0000, 0.0000, -82.7485], 2.0425),
            ([50.0000, 3.1571, -77.2803], [50.0000, 0.0000, -82.7485], 2.8615),
            ([50.0000, 2.8361, -74.0200], [50.0000, 0.0000, -82.7485], 3.4412),
            ([50.0000, -1.3802, -84.2814], [50.0000, 0.0000, -82.7485], 1.0000),
            ([50.0000, 2.4900, -0.0010], [50.0000, -2.4900, 0.0009], 7.1792),
            ([50.0000, 2.5000, 0.0000], [73.0000, 25.0000, -18.0000], 27.1492),
            ([50.0000, 2.5000, 0.0000], [50.0000, 3.1736, 0.5854], 1.0000),
            ([60.2574, -34.0099, 36.2677], [60.4626, -34.1751, 39.4387], 1.2644),
            ([22.7233, 20.0904, -46.6940], [23.0331, 14.9730, -42.5619], 2.0373),
            ([2.0776, 0.0795, -1.1350], [0.9033, -0.0636, -0.5514], 0.9082),
        ];
        for (a, b, want) in cases {
            let got = ciede2000(*a, *b);
            assert!(
                (got - want).abs() < 1e-4,
                "CIEDE2000({a:?}, {b:?}) = {got}, the paper says {want}"
            );
        }
    }

    /// White is white: a colour already in the palette must map to itself at
    /// zero distance, or every number above it is meaningless.
    #[test]
    fn an_exact_palette_colour_costs_nothing() {
        let mut colors = std::collections::HashMap::new();
        colors.insert(15u32, [255u8, 255, 255]);
        colors.insert(4u32, [201u8, 26, 9]);
        let (cells, report) = quantize(&[vec![[255, 255, 255]]], &colors, &[15, 4]).unwrap();
        assert_eq!(cells[0][0], 15);
        assert!(report.max_delta_e < 1e-9, "got {}", report.max_delta_e);
    }
}
