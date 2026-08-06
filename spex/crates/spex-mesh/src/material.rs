//! LDraw finishes → PBR parameters.
//!
//! **Every number in this file is a calibrated artistic choice, not a
//! measurement.** They come from the technical-art review in
//! `docs/FUGEN-ENGINE-REVIEW-01.md`, which corrected the first-draft table in
//! `docs/fugen/phase1-renderer.md`, and they are recorded here — with the
//! reason each one is what it is — precisely so nobody later mistakes them
//! for physics and "fixes" them against a spec sheet.
//!
//! They live in Rust rather than in the viewer for one reason: the bundle
//! should say what a material *is*, so that a second renderer (an offline
//! path tracer, the WASM path in phase 6, a still-image export) resolves the
//! same brick to the same look without re-deriving the table.

use serde::Serialize;
use spex_ldraw::{Finish, LdrawColor};

use crate::bundle::srgb_to_linear;

/// Opaque ABS. Not 0.28 (the first draft) — ABS has a distinct skin layer,
/// which is modelled by the clearcoat below, and 0.34 under a clearcoat reads
/// closer than 0.28 without one.
const ABS_ROUGHNESS: f32 = 0.34;
/// Black reads *only* by its specular highlight: its diffuse term is nearly
/// zero, so at the general roughness a black brick is a flat silhouette.
const BLACK_ROUGHNESS: f32 = 0.22;
const LDRAW_BLACK: u32 = 0;
/// The ABS skin layer. Small, but it is what stops a brick looking like
/// painted chalk.
const ABS_CLEARCOAT: f32 = 0.15;
const ABS_CLEARCOAT_ROUGHNESS: f32 = 0.25;
/// Real polycarbonate/ABS transparent parts. `ior` 1.53 is the one number
/// here that *is* a measurement — it is polycarbonate's.
const TRANS_ROUGHNESS: f32 = 0.10;
const TRANS_TRANSMISSION: f32 = 0.85;
const TRANS_IOR: f32 = 1.53;
/// The alpha LDraw actually uses for a *properly* transparent part — every
/// Trans_* colour in the official file is 128. It is the normalisation point:
/// a colour at this alpha gets the full calibrated glass treatment above, and
/// anything less transparent gets proportionally less of it.
const CANONICAL_TRANS_ALPHA: f32 = 128.0 / 255.0;
/// Rubber tyres: almost fully diffuse.
const RUBBER_ROUGHNESS: f32 = 0.92;
/// Pearlescent is **not** a metal — modelling it as one (the first draft said
/// metalness 0.35) kills the diffuse term that gives it its colour. It is a
/// dielectric with a thin-film layer, which is exactly what three.js's
/// `iridescence` is.
const PEARL_ROUGHNESS: f32 = 0.42;
const PEARL_IRIDESCENCE: f32 = 0.4;
const PEARL_IRIDESCENCE_IOR: f32 = 1.8;
/// Matte metallic at 0.8 metalness is the classic in-between that reads as
/// neither metal nor plastic. Full metal, high roughness.
const MATTE_METALLIC_ROUGHNESS: f32 = 0.62;
const METAL_ROUGHNESS: f32 = 0.35;
/// Chrome at 0.03 gives one hard environment dot and reads as shiny plastic;
/// 0.06 spreads it just enough to read as metal.
const CHROME_ROUGHNESS: f32 = 0.06;
/// Speckle and glitter keep a plastic base — the particles are a separate
/// term, carried through in `speckle` below.
const SPECKLED_ROUGHNESS: f32 = 0.30;

/// The particle layer of a `MATERIAL SPECKLE` / `MATERIAL GLITTER` colour.
///
/// Carried through the manifest in full even though M56 renders only the base
/// material: the procedural noise chunk belongs with the dissolve shader work,
/// and a renderer that has it needs these exact numbers rather than a guess.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SpeckleParams {
    /// Linear rgb, like every other colour in the bundle.
    pub color: [f32; 3],
    /// Fraction of the surface covered by particles, straight from the file.
    pub fraction: f64,
    /// SPECKLE only.
    #[serde(rename = "minSize", skip_serializing_if = "Option::is_none")]
    pub min_size: Option<f64>,
    #[serde(rename = "maxSize", skip_serializing_if = "Option::is_none")]
    pub max_size: Option<f64>,
    /// GLITTER only — volume fraction and particle size.
    #[serde(rename = "vFraction", skip_serializing_if = "Option::is_none")]
    pub vfraction: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<f64>,
}

/// One LDraw colour, resolved to what a renderer needs.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PbrMaterial {
    pub metalness: f32,
    pub roughness: f32,
    /// 0..1. Below 1 means the part is really transparent in the file.
    pub opacity: f32,
    pub clearcoat: f32,
    #[serde(rename = "clearcoatRoughness")]
    pub clearcoat_roughness: f32,
    pub transmission: f32,
    pub ior: f32,
    pub iridescence: f32,
    #[serde(rename = "iridescenceIOR")]
    pub iridescence_ior: f32,
    /// Real `LUMINANCE` scaled to 0..1 — non-zero only for glow-in-the-dark.
    #[serde(rename = "emissiveIntensity")]
    pub emissive_intensity: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speckle: Option<SpeckleParams>,
}

