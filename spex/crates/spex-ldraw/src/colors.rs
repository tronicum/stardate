//! Real LDraw color table parsing (`LDConfig.ldr`).
//!
//! Two readers over the same file, on purpose.
//!
//! [`load_colors`] returns the name+RGB tuple the point pipeline has always
//! used, and its signature does not change: `sampling.rs` and `brick.rs` both
//! *destructure* that tuple, which no accessor method could rescue. Review 01
//! (finding B3) caught a proposal to replace it and the breakage it would
//! have caused.
//!
//! [`load_colors_full`] is the mesh renderer's reader, and returns everything
//! the real file actually carries: `ALPHA`, `LUMINANCE`, and the finish
//! keywords that decide whether a brick is chrome, rubber, pearlescent or
//! glitter. Only `spex-mesh` uses it.
use crate::cache::LdrawCache;
use anyhow::Result;
use std::collections::HashMap;

/// Maps a real LDraw color code to its real name + RGB.
pub type ColorTable = HashMap<u32, (String, [u8; 3])>;

/// Parses the real, official `LDConfig.ldr` color table:
/// `0 !COLOUR <name> CODE <n> VALUE #RRGGBB EDGE #RRGGBB`.
pub fn load_colors(cache: &LdrawCache) -> Result<ColorTable> {
    Ok(load_colors_full(cache)?
        .into_iter()
        .map(|(code, c)| (code, (c.name, c.value)))
        .collect())
}

/// Everything one `!COLOUR` line really says.
#[derive(Clone, Debug, PartialEq)]
pub struct LdrawColor {
    pub code: u32,
    pub name: String,
    pub value: [u8; 3],
    pub edge: [u8; 3],
    /// Real `ALPHA`, 255 when the line omits it (i.e. opaque).
    pub alpha: u8,
    /// Real `LUMINANCE`, 0 when omitted. Non-zero only for the
    /// glow-in-the-dark colours, where it drives emission.
    pub luminance: u8,
    pub finish: Finish,
    /// The heading of the real file's own section this line sits under —
    /// `"Solid"`, `"Transparent"`, `"Modulex"`, `"Rubber"`, ... See
    /// [`parse_section_comment`] for why this is load-bearing and not
    /// bookkeeping. Empty for a line parsed outside `load_colors_full`.
    pub section: String,
}

impl LdrawColor {
    pub fn rgb(&self) -> [u8; 3] {
        self.value
    }
    pub fn is_transparent(&self) -> bool {
        self.alpha < 255
    }
}

/// The real finish keywords LDConfig uses. `MatteMetallic` is in the grammar
/// and in LDraw's own specification, but the current official file contains
/// no colour that uses it — kept because the format allows it, and noted so
/// nobody goes looking for a test that cannot exist.
#[derive(Clone, Debug, PartialEq, Default)]
pub enum Finish {
    #[default]
    Solid,
    Chrome,
    Pearlescent,
    Rubber,
    MatteMetallic,
    Metal,
    /// `MATERIAL SPECKLE VALUE #hex FRACTION f MINSIZE a MAXSIZE b`
    Speckle {
        value: [u8; 3],
        alpha: u8,
        luminance: u8,
        fraction: f64,
        min_size: f64,
        max_size: f64,
    },
    /// `MATERIAL GLITTER VALUE #hex FRACTION f VFRACTION v SIZE s`
    Glitter {
        value: [u8; 3],
        alpha: u8,
        luminance: u8,
        fraction: f64,
        vfraction: f64,
        size: f64,
    },
}

impl Finish {
    /// Short stable name, used as the manifest's own `finish` field.
    pub fn key(&self) -> &'static str {
        match self {
            Finish::Solid => "solid",
            Finish::Chrome => "chrome",
            Finish::Pearlescent => "pearlescent",
            Finish::Rubber => "rubber",
            Finish::MatteMetallic => "matte_metallic",
            Finish::Metal => "metal",
            Finish::Speckle { .. } => "speckle",
            Finish::Glitter { .. } => "glitter",
        }
    }
}

/// Parses every real field of every real `!COLOUR` line.
pub fn load_colors_full(cache: &LdrawCache) -> Result<HashMap<u32, LdrawColor>> {
    let text = cache.fetch("LDConfig.ldr")?;
    let mut colors = HashMap::new();
    let mut section = String::new();
    for line in text.lines() {
        if let Some(name) = parse_section_comment(line) {
            section = name;
            continue;
        }
        if let Some(mut color) = parse_colour_line(line) {
            color.section = section.clone();
            colors.insert(color.code, color);
        }
    }
    Ok(colors)
}

