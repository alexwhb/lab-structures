//! v2 builder stage and panel colours — `docs/design/v2/mockup/Builder.dc.html` (its inline SVG
//! in `renderVals()` order and the overlay CSS). Owned by the `screen` agent: every token names
//! its CSS source.
//!
//! The force colours, the neutrals and the sand ramp live in the crate root
//! ([`crate::TENSION`], [`crate::OVERLAY`], [`crate::ACCENT`], …); this module only adds what
//! the builder's stage and aside draw on top of them.

use crate::{hex, rgba};
use egui::Color32;

// ============================================================================================
// The stage (`<svg viewBox="0 0 1120 600">`, background `#0C0D0F`)

/// Stage background `background: #0C0D0F`.
pub const STAGE: Color32 = hex(0x0C0D0F);
/// Snap-grid dots `<circle r="1.3" fill="#34373C">` (the `#sgrid` pattern).
pub const GRID_DOT: Color32 = hex(0x34373C);
/// River `waterD fill="#0B1A24"`.
pub const WATER: Color32 = hex(0x0B1A24);
/// River dashes `flowD stroke="#24546E" stroke-width="2" stroke-dasharray="18 30"`.
pub const WATER_FLOW: Color32 = hex(0x24546E);
/// Grains in the soil `grainD fill="#000000" fill-opacity="0.22"`.
pub const GRAIN: Color32 = rgba(0, 0, 0, 0.22);
/// The ground's edge line `edgeD stroke="#3A3A3E" stroke-width="3"`.
pub const GROUND_EDGE: Color32 = hex(0x3A3A3E);
/// Foundations `foundD fill="#6E7178" stroke="#9A9DA4" stroke-width="1"`.
pub const FOUNDATION: Color32 = hex(0x6E7178);
pub const FOUNDATION_EDGE: Color32 = hex(0x9A9DA4);
/// The joint marker over a foundation `supD fill="#8A8F98"` (also the crate wire).
pub const SUPPORT: Color32 = hex(0x8A8F98);
/// Wind streaks `windD stroke="#4CC3F0" stroke-opacity="0.5" stroke-width="1.5"
/// stroke-dasharray="20 28"`.
pub const WIND_STREAK: Color32 = rgba(76, 195, 240, 0.5);
/// Wind arrows `windArrowD stroke="#4CC3F0" stroke-width="2"`.
pub const WIND_ARROW: Color32 = hex(0x4CC3F0);
/// The line pieces' back copies `backD stroke="#303338" stroke-width="3"`.
pub const BACK_FACE: Color32 = hex(0x303338);
/// The ghost piece `ghostD stroke="#D6C7A1" stroke-opacity="0.55" stroke-width="2"
/// stroke-dasharray="6 6"`.
pub const GHOST_LINE: Color32 = rgba(214, 199, 161, 0.55);
/// Floor façades `facadeD fill="#1D2A36" fill-opacity="0.78" stroke="#2C3B4A"
/// stroke-width="0.8"`.
pub const FACADE: Color32 = rgba(0x1D, 0x2A, 0x36, 0.78);
pub const FACADE_EDGE: Color32 = hex(0x2C3B4A);
/// The façade's side face `facadeSideD fill="#15202A" fill-opacity="0.85"`.
pub const FACADE_SIDE: Color32 = rgba(0x15, 0x20, 0x2A, 0.85);
/// The window line `winD stroke="#D6C7A1" stroke-opacity="0.28" stroke-width="1.2"`.
pub const WINDOW_LINE: Color32 = rgba(214, 199, 161, 0.28);
/// Deck slabs' top face `roadTopD fill="#46474C"`.
pub const DECK_TOP: Color32 = hex(0x46474C);
/// Deck slabs' front `roadFrontD fill="#2A2B2F" stroke="#1C1D20" stroke-width="0.8"`.
pub const DECK_FRONT: Color32 = hex(0x2A2B2F);
pub const DECK_EDGE: Color32 = hex(0x1C1D20);
/// Snow on exposed decks `snowD fill="#EEF3F6" fill-opacity="0.92"`.
pub const SNOW: Color32 = rgba(0xEE, 0xF3, 0xF6, 0.92);
/// The road's lane line `roadLineD stroke="#D6C7A1" stroke-opacity="0.35" stroke-width="1"
/// stroke-dasharray="6 8"`.
pub const LANE_LINE: Color32 = rgba(214, 199, 161, 0.35);
/// Falling snow `flakeD fill="#F2F6F8" fill-opacity="0.75"` (r 1.6).
pub const FLAKE: Color32 = rgba(0xF2, 0xF6, 0xF8, 0.75);
/// The level's target height `targetD stroke="#D6C7A1" stroke-opacity="0.6"
/// stroke-width="1.5" stroke-dasharray="10 8"`.
pub const TARGET_LINE: Color32 = rgba(214, 199, 161, 0.6);
/// A vehicle's splash `splashD stroke="#C9E2F0" stroke-width="2"`.
pub const SPLASH: Color32 = hex(0xC9E2F0);
/// The hover dot `hoverD fill="#D6C7A1" fill-opacity="0.6"` (r 5).
pub const HOVER_DOT: Color32 = rgba(214, 199, 161, 0.6);