impl Default for PbrMaterial {
    fn default() -> Self {
        Self {
            metalness: 0.0,
            roughness: ABS_ROUGHNESS,
            opacity: 1.0,
            clearcoat: ABS_CLEARCOAT,
            clearcoat_roughness: ABS_CLEARCOAT_ROUGHNESS,
            transmission: 0.0,
            ior: 1.5,
            iridescence: 0.0,
            iridescence_ior: 1.3,
            emissive_intensity: 0.0,
            speckle: None,
        }
    }
}

fn linear(rgb: [u8; 3]) -> [f32; 3] {
    [srgb_to_linear(rgb[0]), srgb_to_linear(rgb[1]), srgb_to_linear(rgb[2])]
}

/// Resolves one real LDraw colour to PBR parameters.
///
/// Order matters: a colour can be both transparent *and* glitter (most of the
/// glitter colours are), so transparency is applied on top of the finish
/// rather than as a competing branch.
pub fn from_ldraw(color: &LdrawColor) -> PbrMaterial {
    let mut m = PbrMaterial::default();

    match &color.finish {
        Finish::Solid => {
            if color.code == LDRAW_BLACK {
                m.roughness = BLACK_ROUGHNESS;
            }
        }
        Finish::Chrome => {
            m.metalness = 1.0;
            m.roughness = CHROME_ROUGHNESS;
            m.clearcoat = 0.0;
        }
        Finish::Metal => {
            m.metalness = 1.0;
            m.roughness = METAL_ROUGHNESS;
            m.clearcoat = 0.0;
        }
        Finish::MatteMetallic => {
            m.metalness = 1.0;
            m.roughness = MATTE_METALLIC_ROUGHNESS;
            m.clearcoat = 0.0;
        }
        Finish::Rubber => {
            m.roughness = RUBBER_ROUGHNESS;
            m.clearcoat = 0.0;
        }
        Finish::Pearlescent => {
            m.roughness = PEARL_ROUGHNESS;
            m.iridescence = PEARL_IRIDESCENCE;
            m.iridescence_ior = PEARL_IRIDESCENCE_IOR;
        }
        Finish::Speckle { value, fraction, min_size, max_size, .. } => {
            m.roughness = SPECKLED_ROUGHNESS;
            m.speckle = Some(SpeckleParams {
                color: linear(*value),
                fraction: *fraction,
                min_size: Some(*min_size),
                max_size: Some(*max_size),
                vfraction: None,
                size: None,
            });
        }
        Finish::Glitter { value, fraction, vfraction, size, .. } => {
            m.roughness = SPECKLED_ROUGHNESS;
            m.speckle = Some(SpeckleParams {
                color: linear(*value),
                fraction: *fraction,
                min_size: None,
                max_size: None,
                vfraction: Some(*vfraction),
                size: Some(*size),
            });
        }
    }

    if color.is_transparent() {
        m.opacity = color.alpha as f32 / 255.0;
        // How glassy, on a scale where LDraw's own transparent alpha of 128 is
        // fully glass. Not a step: the real file uses ALPHA for two different
        // things. `Trans_Clear` is 128 and really is glass; `Glow_In_Dark_
        // Opaque` is 245 — it says "opaque" in its own name and is a brick
        // with a trace of translucency. Treating both as glass gave the glow
        // bricks transmission 0.85 and roughness 0.10, i.e. a lump of resin.
        let glassiness = ((1.0 - m.opacity) / (1.0 - CANONICAL_TRANS_ALPHA)).clamp(0.0, 1.0);
        let lerp = |a: f32, b: f32| a + (b - a) * glassiness;
        m.transmission = TRANS_TRANSMISSION * glassiness;
        m.ior = lerp(m.ior, TRANS_IOR);
        // A transparent brick reads *because* you see its own tubes through
        // it, so it must not also be mirror-smooth; but it is far smoother
        // than an opaque one. Chrome and metal are never transparent in the
        // real file, so there is no finish this can contradict.
        m.roughness = lerp(m.roughness, TRANS_ROUGHNESS);
        m.clearcoat = lerp(m.clearcoat, 0.0);
    }

    if color.luminance > 0 {
        m.emissive_intensity = color.luminance as f32 / 255.0;
    }

    m
}

#[cfg(test)]
mod tests {
    use super::*;

    fn color(code: u32, finish: Finish, alpha: u8, luminance: u8) -> LdrawColor {
        LdrawColor {
            code,
            name: format!("test-{code}"),
            value: [200, 100, 50],
            edge: [50, 50, 50],
            alpha,
            luminance,
            finish,
        }
    }

    #[test]
    fn opaque_abs_gets_a_clearcoat_and_black_gets_its_own_roughness() {
        let red = from_ldraw(&color(4, Finish::Solid, 255, 0));
        assert_eq!(red.roughness, ABS_ROUGHNESS);
        assert_eq!(red.clearcoat, ABS_CLEARCOAT);
        assert_eq!(red.metalness, 0.0);

        let black = from_ldraw(&color(0, Finish::Solid, 255, 0));
        assert_eq!(black.roughness, BLACK_ROUGHNESS);
        assert!(black.roughness < red.roughness, "black must be glossier, it reads only by specular");
    }

