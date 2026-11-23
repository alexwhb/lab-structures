//! Time and comfort fold-ins (CMF-1/2, TIME-1/3): the comfort meter's bands, its marker and
//! limit tick, the four-shape bars, the fast-forward's HUD values and the stage's plumb line.
//! Owned by the `time` agent. Each token names its source: the v1 `docs/design/mockup/
//! Tower.dc.html` (occupant comfort bar, "Same storm, four shapes", the stage's plumb line)
//! and `Ground.dc.html` (the years slider), drawn in the v2 builder's look
//! (`docs/design/v2/mockup/Builder.dc.html`).

use crate::{hex, rgba};
use egui::Color32;

/// Comfort meter bands, left to right — v1 Tower "Occupant comfort" bar `<span style="flex: 5;
/// background: #1F4A33">` … `#4A4320` (5), `#5A3A1C` (10), `#5C2A25` (10).
pub const COMFORT_BANDS: [Color32; 4] =
    [hex(0x1F4A33), hex(0x4A4320), hex(0x5A3A1C), hex(0x5C2A25)];
/// The meter's marker — v1 Tower `width: 4px; height: 18px; border-radius: 2px; background:
/// #F5F5F5`.
pub const METER_MARKER: Color32 = hex(0xF5F5F5);
/// The level's comfort limit tick and label — the builder HUD's over-limit comfort colour
/// (`hud.push({ k: 'Comfort ', … c: L.comfort && mg > L.comfort ? '#E8A39E' : '#EAEAEA' })`).
pub const LIMIT: Color32 = hex(0xE8A39E);

/// Comfort / tone colours — v1 Tower `comfortColor`: nobody feels it `#5FBF8A`.
pub const TONE_OK: Color32 = hex(0x5FBF8A);
/// Some people notice — `#D6C7A1` (the accent).
pub const TONE_MARGINAL: Color32 = hex(0xD6C7A1);
/// Lamps swing — `#E7A47C`.
pub const TONE_CAUTION: Color32 = hex(0xE7A47C);
/// Seasick / over the limit — `#E8A39E`.
pub const TONE_ALERT: Color32 = hex(0xE8A39E);
/// Fell / toppled — the builder's "will break" `#E5534B`.
pub const TONE_DANGER: Color32 = hex(0xE5534B);

/// The fast-forward's plumb line through the foundations' centroid — v1 Tower stage
/// `stroke="#D6C7A1" stroke-width="1" stroke-dasharray="3 5"`.
pub const PLUMB: Color32 = hex(0xD6C7A1);
/// The plumb line's dashes, px (dash, gap).
pub const PLUMB_DASH: [f32; 2] = [3.0, 5.0];
/// The lean line from the foundations' centroid to the top floor's centroid — the accent ink
/// `#E9DFC6` at 0.9 (so it reads against the plumb line).
pub const LEAN: Color32 = rgba(233, 223, 198, 0.9);

/// HUD chip values — the builder HUD's plain `#EAEAEA`.
pub const HUD_VALUE: Color32 = hex(0xEAEAEA);
/// A tilt past the goal (0.5°) in the HUD — the builder's "Stands, but strained" `#D8B86A`.
pub const HUD_TILT_WARN: Color32 = hex(0xD8B86A);
/// A tilt near toppling (≥ 4°) in the HUD — the over-limit `#E8A39E`.
pub const HUD_TILT_DANGER: Color32 = hex(0xE8A39E);

#[cfg(test)]
mod tests {
    use super::*;
    use lab_theme::contrast::ratio;

    #[test]
    fn text_reads_on_the_well_and_the_hud() {
        // the meter's value and sentence sit on the aside's `.well` (#0D0D0D); HUD chips on
        // rgba(10,10,10,.85) over the stage (≈ #0A0A0A)
        for c in [
            TONE_OK,
            TONE_MARGINAL,
            TONE_CAUTION,
            TONE_ALERT,
            TONE_DANGER,
            LIMIT,
        ] {
            assert!(ratio(c, crate::WELL) >= 4.5, "{c:?}");
        }
        for c in [HUD_VALUE, HUD_TILT_WARN, HUD_TILT_DANGER] {
            assert!(ratio(c, hex(0x0A0A0A)) >= 4.5, "{c:?}");
        }
    }

    #[test]
    fn the_marker_and_the_plumb_line_are_visible() {
        for b in COMFORT_BANDS {
            assert!(ratio(METER_MARKER, b) >= 3.0, "{b:?}");
        }
        assert!(ratio(PLUMB, crate::GRAPH.base) >= 3.0);
        assert!(ratio(LIMIT, crate::WELL) >= 3.0);
    }
}