/// Force bands (`t1 … x` paths, stroke 2.5; `x` 3.5 dashed 9 6): `t1`/`t2` `#7FB0E0` at .5 /
/// .8, `t3` `#9CC6F0`; `c1`/`c2` `#E0905E` at .5 / .8, `c3` `#F0A878`; `z` `#6A6A6A`.
pub const TENSION_HOT: Color32 = hex(0x9CC6F0);
pub const COMPRESSION_HOT: Color32 = hex(0xF0A878);
pub const FORCE_IDLE: Color32 = hex(0x6A6A6A);
/// `stroke-opacity` of bands 1 and 2 (band 3 is opaque).
pub const BAND_OPACITY: [f32; 3] = [0.5, 0.8, 1.0];

// ============================================================================================
// Pieces (`types()`: `sw`, the palette swatch and the line colour on the stage)

/// A piece type's look: the swatch (palette chip bar, Materials curve, line colour), the line
/// width on the stage and whether its caps are round (`stroke-linecap`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PieceLook {
    pub swatch: Color32,
    pub width: f32,
    pub round: bool,
    /// The palette chip's swatch bar height (`types()[k].h`, at least 3).
    pub bar_h: f32,
}

/// Road `sw: '#46474C', h: 7` (a deck: drawn as a slab).
pub const ROAD: PieceLook = PieceLook {
    swatch: hex(0x46474C),
    width: 0.0,
    round: false,
    bar_h: 7.0,
};
/// Floor `sw: '#3A4A5C', h: 7` (a deck with a façade).
pub const FLOOR: PieceLook = PieceLook {
    swatch: hex(0x3A4A5C),
    width: 0.0,
    round: false,
    bar_h: 7.0,
};
/// Wood `sw: '#8A6A43', h: 6`; `woodD stroke-width="6" stroke-linecap="round"`.
pub const WOOD: PieceLook = PieceLook {
    swatch: hex(0x8A6A43),
    width: 6.0,
    round: true,
    bar_h: 6.0,
};
/// Steel `sw: '#7E858F', h: 4`; `steelD stroke-width="4.5"` round.
pub const STEEL: PieceLook = PieceLook {
    swatch: hex(0x7E858F),
    width: 4.5,
    round: true,
    bar_h: 4.0,
};
/// Concrete `sw: '#9C9A92', h: 7`; `concreteD stroke-width="7" stroke-linecap="butt"`.
pub const CONCRETE: PieceLook = PieceLook {
    swatch: hex(0x9C9A92),
    width: 7.0,
    round: false,
    bar_h: 7.0,
};
/// Rope `sw: '#B59A6A', h: 2` (bar ≥ 3); `ropeD stroke-width="2.4"` round.
pub const ROPE: PieceLook = PieceLook {
    swatch: hex(0xB59A6A),
    width: 2.4,
    round: true,
    bar_h: 3.0,
};
/// Cable `sw: '#C9CED6', h: 2` (bar ≥ 3); `cableD stroke-width="2.2"` round.
pub const CABLE: PieceLook = PieceLook {
    swatch: hex(0xC9CED6),
    width: 2.2,
    round: true,
    bar_h: 3.0,
};
/// Every piece in `types()` order: Road, Floor, Wood, Steel, Concrete, Rope, Cable.
pub const PIECES: [PieceLook; 7] = [ROAD, FLOOR, WOOD, STEEL, CONCRETE, ROPE, CABLE];

