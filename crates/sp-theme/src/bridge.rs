//! Bridge (Playground) stage colours — `Playground.dc.html` inline SVG and its overlay CSS.
//! Owned by the `bridge` agent: every token names its CSS source.
//!
//! The force colours (tension / compression / zero / no solution / over) and the generic
//! neutrals live in the crate root ([`crate::TENSION`], [`crate::OVERLAY`], …).

use crate::{hex, rgba};
use egui::Color32;

/// Stage background `#0C0D0F`; the snap-grid dots `#3A3D42` (r 1.3, every 36 px = 2 m).
pub const STAGE: Color32 = hex(0x0C0D0F);
pub const GRID_DOT: Color32 = hex(0x3A3D42);
/// River `#0B1A24`, its flowing dashes `#24546E`.
pub const WATER: Color32 = hex(0x0B1A24);
pub const WATER_FLOW: Color32 = hex(0x24546E);
/// Banks: top face `#2B2721`, body and riverbed `#1A1713`.
pub const BANK_TOP: Color32 = hex(0x2B2721);
pub const BANK: Color32 = hex(0x1A1713);
/// The members' back face (the 2.5D offset copy, +18, −12 px) `#34373C`.
pub const BACK_FACE: Color32 = hex(0x34373C);
/// Supports and the load cable `#8A8F98`.
pub const SUPPORT: Color32 = hex(0x8A8F98);
/// Crates: front `#7E6442` (stroke `#3E3022`), top `#9C7E55`, side `#5A4630`.
pub const CRATE: Color32 = hex(0x7E6442);
pub const CRATE_STROKE: Color32 = hex(0x3E3022);
pub const CRATE_TOP: Color32 = hex(0x9C7E55);
pub const CRATE_SIDE: Color32 = hex(0x5A4630);
/// Joint dot: fill = [`STAGE`] (`.jdot` `#0C0D0F`), ring `#D4D4D4`.
pub const JOINT_RING: Color32 = hex(0xD4D4D4);
/// `.j-pend .jdot` glow `box-shadow: 0 0 0 5px rgba(214,199,161,.25)`.
pub const JOINT_PENDING_GLOW: Color32 = rgba(214, 199, 161, 0.25);
/// `.j-sel .jdot` halo `box-shadow: 0 0 0 4px rgba(233,223,198,.35)`.
pub const JOINT_SELECTED_HALO: Color32 = rgba(233, 223, 198, 0.35);
/// `.j-bad .jdot` halo `box-shadow: 0 0 0 4px rgba(229,83,75,.3)`.
pub const JOINT_BAD_HALO: Color32 = rgba(229, 83, 75, 0.3);
/// Member handle `.mh` fill `rgba(12,13,15,.9)` (border `#3A3A3A`, text `#D9D9D9`).
pub const HANDLE: Color32 = rgba(12, 13, 15, 0.9);
/// Load tag `.ltag` fill `rgba(12,13,15,.85)` (text [`crate::LOAD_TAG`]).
pub const TAG: Color32 = rgba(12, 13, 15, 0.85);
/// Result banner fill `rgba(10,10,10,.94)`.
pub const BANNER: Color32 = rgba(10, 10, 10, 0.94);
/// Legend fill `rgba(10,10,10,.82)` (border `#242424`).
pub const LEGEND: Color32 = rgba(10, 10, 10, 0.82);
/// The ghost member: `stroke="#D6C7A1" stroke-opacity="0.55"` dashed 6 6.
pub const GHOST_LINE: Color32 = rgba(214, 199, 161, 0.55);
/// The hover dot: `fill="#D6C7A1" fill-opacity="0.6"`.
pub const HOVER_DOT: Color32 = rgba(214, 199, 161, 0.6);
/// `.chip:hover` / `.lvl:hover` border `#3A3A3A`.
pub const CHIP_HOVER_BORDER: Color32 = hex(0x3A3A3A);

#[cfg(test)]
mod tests {
    use super::*;
    use lab_theme::contrast::{over, ratio};

    #[test]
    fn values_match_the_css() {
        assert_eq!(STAGE, Color32::from_rgb(0x0C, 0x0D, 0x0F));
        assert_eq!(HANDLE.a(), crate::rgba(0, 0, 0, 0.9).a());
        assert_eq!(GHOST_LINE.a(), crate::rgba(0, 0, 0, 0.55).a());
    }

    /// Stage graphics reach 3:1 against the stage (LOOK-4): members, supports, joint rings,
    /// the danger ring; tags and handle text 4.5:1 on their fills.
    #[test]
    fn stage_contrast() {
        for (name, c) in [
            ("tension", crate::TENSION),
            ("compression", crate::COMPRESSION),
            ("over", crate::OVER),
            ("no solution", crate::NO_FORCE),
            ("support", SUPPORT),
            ("joint ring", JOINT_RING),
            ("accent", crate::ACCENT),
        ] {
            let r = ratio(c, STAGE);
            assert!(r >= 3.0, "{name}: {r:.2}");
        }
        let tag = over(TAG, STAGE);
        assert!(ratio(crate::LOAD_TAG, tag) >= 4.5);
        let handle = over(HANDLE, STAGE);
        assert!(ratio(crate::TEXT_DIM, handle) >= 4.5);
        assert!(ratio(crate::DANGER_SOFT, handle) >= 4.5);
    }
}
