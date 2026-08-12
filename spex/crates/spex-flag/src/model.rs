//! The flag specification: a declarative construction sheet, not a bitmap.
//!
//! Every flag in `flags/` is transcribed from the specification its own state
//! (or that state's recognised vexillological authority) publishes, and the
//! `source` field says which document. This is the same rule the rest of the
//! work runs on — the Ankerstein sizes, the LDraw colour table, the patent
//! dates — and here it also happens to sidestep image licensing entirely:
//! nothing is traced, so there is no image to have a licence.
//!
//! # The one coordinate convention everything else follows
//!
//! A flag is measured in **normalised flag coordinates**: `u` runs 0 at the
//! hoist to 1 at the fly, `v` runs 0 at the top to 1 at the bottom. Both are
//! fractions of that dimension, so the pair is aspect-ratio-free.
//!
//! But **widths are fractions of the HEIGHT**, always, including the widths
//! of vertical bands. That is not a convenience, it is what the real
//! construction sheets say: a Nordic cross has arms of one width, and the
//! Union Flag's vertical cross bar is the same physical width as its
//! horizontal one. Expressing a vertical band as a fraction of the flag's
//! width would make that width change when the ratio does — which is exactly
//! what the Flag Institute's single construction sheet for both 1:2 and 3:5
//! does not do. `Raster` converts by dividing by the aspect ratio in the one
//! place that needs it, and nowhere else has to know.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// One colour of a flag's own palette, as its specification states it.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FlagColor {
    /// The colour as sRGB 0..255. This is the number the quantiser measures
    /// against the real LDraw table.
    pub srgb: [u8; 3],
    /// What the specification actually says — "Pantone 186 C", "CMYK
    /// 0-94-87-0", or a note that no official value exists. Carried because
    /// an sRGB triple derived from a Pantone chip is a *conversion*, and a
    /// reader deserves to know that rather than to be handed three integers
    /// as though they were the source.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub specified_as: Option<String>,
}

/// What the flag is made of, painted in order: later elements cover earlier
/// ones. A field is just the first element every flag has.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Element {
    /// N equal vertical bands, hoist to fly.
    StripesVertical { colors: Vec<String> },
    /// N equal horizontal bands, top to bottom.
    StripesHorizontal { colors: Vec<String> },
    /// An upright cross: one horizontal arm and one vertical arm, each
    /// `arm_width_fraction` of the flag's HEIGHT wide (see the module note).
    /// The centres are given separately because a Nordic cross is offset
    /// toward the hoist and a St George cross is not.
    #[serde(rename_all = "camelCase")]
    Cross {
        color: String,
        arm_width_fraction: f64,
        /// Centre of the vertical arm, as a fraction of the WIDTH.
        center_x_fraction: f64,
        /// Centre of the horizontal arm, as a fraction of the HEIGHT.
        center_y_fraction: f64,
    },
    /// A diagonal cross, corner to corner.
    ///
    /// `offset_fraction` is what makes the Union Flag's saltire the Union
    /// Flag's saltire: the coloured band sits off the diagonal's centreline
    /// by this much, so the white showing on one side is broader than on the
    /// other. `counterchange` then flips that offset between the hoist half
    /// and the fly half, which is the whole asymmetry.
    #[serde(rename_all = "camelCase")]
    Saltire {
        color: String,
        arm_width_fraction: f64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        offset_fraction: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        counterchange: Option<Counterchange>,
    },
}

/// How a saltire's offset behaves across the flag.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Counterchange {
    /// The Union Flag's rule, and the reason this enum has a name rather than
    /// being a boolean: on the hoist half the coloured band sits *below* each
    /// diagonal's centreline, so the broad white is uppermost at the hoist;
    /// on the fly half it sits above, so the narrow white is uppermost there.
    /// The two halves are each other under a half-turn about the centre,
    /// which is the symmetry the real flag has and the reason one rule covers
    /// all four arms.
    HoistBroadAbove,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlagSpec {
    pub version: u32,
    pub iso2: String,
    pub name: String,
    /// `[height, width]`, in the units the specification states them in.
    pub ratio: [u32; 2],
    /// The document this sheet was transcribed from. Required, not optional:
    /// a flag without one is a flag somebody drew.
    pub source: String,
    pub colors: BTreeMap<String, FlagColor>,
    pub elements: Vec<Element>,
    /// Set when this flag's real construction needs something the element set
    /// cannot express. Such a flag is excluded from the Atlas and listed,
    /// rather than approximated into something that looks close.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub unsupported: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

impl FlagSpec {
    /// Width divided by height. Everything that has to turn a height fraction
    /// into a width fraction goes through this.
    pub fn aspect(&self) -> f64 {
        self.ratio[1] as f64 / self.ratio[0] as f64
    }

    pub fn color(&self, name: &str) -> anyhow::Result<[u8; 3]> {
        self.colors
            .get(name)
            .map(|c| c.srgb)
            .ok_or_else(|| anyhow::anyhow!("flag {} has no colour named {name:?}", self.iso2))
    }

    /// Structural checks, all of them at once rather than the first — the
    /// same argument `spex_show::validate` makes.
    pub fn validate(&self) -> Vec<String> {
        let mut errs = Vec::new();
        if self.version != 1 {
            errs.push(format!("version {} is not 1", self.version));
        }
        if self.ratio[0] == 0 || self.ratio[1] == 0 {
            errs.push("ratio has a zero component".into());
        }
        if self.source.trim().is_empty() {
            errs.push("source is empty — every flag must cite the document its construction comes from".into());
        }
        if self.elements.is_empty() {
            errs.push("no elements: even a plain field is one element".into());
        }
        let mut want = |name: &str, errs: &mut Vec<String>| {
            if !self.colors.contains_key(name) {
                errs.push(format!("element refers to colour {name:?}, which the palette does not define"));
            }
        };
        for el in &self.elements {
            match el {
                Element::StripesVertical { colors } | Element::StripesHorizontal { colors } => {
                    if colors.is_empty() {
                        errs.push("a stripe element with no stripes".into());
                    }
                    for c in colors {
                        want(c, &mut errs);
                    }
                }
                Element::Cross { color, arm_width_fraction, .. } => {
                    want(color, &mut errs);
                    if !(0.0..=1.0).contains(arm_width_fraction) {
                        errs.push(format!("cross armWidthFraction {arm_width_fraction} is not in 0..1"));
                    }
                }
                Element::Saltire { color, arm_width_fraction, .. } => {
                    want(color, &mut errs);
                    if !(0.0..=1.0).contains(arm_width_fraction) {
                        errs.push(format!("saltire armWidthFraction {arm_width_fraction} is not in 0..1"));
                    }
                }
            }
        }
        errs
    }
}