/// The real file's own section headings: `0 // LDraw Solid Colours`,
/// `0 // LDraw Modulex Colours`, and a dozen more.
///
/// M75 is why this is parsed. `Finish` cannot tell a LEGO brick colour from a
/// Modulex one — Modulex entries are plain `!COLOUR` lines with no material
/// keyword, so they are `Finish::Solid` and opaque like any brick. The first
/// Belgian flag built here came out in `30006 Modulex_Ochre_Yellow`, which is
/// an architectural modelling block from a different product line at a
/// different scale: a colour nobody can build a flag out of, chosen because
/// it happened to be the nearest in Lab. The section heading is the only
/// thing in the real file that says so, so it is now carried.
fn parse_section_comment(line: &str) -> Option<String> {
    let rest = line.trim().strip_prefix("0 //")?.trim();
    // Every heading ends in " Colours" and nothing in the file's prose
    // preamble does — checked against the real file rather than assumed.
    let name = rest.strip_suffix(" Colours")?;
    let name = name.strip_prefix("LDraw ").unwrap_or(name).trim();
    (!name.is_empty()).then(|| name.to_string())
}

/// Splits the line at `MATERIAL` **before** looking for anything else.
///
/// This is not tidiness, it is correctness: `VALUE`, `ALPHA` and `LUMINANCE`
/// all appear on *both* sides of that token in the real file — e.g.
/// `... EDGE #B9275F ALPHA 128 MATERIAL GLITTER VALUE #B92790 FRACTION 0.17 ...`
/// — so a whole-line `find_after("VALUE")` reads the speckle's colour as the
/// brick's whenever the order differs, and every speckle and glitter colour
/// parses wrong. Review 01, finding B3.
fn parse_colour_line(line: &str) -> Option<LdrawColor> {
    let tokens: Vec<&str> = line.split_whitespace().collect();
    if tokens.len() < 8 || tokens.get(1) != Some(&"!COLOUR") {
        return None;
    }
    let split = tokens.iter().position(|t| *t == "MATERIAL");
    let (head, tail) = match split {
        Some(i) => (&tokens[..i], &tokens[i + 1..]),
        None => (&tokens[..], &[][..]),
    };

    let name = head[2].to_string();
    let code = find_after(head, "CODE")?.parse::<u32>().ok()?;
    let value = parse_hex_rgb(find_after(head, "VALUE")?)?;
    // EDGE is not universally present in third-party configs; falling back to
    // the colour itself is better than dropping the whole line.
    let edge = find_after(head, "EDGE").and_then(parse_hex_rgb).unwrap_or(value);
    let alpha = find_after(head, "ALPHA").and_then(|s| s.parse().ok()).unwrap_or(255u8);
    let luminance = find_after(head, "LUMINANCE").and_then(|s| s.parse().ok()).unwrap_or(0u8);

    let finish = if tail.is_empty() {
        // A bare finish keyword sits at the end of the line, after EDGE and
        // any ALPHA/LUMINANCE. Matching on the *tokens* and not on the line
        // matters: "Chrome_Silver" and "Rubber_Black" are colour *names*, and
        // a substring search would classify half the file by its label.
        head.iter()
            .find_map(|t| match *t {
                "CHROME" => Some(Finish::Chrome),
                "PEARLESCENT" => Some(Finish::Pearlescent),
                "RUBBER" => Some(Finish::Rubber),
                "MATTE_METALLIC" => Some(Finish::MatteMetallic),
                "METAL" => Some(Finish::Metal),
                _ => None,
            })
            .unwrap_or_default()
    } else {
        parse_material(tail)?
    };

    Some(LdrawColor { code, name, value, edge, alpha, luminance, finish, section: String::new() })
}

fn parse_material(tail: &[&str]) -> Option<Finish> {
    let value = parse_hex_rgb(find_after(tail, "VALUE")?)?;
    let alpha = find_after(tail, "ALPHA").and_then(|s| s.parse().ok()).unwrap_or(255u8);
    let luminance = find_after(tail, "LUMINANCE").and_then(|s| s.parse().ok()).unwrap_or(0u8);
    let fraction = find_after(tail, "FRACTION").and_then(|s| s.parse().ok()).unwrap_or(0.0);
    match tail.first().copied() {
        Some("SPECKLE") => Some(Finish::Speckle {
            value,
            alpha,
            luminance,
            fraction,
            min_size: find_after(tail, "MINSIZE").and_then(|s| s.parse().ok()).unwrap_or(1.0),
            max_size: find_after(tail, "MAXSIZE").and_then(|s| s.parse().ok()).unwrap_or(1.0),
        }),
        Some("GLITTER") => Some(Finish::Glitter {
            value,
            alpha,
            luminance,
            fraction,
            vfraction: find_after(tail, "VFRACTION").and_then(|s| s.parse().ok()).unwrap_or(0.0),
            size: find_after(tail, "SIZE").and_then(|s| s.parse().ok()).unwrap_or(1.0),
        }),
        _ => None,
    }
}

