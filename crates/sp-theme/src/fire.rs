//! Fire (FIRE-1) and sample drawer (LAB-1) colours. Owned by the `lab` agent.
//!
//! Neither fold-in is in the v2 mockup, so the tokens build on its palette: the piece swatches
//! (`types()[k].sw`, [`crate::builder::PIECES`]), the v1 Materials rig and plot
//! ([`crate::materials`]), the status colours, and for heat the incandescence ("heat colour")
//! of hot steel — faint red at about 480 °C, cherry at about 750 °C, orange at about 900 °C,
//! yellow near 1000 °C (blacksmiths' heat-colour charts; Draper point ≈ 525 °C).

use egui::Color32;
use lab_theme::{hex, rgba};

// ============================================================================================
// Sample drawer (LAB-1)

/// The sample's fill in the rig, per piece type in `types()` order (Road, Floor, Wood, Steel,
/// Concrete, Rope, Cable): the builder's swatches (`types()[k].sw`).
pub const SAMPLE_FILL: [Color32; 7] = [
    crate::builder::ROAD.swatch,
    crate::builder::FLOOR.swatch,
    crate::builder::WOOD.swatch,
    crate::builder::STEEL.swatch,
    crate::builder::CONCRETE.swatch,
    crate::builder::ROPE.swatch,
    crate::builder::CABLE.swatch,
];

/// The curve on the plot, per piece type: the swatches, the two dark deck swatches lightened
/// (Road `#46474C` → `#9A9CA3`, Floor `#3A4A5C` → `#8AA3BE`) and Wood and Steel a step lighter,
/// so every curve reaches 3:1 on the `.graph` ground (LOOK-4).
pub const SAMPLE_LINE: [Color32; 7] = [
    hex(0x9A9CA3),
    hex(0x8AA3BE),
    hex(0xB08A5C),
    hex(0xA9B0BA),
    crate::builder::CONCRETE.swatch,
    crate::builder::ROPE.swatch,
    crate::builder::CABLE.swatch,
];

/// A burned-through sample (charred wood, melted rope).
pub const CHARRED: Color32 = hex(0x2B2522);

/// The builder's limit on the plot: a dashed line in the "will break" red (`#E5534B`).
pub const LIMIT_LINE: Color32 = rgba(229, 83, 75, 0.85);

/// The limit line's label ("builder limit 153 t"), the "will break" red at text contrast.
pub const LIMIT_TEXT: Color32 = hex(0xEF7A73);

/// The room-temperature curve behind a heated one: v1's ghost grey
/// ([`crate::materials::GHOST_CURVE`]).
pub const ROOM_GHOST: Color32 = crate::materials::GHOST_CURVE;

// ============================================================================================
// Fire on the stage (FIRE-1)

/// The flames' outer tongues.
pub const FLAME_OUTER: Color32 = rgba(232, 102, 58, 0.85);
/// The flames' middle.
pub const FLAME_MID: Color32 = rgba(245, 154, 60, 0.9);
/// The flames' core.
pub const FLAME_CORE: Color32 = hex(0xFFD27A);
/// The burning heap / barge under the flames.
pub const FIRE_BED: Color32 = hex(0x4A2418);
/// Embers on the bed.
pub const EMBER: Color32 = hex(0xC2451E);
/// The fire's reach: a faint warm haze.
pub const REACH_HAZE: Color32 = rgba(240, 120, 50, 0.06);

/// The HUD chip's value ("Fire 650 °C"), flame orange at text contrast on the HUD pill.
pub const HUD_FIRE: Color32 = hex(0xF2995E);

/// The Fire row's chip dots: Off grey, Small / Big / Inferno warmer.
pub const LEVEL_DOT: [Color32; 4] = [hex(0x6A6A6A), hex(0xC9763F), hex(0xE5583A), hex(0xF7B23B)];

/// Heat colour stops for a hot piece's glow: `(°C, colour)`, interpolated in between; no glow
/// below the first stop.
pub const GLOW_STOPS: [(f32, Color32); 6] = [
    (150.0, rgba(120, 30, 18, 0.0)),
    (400.0, rgba(140, 32, 18, 0.55)),
    (550.0, rgba(178, 38, 18, 0.7)),
    (700.0, rgba(222, 62, 26, 0.8)),
    (850.0, rgba(246, 128, 40, 0.85)),
    (950.0, rgba(255, 196, 92, 0.9)),
];

/// The glow drawn over a piece at `temp_c` °C (`None` when it's too cool to show): the heat
/// colour stops, interpolated.
pub fn glow(temp_c: f32) -> Option<Color32> {
    let t = if temp_c.is_nan() { 0.0 } else { temp_c };
    let (t0, _) = GLOW_STOPS[0];
    if t <= t0 {
        return None;
    }
    let last = GLOW_STOPS[GLOW_STOPS.len() - 1];
    if t >= last.0 {
        return Some(last.1);
    }
    for w in GLOW_STOPS.windows(2) {
        let ((a, ca), (b, cb)) = (w[0], w[1]);
        if t <= b {
            let f = (t - a) / (b - a);
            return Some(mix_rgba(ca, cb, f));
        }
    }
    Some(last.1)
}

/// Linear mix of two colours in unmultiplied sRGBA.
fn mix_rgba(a: Color32, b: Color32, f: f32) -> Color32 {
    let (a, b) = (a.to_srgba_unmultiplied(), b.to_srgba_unmultiplied());
    let m = |i: usize| (a[i] as f32 + (b[i] as f32 - a[i] as f32) * f).round() as u8;
    Color32::from_rgba_unmultiplied(m(0), m(1), m(2), m(3))
}

#[cfg(test)]
mod tests {
    use super::*;
    use lab_theme::contrast::{over, ratio};

    #[test]
    fn sample_curves_reach_3_to_1_on_the_graph() {
        for (i, c) in SAMPLE_LINE.iter().enumerate() {
            let r = ratio(*c, crate::GRAPH.base);
            assert!(r >= 3.0, "curve {i}: {r:.2}");
        }
        assert!(ratio(over(LIMIT_LINE, crate::GRAPH.base), crate::GRAPH.base) >= 3.0);
        assert!(ratio(LIMIT_TEXT, crate::WELL) >= 4.5);
        assert!(ratio(LIMIT_TEXT, crate::GRAPH.base) >= 4.5);
    }

    #[test]
    fn hud_and_dots_are_legible() {
        let pill = over(crate::builder::HUD, crate::builder::STAGE);
        assert!(ratio(HUD_FIRE, pill) >= 4.5);
        let well = crate::WELL;
        for d in LEVEL_DOT.iter().skip(1) {
            assert!(ratio(*d, well) >= 3.0);
        }
    }

    #[test]
    fn the_glow_warms_with_temperature() {
        assert_eq!(glow(20.0), None);
        assert_eq!(glow(150.0), None);
        assert_eq!(glow(f32::NAN), None);
        let a = glow(400.0).unwrap();
        let b = glow(900.0).unwrap();
        assert!(
            b.to_srgba_unmultiplied()[1] > a.to_srgba_unmultiplied()[1],
            "yellower"
        );
        assert_eq!(glow(5000.0), Some(GLOW_STOPS[5].1));
        // A hot piece's glow stands out on the stage.
        let hot = over(glow(700.0).unwrap(), crate::builder::STAGE);
        assert!(ratio(hot, crate::builder::STAGE) >= 3.0);
    }
}