    #[test]
    fn chrome_is_a_smooth_metal_with_no_clearcoat() {
        let m = from_ldraw(&color(383, Finish::Chrome, 255, 0));
        assert_eq!(m.metalness, 1.0);
        assert_eq!(m.roughness, CHROME_ROUGHNESS);
        assert_eq!(m.clearcoat, 0.0, "a clearcoat over chrome is a second specular lobe on a mirror");
    }

    #[test]
    fn pearlescent_is_not_a_metal() {
        // The first-draft table said metalness 0.35, which removes the diffuse
        // term that gives a pearl colour its colour.
        let m = from_ldraw(&color(83, Finish::Pearlescent, 255, 0));
        assert_eq!(m.metalness, 0.0);
        assert_eq!(m.iridescence, PEARL_IRIDESCENCE);
        assert_eq!(m.iridescence_ior, PEARL_IRIDESCENCE_IOR);
    }

    #[test]
    fn rubber_is_nearly_fully_diffuse() {
        let m = from_ldraw(&color(256, Finish::Rubber, 255, 0));
        assert_eq!(m.roughness, RUBBER_ROUGHNESS);
        assert_eq!(m.metalness, 0.0);
        assert_eq!(m.clearcoat, 0.0);
    }

    #[test]
    fn transparency_is_applied_on_top_of_the_finish_not_instead_of_it() {
        // Most real glitter colours are transparent too, so the two have to
        // compose rather than compete.
        let m = from_ldraw(&color(
            114,
            Finish::Glitter {
                value: [0xB9, 0x27, 0x90],
                alpha: 255,
                luminance: 0,
                fraction: 0.17,
                vfraction: 0.2,
                size: 1.0,
            },
            128,
            0,
        ));
        assert!(m.speckle.is_some(), "the glitter layer survives");
        assert_eq!(m.transmission, TRANS_TRANSMISSION);
        assert_eq!(m.ior, TRANS_IOR);
        assert!((m.opacity - 128.0 / 255.0).abs() < 1e-6);
        assert!((m.roughness - TRANS_ROUGHNESS).abs() < 1e-5);
    }

    #[test]
    fn a_barely_translucent_colour_is_not_treated_as_glass() {
        // Glow_In_Dark_Opaque is really ALPHA 245. It says "opaque" in its own
        // name; giving it transmission 0.85 and roughness 0.10 turned a brick
        // into a lump of resin. Glassiness is proportional to how transparent
        // the colour actually is, normalised at LDraw's own 128.
        let glow = from_ldraw(&color(21, Finish::Solid, 245, 15));
        assert!(glow.transmission < 0.1, "got {}", glow.transmission);
        assert!(glow.roughness > 0.3, "still reads as ABS, got {}", glow.roughness);
        assert!(glow.clearcoat > 0.1, "keeps its skin layer, got {}", glow.clearcoat);

        let glass = from_ldraw(&color(47, Finish::Solid, 128, 0));
        assert!((glass.transmission - TRANS_TRANSMISSION).abs() < 1e-6, "128 is the full treatment");
        assert_eq!(glass.clearcoat, 0.0);
    }

    #[test]
    fn a_speckle_carries_its_own_linear_colour_and_real_sizes() {
        let m = from_ldraw(&color(
            75,
            Finish::Speckle {
                value: [0xAB, 0x60, 0x38],
                alpha: 255,
                luminance: 0,
                fraction: 0.4,
                min_size: 1.0,
                max_size: 3.0,
            },
            255,
            0,
        ));
        let s = m.speckle.unwrap();
        assert_eq!(s.fraction, 0.4);
        assert_eq!(s.min_size, Some(1.0));
        assert_eq!(s.max_size, Some(3.0));
        assert_eq!(s.vfraction, None, "SPECKLE has no VFRACTION");
        // Linear, not sRGB — 0xAB/255 = 0.671 in sRGB is 0.404 linear.
        assert!((s.color[0] - 0.4072).abs() < 0.01, "got {}", s.color[0]);
    }

    #[test]
    fn luminance_becomes_emission_and_zero_stays_zero() {
        let glow = from_ldraw(&color(21, Finish::Solid, 245, 15));
        assert!((glow.emissive_intensity - 15.0 / 255.0).abs() < 1e-6);
        let plain = from_ldraw(&color(4, Finish::Solid, 255, 0));
        assert_eq!(plain.emissive_intensity, 0.0);
    }

    #[test]
    fn an_opaque_colour_never_gets_transmission() {
        for f in [Finish::Solid, Finish::Chrome, Finish::Metal, Finish::Rubber, Finish::Pearlescent] {
            let m = from_ldraw(&color(1, f.clone(), 255, 0));
            assert_eq!(m.transmission, 0.0, "{f:?} is opaque");
            assert_eq!(m.opacity, 1.0, "{f:?} is opaque");
        }
    }
}