fn find_after<'a>(tokens: &[&'a str], key: &str) -> Option<&'a str> {
    tokens.iter().position(|t| *t == key).and_then(|i| tokens.get(i + 1)).copied()
}

fn parse_hex_rgb(hex: &str) -> Option<[u8; 3]> {
    let hex = hex.strip_prefix('#').unwrap_or(hex);
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some([r, g, b])
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Real lines, copied verbatim out of the official `LDConfig.ldr`.
    const REAL_LINES: &str = "\
0 LDraw.org Configuration File
0 // some comment line, not a color
0 !COLOUR Black                       CODE   0 VALUE #1B2A34 EDGE #808080
0 !COLOUR Red                         CODE   4 VALUE #B40000 EDGE #333333
0 !COLOUR Trans_Clear                 CODE  47 VALUE #FCFCFC EDGE #C9C9C9 ALPHA 128
0 !COLOUR Chrome_Silver               CODE 383 VALUE #CECECE EDGE #9C9C9C CHROME
0 !COLOUR Rubber_Black                CODE 256 VALUE #1B2A34 EDGE #808080 RUBBER
0 !COLOUR Pearl_Black                 CODE  83 VALUE #0A1327 EDGE #333333 PEARLESCENT
0 !COLOUR Metallic_Silver             CODE  80 VALUE #767676 EDGE #333333 METAL
0 !COLOUR Glow_In_Dark_Opaque         CODE  21 VALUE #E0FFB0 EDGE #B8FF4D ALPHA 245 LUMINANCE 15
0 !COLOUR Speckle_Black_Copper        CODE  75 VALUE #000000 EDGE #AB6038 MATERIAL SPECKLE VALUE #AB6038 FRACTION 0.4 MINSIZE 1 MAXSIZE 3
0 !COLOUR Glitter_Trans_Dark_Pink     CODE 114 VALUE #DF6695 EDGE #B9275F ALPHA 128 MATERIAL GLITTER VALUE #B92790 FRACTION 0.17 VFRACTION 0.2 SIZE 1
";

    fn table() -> HashMap<u32, LdrawColor> {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("LDConfig.ldr"), REAL_LINES).unwrap();
        load_colors_full(&LdrawCache::new(dir.path())).unwrap()
    }

    #[test]
    fn parses_real_ldconfig_lines_and_ignores_others() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("LDConfig.ldr"), REAL_LINES).unwrap();
        let colors = load_colors(&LdrawCache::new(dir.path())).unwrap();
        assert_eq!(colors.len(), 10);
        assert_eq!(colors[&0], ("Black".to_string(), [0x1B, 0x2A, 0x34]));
        assert_eq!(colors[&4], ("Red".to_string(), [0xB4, 0x00, 0x00]));
    }

    #[test]
    fn the_tuple_reader_still_sees_exactly_what_it_always_did() {
        // The point pipeline destructures this tuple in two places
        // (sampling.rs, brick.rs). If this ever stops compiling as a 2-tuple,
        // those break — which is the whole reason `load_colors` was not
        // replaced. Review 01, B3.
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("LDConfig.ldr"), REAL_LINES).unwrap();
        let colors = load_colors(&LdrawCache::new(dir.path())).unwrap();
        let (name, rgb) = &colors[&383];
        assert_eq!(name, "Chrome_Silver");
        assert_eq!(*rgb, [0xCE, 0xCE, 0xCE]);
    }

    #[test]
    fn bare_finish_keywords_parse() {
        let t = table();
        assert_eq!(t[&383].finish, Finish::Chrome);
        assert_eq!(t[&256].finish, Finish::Rubber);
        assert_eq!(t[&83].finish, Finish::Pearlescent);
        assert_eq!(t[&80].finish, Finish::Metal);
        assert_eq!(t[&0].finish, Finish::Solid);
        assert_eq!(t[&4].finish, Finish::Solid);
    }

    #[test]
    fn a_colour_named_after_a_finish_is_not_classified_by_its_name() {
        // "Chrome_Silver" really is chrome, but "Rubber_Black" being rubber
        // must come from the trailing RUBBER token, not from the label. This
        // line has a chrome-sounding name and no finish keyword at all.
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("LDConfig.ldr"),
            "0 !COLOUR Chrome_Look_Alike CODE 900 VALUE #CECECE EDGE #9C9C9C\n",
        )
        .unwrap();
        let t = load_colors_full(&LdrawCache::new(dir.path())).unwrap();
        assert_eq!(t[&900].finish, Finish::Solid);
    }

    #[test]
    fn alpha_and_luminance_are_read_and_default_correctly() {
        let t = table();
        assert_eq!(t[&47].alpha, 128, "Trans_Clear is really ALPHA 128");
        assert!(t[&47].is_transparent());
        assert_eq!(t[&0].alpha, 255, "an opaque colour omits ALPHA");
        assert!(!t[&0].is_transparent());
        assert_eq!(t[&21].luminance, 15, "Glow_In_Dark_Opaque is really LUMINANCE 15");
        assert_eq!(t[&21].alpha, 245);
        assert_eq!(t[&0].luminance, 0);
    }

    #[test]
    fn a_speckle_line_takes_its_own_value_not_the_bricks() {
        // This is B3 in one assertion: the brick is #000000 and the speckle is
        // #AB6038, and both are written as `VALUE`. Splitting at MATERIAL first
        // is what keeps them apart.
        let t = table();
        assert_eq!(t[&75].value, [0x00, 0x00, 0x00]);
        assert_eq!(
            t[&75].finish,
            Finish::Speckle {
                value: [0xAB, 0x60, 0x38],
                alpha: 255,
                luminance: 0,
                fraction: 0.4,
                min_size: 1.0,
                max_size: 3.0,
            }
        );
    }

    #[test]
    fn a_glitter_line_keeps_the_bricks_own_alpha() {
        // ALPHA 128 appears *before* MATERIAL here and belongs to the brick.
        let t = table();
        assert_eq!(t[&114].alpha, 128);
        assert_eq!(t[&114].value, [0xDF, 0x66, 0x95]);
        assert_eq!(
            t[&114].finish,
            Finish::Glitter {
                value: [0xB9, 0x27, 0x90],
                alpha: 255,
                luminance: 0,
                fraction: 0.17,
                vfraction: 0.2,
                size: 1.0,
            }
        );
    }

    #[test]
    #[ignore = "real live network fetch against ldraw.org, not run by default"]
    fn real_live_fetch_of_ldconfig_works() {
        let dir = tempfile::tempdir().unwrap();
        let cache = LdrawCache::new(dir.path());
        let colors = load_colors(&cache).unwrap();
        assert!(colors.len() > 100, "expected the real official color table to have 100+ colors, got {}", colors.len());
        assert_eq!(colors[&0].0, "Black");
        assert_eq!(colors[&4].0, "Red");
    }

    #[test]
    #[ignore = "reads the real LDConfig.ldr, network on a cold cache"]
    fn the_real_ldconfig_yields_at_least_five_distinct_finishes() {
        // M56 AC1, against the real official file.
        // The workspace-root cache, not "wherever cargo happened to start" —
        // a relative path here silently creates a second cache inside the
        // crate directory and re-downloads the whole file.
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join(".ldraw-cache");
        let cache = LdrawCache::new(&root);
        let colors = load_colors_full(&cache).unwrap();
        let kinds: std::collections::BTreeSet<&str> =
            colors.values().map(|c| c.finish.key()).collect();
        assert!(
            kinds.len() >= 6,
            "expected >= 6 distinct finishes in the real file, got {kinds:?}"
        );
        assert_eq!(colors[&0].finish, Finish::Solid, "Black");
        assert_eq!(colors[&4].finish, Finish::Solid, "Red");
        assert_eq!(colors[&47].alpha, 128, "Trans_Clear");
        assert_eq!(colors[&383].finish, Finish::Chrome, "Chrome_Silver");
        // MATTE_METALLIC is in the grammar but no colour in the current
        // official file uses it — asserted so the absence is a recorded fact
        // rather than a suspected parser bug.
        assert!(
            !colors.values().any(|c| c.finish == Finish::MatteMetallic),
            "the real file gained a MATTE_METALLIC colour — update the note in Finish"
        );
    }
}
