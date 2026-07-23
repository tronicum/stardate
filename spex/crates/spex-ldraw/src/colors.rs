//! Real LDraw color table parsing (`LDConfig.ldr`).
use crate::cache::LdrawCache;
use anyhow::Result;
use std::collections::HashMap;

/// Maps a real LDraw color code to its real name + RGB.
pub type ColorTable = HashMap<u32, (String, [u8; 3])>;

/// Parses the real, official `LDConfig.ldr` color table:
/// `0 !COLOUR <name> CODE <n> VALUE #RRGGBB EDGE #RRGGBB`.
pub fn load_colors(cache: &LdrawCache) -> Result<ColorTable> {
    let text = cache.fetch("LDConfig.ldr")?;
    let mut colors = ColorTable::new();
    for line in text.lines() {
        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens.len() < 8 || tokens.get(1) != Some(&"!COLOUR") {
            continue;
        }
        let name = tokens[2].to_string();
        let Some(code) = find_after(&tokens, "CODE").and_then(|s| s.parse::<u32>().ok()) else {
            continue;
        };
        let Some(hex) = find_after(&tokens, "VALUE") else {
            continue;
        };
        let Some(rgb) = parse_hex_rgb(hex) else {
            continue;
        };
        colors.insert(code, (name, rgb));
    }
    Ok(colors)
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

    #[test]
    fn parses_real_ldconfig_lines_and_ignores_others() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("LDConfig.ldr"),
            "0 LDraw.org Configuration File\n\
             0 // some comment line, not a color\n\
             0 !COLOUR Black CODE 0 VALUE #1B2A34 EDGE #595959\n\
             0 !COLOUR Red CODE 4 VALUE #C91A09 EDGE #595959\n",
        )
        .unwrap();
        let cache = LdrawCache::new(dir.path());
        let colors = load_colors(&cache).unwrap();
        assert_eq!(colors.len(), 2);
        assert_eq!(colors[&0], ("Black".to_string(), [0x1B, 0x2A, 0x34]));
        assert_eq!(colors[&4], ("Red".to_string(), [0xC9, 0x1A, 0x09]));
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
}
