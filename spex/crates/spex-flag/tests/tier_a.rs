//! The two acceptance criteria M75 names, against the real sheets in `flags/`.
//!
//! Both are asserted in **studs**, on the rasterised mosaic, not on the
//! declarative numbers. Checking that `armWidthFraction` equals 4/28 would
//! only be checking that the JSON says what the JSON says; the question worth
//! asking is whether the thing that comes out the other end of `rasterize`
//! has the arm the regulation describes.
use spex_flag::{rasterize, FlagSpec};

fn load(iso2: &str) -> FlagSpec {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../flags")
        .join(format!("{iso2}.json"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let spec: FlagSpec = serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    assert!(spec.validate().is_empty(), "{iso2}: {:?}", spec.validate());
    spec
}

/// A run-length view of one line of cells: `(colour, length)` in order.
fn runs(line: impl Iterator<Item = [u8; 3]>) -> Vec<([u8; 3], usize)> {
    let mut out: Vec<([u8; 3], usize)> = Vec::new();
    for c in line {
        match out.last_mut() {
            Some((prev, n)) if *prev == c => *n += 1,
            _ => out.push((c, 1)),
        }
    }
    out
}

/// **AC1.** `spex flag DK --width-studs 48` produces a cross whose arm width
/// and offset, measured in studs, match the published fractions to within one
/// stud.
///
/// The published sheet (FKOBST L.202-4, and the 1748 regulation's "1/7 of the
/// flag's height") is horizontally 12/4/21 and vertically 12/4/12 of a 28:37
/// flag. At 48 studs wide the flag is 36 tall, so:
///
///   arm width      4/28 * 36 = 5.14 studs
///   hoist field    12/37 * 48 = 15.57 studs
///   upper field    12/28 * 36 = 15.43 studs
#[test]
fn ac1_dannebrog_cross_matches_the_published_fractions() {
    let spec = load("dk");
    let cells = rasterize(&spec, 48).unwrap();
    let (h, w) = (cells.len(), cells[0].len());
    assert_eq!((h, w), (36, 48), "28:37 at 48 studs wide");

    let red = spec.color("red").unwrap();
    let white = spec.color("white").unwrap();

    // Along the top row only the vertical arm can reach.
    let top = runs(cells[0].iter().copied());
    assert_eq!(top.len(), 3, "hoist field, upright, fly field: {top:?}");
    assert_eq!(top[0].0, red);
    assert_eq!(top[1].0, white);
    assert_eq!(top[2].0, red);
    let within_one = |got: usize, want: f64, what: &str| {
        assert!(
            (got as f64 - want).abs() <= 1.0,
            "{what}: {got} studs against the published {want:.2}"
        );
    };
    within_one(top[0].1, 12.0 / 37.0 * 48.0, "hoist field width");
    within_one(top[1].1, 4.0 / 28.0 * 36.0, "upright width");
    within_one(top[2].1, 21.0 / 37.0 * 48.0, "fly field width");

    // Down the hoist column only the horizontal arm can reach.
    let hoist = runs(cells.iter().map(|r| r[0]));
    assert_eq!(hoist.len(), 3, "upper field, arm, lower field: {hoist:?}");
    within_one(hoist[0].1, 12.0 / 28.0 * 36.0, "upper field height");
    within_one(hoist[1].1, 4.0 / 28.0 * 36.0, "arm height");
    within_one(hoist[2].1, 12.0 / 28.0 * 36.0, "lower field height");

    // The arm is as wide as it is thick — the property the whole "widths are
    // fractions of the height" convention exists for. At 48 studs it holds
    // only to within a stud, and that is a fact about the grid rather than
    // about the flag: the true arm is 4/28 * 36 = 5.14 studs, and the two
    // directions round it differently because the cell centres fall
    // differently against 12/37 of the width and 12/28 of the height. 5 and 6
    // are both 5.14 to the nearest buildable thing.
    assert!(
        (top[1].1 as i64 - hoist[1].1 as i64).abs() <= 1,
        "upright {} vs arm thickness {} — more than a stud apart",
        top[1].1,
        hoist[1].1
    );
}

/// The same flag at a width its own construction actually divides.
///
/// The Dannebrog is 12/4/21 by 12/4/12, so 37 studs across and 28 down puts
/// every boundary exactly on a stud line and the arm is 4 studs both ways with
/// no rounding at all. This is the test that shows the one-stud tolerance
/// above is the grid's and not the code's — and it is worth knowing that a
/// flag has natural widths, because 48 is not one of the Dannebrog's.
#[test]
fn ac1_at_a_natural_width_the_dannebrog_needs_no_tolerance() {
    let spec = load("dk");
    let cells = rasterize(&spec, 37).unwrap();
    assert_eq!((cells.len(), cells[0].len()), (28, 37));
    let white = spec.color("white").unwrap();
    let top = runs(cells[0].iter().copied());
    let hoist = runs(cells.iter().map(|r| r[0]));
    assert_eq!(
        (top[0].1, top[1].1, top[2].1),
        (12, 4, 21),
        "horizontally 12/4/21, exactly as the sheet says"
    );
    assert_eq!(
        (hoist[0].1, hoist[1].1, hoist[2].1),
        (12, 4, 12),
        "vertically 12/4/12, exactly as the sheet says"
    );
    assert_eq!(top[1].0, white);
    assert_eq!(top[1].1, hoist[1].1, "4 studs both ways, no rounding");
}

/// **AC2.** The Union Flag's asymmetric saltire: the broad white diagonal is
/// uppermost at the hoist and the narrow one is uppermost at the fly.
///
/// Read down a column near the hoist and a column near the fly, and compare
/// the white runs on either side of the red one. Colour, not geometry, is the
/// subject here — this runs before quantisation on purpose, so a failure means
/// the construction is wrong rather than that a brick was.
#[test]
fn ac2_union_flag_saltire_counterchanges() {
    let spec = load("gb");
    let cells = rasterize(&spec, 96).unwrap();
    let (h, w) = (cells.len(), cells[0].len());
    assert_eq!((h, w), (48, 96), "1:2 at 96 studs wide");

    let blue = spec.color("blue").unwrap();
    let white = spec.color("white").unwrap();
    let red = spec.color("red").unwrap();

    // A column 1/8 in from the hoist, and its mirror 1/8 in from the fly.
    // Both are clear of the central cross and of the corners.
    let column = |col: usize| runs(cells.iter().map(|r| r[col]));

    for (col, side, broad_first) in [(w / 8, "hoist", true), (w - w / 8 - 1, "fly", false)] {
        let r = column(col);
        // Top half: blue, white, red, white, blue — the upper-left arm. Then
        // the lower arm mirrors it. Take the first five runs.
        assert!(r.len() >= 5, "{side} column has only {} runs: {r:?}", r.len());
        assert_eq!(r[0].0, blue, "{side}: starts in the field");
        assert_eq!(r[1].0, white, "{side}: white before red");
        assert_eq!(r[2].0, red, "{side}: the St Patrick stripe");
        assert_eq!(r[3].0, white, "{side}: white after red");
        let (above, below) = (r[1].1, r[3].1);
        if broad_first {
            assert!(
                above > below,
                "at the hoist the broad white must be uppermost, got {above} above and {below} below"
            );
        } else {
            assert!(
                below > above,
                "at the fly the narrow white must be uppermost, got {above} above and {below} below"
            );
        }
        // Published: white 3 units, red 2, white 1, of 30 height units.
        let unit = h as f64 / 30.0;
        let (broad, narrow) = if broad_first { (above, below) } else { (below, above) };
        assert!((broad as f64 - 3.0 * unit).abs() <= 1.0, "{side}: broad white {broad} vs 3 units = {:.2}", 3.0 * unit);
        assert!((narrow as f64 - 1.0 * unit).abs() <= 1.0, "{side}: narrow white {narrow} vs 1 unit = {:.2}", unit);
        assert!((r[2].1 as f64 - 2.0 * unit).abs() <= 1.0, "{side}: red {} vs 2 units = {:.2}", r[2].1, 2.0 * unit);
    }
}

/// **AC4.** Every shipped sheet cites the document its construction comes from.
/// Cheap, and it is the rule the whole `flags/` directory exists under.
#[test]
fn ac4_every_sheet_cites_its_source() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../flags");
    let mut seen = 0;
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let spec: FlagSpec = serde_json::from_str(&std::fs::read_to_string(&path).unwrap())
            .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        assert!(
            spec.source.len() > 40,
            "{}: a one-line source is not a citation",
            path.display()
        );
        assert!(spec.validate().is_empty(), "{}: {:?}", path.display(), spec.validate());
        seen += 1;
    }
    assert_eq!(seen, 3, "Tier A is three flags: BE, DK, GB");
}
