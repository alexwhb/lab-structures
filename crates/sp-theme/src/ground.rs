//! Ground stage colours — `Ground.dc.html` inline SVG, soil swatches and readouts. Owned by the
//! `worlds` agent. (Bar marker and goal-box tokens are in [`crate::tower`].)

use crate::{hex, rgba};
use egui::Color32;

/// A soil's swatch (mockup `defs().soil`): the block body `color`, the surface strip `top`
/// and the picture tile's 2.5D `side`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SoilSwatch {
    /// Body (`color`).
    pub body: Color32,
    /// Surface (`top`).
    pub top: Color32,
    /// Tile side (`side`, the tile swatch's `0 4px 0` shadow).
    pub side: Color32,
}

/// Soil swatches in `sp_engine::ground::Soil::ALL` order: dense sand `#8C7A55 / #A39066 /
/// #4E4430`, stiff clay `#7A5A48 / #8F6C58 / #463328`, soft clay `#5E5546 / #716654 /
/// #332E26`, loose wet sand `#9A8A62 / #B0A074 / #564D36`.
pub const SOILS: [SoilSwatch; 4] = [
    SoilSwatch {
        body: hex(0x8C7A55),
        top: hex(0xA39066),
        side: hex(0x4E4430),
    },
    SoilSwatch {
        body: hex(0x7A5A48),
        top: hex(0x8F6C58),
        side: hex(0x463328),
    },
    SoilSwatch {
        body: hex(0x5E5546),
        top: hex(0x716654),
        side: hex(0x332E26),
    },
    SoilSwatch {
        body: hex(0x9A8A62),
        top: hex(0xB0A074),
        side: hex(0x564D36),
    },
];

/// The swatch's inset bottom shadow `inset 0 -3px 0 rgba(0,0,0,.25)`.
pub const SWATCH_INSET: Color32 = rgba(0, 0, 0, 0.25);
/// The swatch's drop shadow `0 8px 10px rgba(0,0,0,.6)`.
pub const SWATCH_SHADOW: Color32 = rgba(0, 0, 0, 0.6);

/// The topsoil band `M0 260 H900 V278 H0 Z` `#3A3226` at `fill-opacity: 0.8`.
pub const TOPSOIL: Color32 = rgba(0x3A, 0x32, 0x26, 0.8);
/// Soil grains: black at 0.18.
pub const GRAIN: Color32 = rgba(0, 0, 0, 0.18);
/// Bedrock `M0 530 H900 V600 H0 Z` `#2A2C30`; its cracks `#45484E` (1.5).
pub const BEDROCK: Color32 = hex(0x2A2C30);
/// See [`BEDROCK`].
pub const BEDROCK_CRACK: Color32 = hex(0x45484E);
/// The water table `#4CC3F0` (1.5, dashes 10 8, flowing).
pub const WATER: Color32 = hex(0x4CC3F0);
/// Sand boils `#C9B98A`.
pub const BOIL: Color32 = hex(0xC9B98A);
/// Foundations `#6E7178` with a `#9A9DA4` edge.
pub const FOUNDATION: Color32 = hex(0x6E7178);
/// See [`FOUNDATION`].
pub const FOUNDATION_EDGE: Color32 = hex(0x9A9DA4);
/// Building side `#1B1D21`.
pub const BUILDING_SIDE: Color32 = hex(0x1B1D21);
/// Building front `#2E323A`, edge `#15171A` (0.8).
pub const BUILDING_FRONT: Color32 = hex(0x2E323A);
/// See [`BUILDING_FRONT`].
pub const BUILDING_EDGE: Color32 = hex(0x15171A);
/// Window bands: the accent at `stroke-opacity: 0.35` (width 2).
pub const WINDOWS: Color32 = rgba(214, 199, 161, 0.35);
/// The roof `#3C414A`.
pub const ROOF: Color32 = hex(0x3C414A);
/// The plumb line `M450 236 V40`: the accent, width 1, dashes 3 5.
pub const PLUMB: Color32 = crate::ACCENT;
/// The pressure bulb `#E0905E` at `stroke-opacity: 0.7` (1.5, dashes 4 4).
pub const BULB: Color32 = rgba(0xE0, 0x90, 0x5E, 0.7);
/// The legend's bulb swatch `2px dashed #E0905E`.
pub const BULB_LEGEND: Color32 = hex(0xE0905E);
/// The soil name on the stage `#E5E0D0` (mono 11).
pub const SOIL_LABEL: Color32 = hex(0xE5E0D0);
/// The legend box `rgba(10,10,10,.82)` with a `#242424` border.
pub const LEGEND: Color32 = rgba(10, 10, 10, 0.82);
/// The foundation tile icons' ground line `#6A6A6A`.
pub const ICON_GROUND: Color32 = hex(0x6A6A6A);

/// Safety factor bar bands (`flex: 1 2 2`): `#5C2A25` `#4A4320` `#1F4A33`.
pub const SAFETY_BANDS: [Color32; 3] = [hex(0x5C2A25), hex(0x4A4320), hex(0x1F4A33)];

#[cfg(test)]
mod tests {
    use super::*;
    use lab_theme::contrast::{over, ratio};

    #[test]
    fn labels_read_on_every_soil() {
        for s in SOILS {
            let r = ratio(SOIL_LABEL, s.body);
            assert!(r >= 1.9, "soil label on {:?}: {r:.2}", s.body);
        }
        // The muted "bedrock" label on the bedrock.
        assert!(ratio(crate::TEXT_MUTED, BEDROCK) >= 4.5);
        // The legend text on its box over the stage.
        assert!(ratio(crate::TEXT_DIM, over(LEGEND, crate::GRAPH.base)) >= 4.5);
    }
}
