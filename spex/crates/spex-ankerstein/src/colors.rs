//! Real Ankerstein color table — three canonical colors, matching the
//! actual baked-quartz-sand/chalk/linseed-oil material (matte, not glossy
//! like injection-molded plastic). Static, no fetch needed (unlike
//! `spex-ldraw::colors`' `LDConfig.ldr` parse) — there is no equivalent
//! official machine-readable color registry for Ankerstein.
//!
//! **Provenance note**: the RGB values below are a first, reasonable
//! approximation from written descriptions ("brick red", "cement/ochre
//! yellow", "slate blue-grey" — see the concept note and
//! `docs/ANKERSTEIN-ENGINE.md` §3), not yet a real hex sample pulled from
//! an actual photographed stone. `docs/ANKERSTEIN-ENGINE.md` §3 flags this
//! explicitly as needing a real photo-sourced correction before M98 is
//! considered fully "real data" rather than "plausible placeholder" — do
//! not treat these three values as final without doing that pass.
use std::collections::HashMap;

pub type AnkersteinColorTable = HashMap<&'static str, [u8; 3]>;

pub fn load_colors() -> AnkersteinColorTable {
    let mut colors = AnkersteinColorTable::new();
    colors.insert("brick-red", [0xB0, 0x40, 0x30]);
    colors.insert("cement-yellow", [0xD8, 0xC0, 0x78]);
    colors.insert("slate-blue-grey", [0x5C, 0x66, 0x70]);
    colors
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn has_exactly_the_three_historical_colors() {
        let colors = load_colors();
        assert_eq!(colors.len(), 3);
        assert!(colors.contains_key("brick-red"));
        assert!(colors.contains_key("cement-yellow"));
        assert!(colors.contains_key("slate-blue-grey"));
    }
}