// ============================================================================================
// Soils (`soils()`: `color` the body, `top` the top face and the Conditions chip dot)

/// A soil's body and top-face colours.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SoilLook {
    pub body: Color32,
    pub top: Color32,
}
/// Every soil in `soils()` order: Rock `#45474B`/`#55575C`, Dense sand `#7E6E4C`/`#958360`,
/// Stiff clay `#6E5141`/`#836251`, Soft clay `#544C3F`/`#665C4C`, Loose wet sand
/// `#8A7C58`/`#9F906A`.
pub const SOILS: [SoilLook; 5] = [
    SoilLook {
        body: hex(0x45474B),
        top: hex(0x55575C),
    },
    SoilLook {
        body: hex(0x7E6E4C),
        top: hex(0x958360),
    },
    SoilLook {
        body: hex(0x6E5141),
        top: hex(0x836251),
    },
    SoilLook {
        body: hex(0x544C3F),
        top: hex(0x665C4C),
    },
    SoilLook {
        body: hex(0x8A7C58),
        top: hex(0x9F906A),
    },
];

// ============================================================================================
// Crates and vehicles

/// Crates: wire `crateWireD stroke="#8A8F98" stroke-width="1.2"`, side `#5A4630`, top
/// `#9C7E55`, front `#7E6442` with a `#3E3022` stroke.
pub const CRATE_WIRE: Color32 = SUPPORT;
pub const CRATE_SIDE: Color32 = hex(0x5A4630);
pub const CRATE_TOP: Color32 = hex(0x9C7E55);
pub const CRATE: Color32 = hex(0x7E6442);
pub const CRATE_EDGE: Color32 = hex(0x3E3022);
/// Car body `vCarD fill="#B9C0CA"`.
pub const CAR: Color32 = hex(0xB9C0CA);
/// Bus body `vBusD fill="#C9B98F"`.
pub const BUS: Color32 = hex(0xC9B98F);
/// Truck box `vBoxD fill="#5A5F68"`, cab `vCabD fill="#A89A78"`.
pub const TRUCK_BOX: Color32 = hex(0x5A5F68);
pub const TRUCK_CAB: Color32 = hex(0xA89A78);
/// Vehicle outlines `stroke="#0C0D0F" stroke-width="1"` (= [`STAGE`]).
pub const VEHICLE_EDGE: Color32 = STAGE;
/// Vehicle windows `vWinD fill="#1B2733"`.
pub const VEHICLE_WINDOW: Color32 = hex(0x1B2733);
/// Wheels `vWheelD fill="#111214" stroke="#6A6E75" stroke-width="1.2"`.
pub const WHEEL: Color32 = hex(0x111214);
pub const WHEEL_EDGE: Color32 = hex(0x6A6E75);

// ============================================================================================
// The stage's HTML layer (handles, joints, tags, HUD, toast, overlays)

