//! Tower stage colours — `Tower.dc.html` inline SVG and readouts. Owned by the `worlds` agent.
//! Also the controls the Tower and Ground columns share (Chrome's checkbox, the comfort /
//! safety band colours, the challenge goal boxes).

use crate::{hex, rgba};
use egui::Color32;

// ============================================================================================
// Stage (`<svg viewBox="0 0 900 620">`)

/// The ground strip `M0 580 H900 V620 H0 Z` `#17150F`.
pub const GROUND: Color32 = hex(0x17150F);
/// The ground line `M0 580 H900` `#3A352A` (width 2).
pub const GROUND_LINE: Color32 = hex(0x3A352A);
/// The plinth under the tower `M392 580 L400 600 H560 L552 580 Z` `#26231C`.
pub const PLINTH: Color32 = hex(0x26231C);
/// Wind arrows `#4CC3F0` (width 2.2) — the info colour.
pub const WIND: Color32 = hex(0x4CC3F0);
/// Wind streaks `#4CC3F0` at `stroke-opacity: 0.35` (width 1.5, dashes 26 34).
pub const STREAK: Color32 = rgba(76, 195, 240, 0.35);
/// Tower side faces `#1B1D21`, edge `#101114` (0.6).
pub const SIDE: Color32 = hex(0x1B1D21);
/// See [`SIDE`].
pub const SIDE_EDGE: Color32 = hex(0x101114);
/// Tower front faces `#2E323A`, edge `#15171A` (0.7).
pub const FRONT: Color32 = hex(0x2E323A);
/// See [`FRONT`].
pub const FRONT_EDGE: Color32 = hex(0x15171A);
/// The hollow core's sky slots `#0A0A0A`.
pub const HOLE: Color32 = hex(0x0A0A0A);
/// System lines (core walls, tube columns, braces) `#8A93A0` at `stroke-opacity: 0.85`
/// (width 1.3); outriggers use the accent.
pub const SYSTEM: Color32 = rgba(138, 147, 160, 0.85);
/// Outrigger system lines: the accent `#D6C7A1` at 0.85.
pub const SYSTEM_OUTRIGGER: Color32 = rgba(214, 199, 161, 0.85);
/// The roof cap `#3C414A`.
pub const CAP: Color32 = hex(0x3C414A);
/// The plumb line: the accent, width 1, dashes 3 5.
pub const PLUMB: Color32 = crate::ACCENT;
/// The tuned mass damper: accent disc (r 6) with a `#0A0A0A` 1.5 outline.
pub const DAMPER: Color32 = crate::ACCENT;
/// See [`DAMPER`].
pub const DAMPER_EDGE: Color32 = hex(0x0A0A0A);
/// "Wind at the top" box: `rgba(10,10,10,.82)` with a `#242424` border (radius 8).
pub const STAGE_BOX: Color32 = rgba(10, 10, 10, 0.82);

// ============================================================================================
// Readouts

/// Occupant comfort bar bands (`flex: 5 5 10 10`): `#1F4A33` `#4A4320` `#5A3A1C` `#5C2A25`.
pub const COMFORT_BANDS: [Color32; 4] = [
    hex(0x1F4A33),
    hex(0x4A4320),
    hex(0x5A3A1C),
    hex(0x5C2A25),
];
/// The comfort / safety bar marker `#F5F5F5` (4 × 18, radius 2).
pub const BAR_MARKER: Color32 = hex(0xF5F5F5);
/// Unchecked challenge goal box border `#5A5A5A` (18 × 18, radius 5, 1.5 px).
pub const GOAL_OPEN: Color32 = hex(0x5A5A5A);

// ============================================================================================
// Chrome's `input[type=checkbox]` with `accent-color: #D6C7A1` (Tower damper, Ground quake)

/// Unchecked fill `#FFFFFF` (sampled from the reference render).
pub const CHECKBOX: Color32 = hex(0xFFFFFF);
/// Unchecked border `#767676` (1 px, radius 2; sampled).
pub const CHECKBOX_BORDER: Color32 = hex(0x767676);
/// Checked: the accent fill with a dark tick (Chrome picks a dark tick on a light accent).
pub const CHECKBOX_ON: Color32 = crate::ACCENT;
/// See [`CHECKBOX_ON`].
pub const CHECKBOX_TICK: Color32 = hex(0x161616);

#[cfg(test)]
mod tests {
    use super::*;
    use lab_theme::contrast::ratio;

    #[test]
    fn stage_graphics_are_visible() {
        let stage = crate::GRAPH.base;
        // The wind arrows and plumb line read against the stage; the tower front reads against
        // its edges.
        assert!(ratio(WIND, stage) >= 3.0);
        assert!(ratio(PLUMB, stage) >= 3.0);
        assert!(ratio(DAMPER, FRONT) >= 3.0);
        assert!(ratio(CHECKBOX_TICK, CHECKBOX_ON) >= 3.0);
        assert!(ratio(CHECKBOX_BORDER, CHECKBOX) >= 3.0);
    }
}