/// Piece handle `.mh` fill `rgba(12,13,15,.9)` (border `#3A3A3A`, text `#D9D9D9`).
pub const HANDLE: Color32 = rgba(12, 13, 15, 0.9);
/// Crate tag `.ltag` fill `rgba(12,13,15,.85)` (text [`crate::LOAD_TAG`] `#E6D6B8`).
pub const TAG: Color32 = rgba(12, 13, 15, 0.85);
/// Joint dot `.jdot` ring `#D4D4D4` (fill = [`STAGE`]).
pub const JOINT_RING: Color32 = hex(0xD4D4D4);
/// `.j-pend .jdot` glow `box-shadow: 0 0 0 5px rgba(214,199,161,.25)`.
pub const JOINT_PENDING_GLOW: Color32 = rgba(214, 199, 161, 0.25);
/// `.j-sel .jdot` halo `box-shadow: 0 0 0 4px rgba(233,223,198,.35)`.
pub const JOINT_SELECTED_HALO: Color32 = rgba(233, 223, 198, 0.35);
/// `.j-bad .jdot` halo `box-shadow: 0 0 0 4px rgba(229,83,75,.3)`.
pub const JOINT_BAD_HALO: Color32 = rgba(229, 83, 75, 0.3);
/// HUD pills `.pill` with `background: rgba(10,10,10,.85)` (border `#2E2E2E`).
pub const HUD: Color32 = rgba(10, 10, 10, 0.85);
/// HUD values: wind `#8FD3EE`, snow `#E6EEF3`, shaking `#E7A47C` (= [`crate::CAUTION`]).
pub const HUD_WIND: Color32 = hex(0x8FD3EE);
pub const HUD_SNOW: Color32 = hex(0xE6EEF3);
/// Result banner `background: rgba(10,10,10,.94)` (border `#2B5A3C` / `#5C2A25`).
pub const BANNER: Color32 = rgba(10, 10, 10, 0.94);
/// Hint text and the zoom percentage `color: #BDBDBD`.
pub const HINT_TEXT: Color32 = hex(0xBDBDBD);
/// `.chip:hover` / `.lvl:hover` border `#3A3A3A`.
pub const CHIP_HOVER_BORDER: Color32 = hex(0x3A3A3A);
/// The Materials curve glyph: `background: #0D0D0D; border: 1px solid #242424`, axes
/// `stroke="#2E2E2E"`.
pub const GLYPH_AXES: Color32 = hex(0x2E2E2E);
/// Legend "Slack / idle" swatch `background: #5E5E5E` (= [`crate::ZERO_FORCE`]).
pub const LEGEND_IDLE: Color32 = crate::ZERO_FORCE;

#[cfg(test)]
mod tests {
    use super::*;
    use lab_theme::contrast::{over, ratio};

    #[test]
    fn values_match_the_css() {
        assert_eq!(STAGE, Color32::from_rgb(0x0C, 0x0D, 0x0F));
        assert_eq!(GRID_DOT, Color32::from_rgb(0x34, 0x37, 0x3C));
        assert_eq!(HANDLE.a(), rgba(0, 0, 0, 0.9).a());
        assert_eq!(PIECES[3].swatch, Color32::from_rgb(0x7E, 0x85, 0x8F));
        assert_eq!(SOILS[1].top, Color32::from_rgb(0x95, 0x83, 0x60));
    }

    /// Stage graphics reach 3:1 against the stage (LOOK-4): force colours, the support marker,
    /// joint rings, the accent; HUD values, tags, handle text and the hint 4.5:1 on their fills.
    #[test]
    fn stage_contrast() {
        for (name, c) in [
            ("tension", crate::TENSION),
            ("tension hot", TENSION_HOT),
            ("compression", crate::COMPRESSION),
            ("compression hot", COMPRESSION_HOT),
            ("over", crate::OVER),
            ("support", SUPPORT),
            ("joint ring", JOINT_RING),
            ("accent", crate::ACCENT),
            ("wind arrow", WIND_ARROW),
        ] {
            let r = ratio(c, STAGE);
            assert!(r >= 3.0, "{name}: {r:.2}");
        }
        let hud = over(HUD, STAGE);
        for (name, c) in [
            ("hud key", crate::TEXT_MUTED),
            ("hud value", crate::TEXT),
            ("hud ok", crate::OK),
            ("hud danger", crate::DANGER_INK),
            ("hud wind", HUD_WIND),
            ("hud snow", HUD_SNOW),
            ("hud shaking", crate::CAUTION),
        ] {
            let r = ratio(c, hud);
            assert!(r >= 4.5, "{name}: {r:.2}");
        }
        let tag = over(TAG, STAGE);
        assert!(ratio(crate::LOAD_TAG, tag) >= 4.5);
        let handle = over(HANDLE, STAGE);
        assert!(ratio(crate::TEXT_DIM, handle) >= 4.5);
        assert!(ratio(crate::DANGER_SOFT, handle) >= 4.5);
        let panel = over(crate::PANEL, crate::PAPER.base);
        assert!(ratio(HINT_TEXT, panel) >= 4.5);
        assert!(ratio(HINT_TEXT, crate::PAPER.base) >= 4.5);
    }
}
