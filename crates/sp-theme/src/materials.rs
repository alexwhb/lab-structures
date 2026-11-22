//! Materials lab colours — `Materials.dc.html` inline SVG and markup. Owned by the `materials`
//! agent. The sample swatches are the shared [`crate::STEEL`], [`crate::ALUMINUM`],
//! [`crate::CONCRETE`], [`crate::TIMBER`] and [`crate::CARBON`]; status colours are the shared
//! ramp ([`crate::OK`], [`crate::ACCENT`], [`crate::CAUTION`], [`crate::DANGER`]).

use crate::{Swatch, hex, rgba};
use egui::Color32;

// ---------------------------------------------------------------------------------------------
// The tensile rig (`<svg viewBox="0 0 420 560">`)

/// Machine base, top face `<path d="M60 500 H360 L392 480 H92 Z" fill="#33363C">`.
pub const BASE_TOP: Color32 = hex(0x33363C);
/// Machine base, front `<path d="M60 500 H360 V530 H60 Z" fill="#24262A">`.
pub const BASE_FRONT: Color32 = hex(0x24262A);
/// The two columns' fronts `fill="#2E323A"`.
pub const COLUMN: Color32 = hex(0x2E323A);
/// The columns' 2.5D sides `fill="#1B1D21"`.
pub const COLUMN_SIDE: Color32 = hex(0x1B1D21);
/// The fixed top beam `<path d="M80 34 H340 V60 H80 Z" fill="#3C414A">`.
pub const BEAM: Color32 = hex(0x3C414A);
/// Crosshead and bottom grip `fill="#4A4F58" stroke="#5E636C"`.
pub const HEAD: Color32 = hex(0x4A4F58);
/// See [`HEAD`].
pub const HEAD_STROKE: Color32 = hex(0x5E636C);
/// The crosshead's top face `fill="#5A606A"`.
pub const HEAD_TOP: Color32 = hex(0x5A606A);
/// The sample's outline `stroke="#0A0A0A"` (1 px).
pub const SPECIMEN_STROKE: Color32 = hex(0x0A0A0A);
/// The extensometer `stroke="#D6C7A1"` (1.5 px) = the accent.
pub const GAUGE: Color32 = crate::ACCENT;
/// The crack `stroke="#E5534B"` (2 px) = the danger red.
pub const CRACK: Color32 = crate::DANGER;
/// The load cell overlay `background: rgba(10,10,10,.82); border: 1px solid #242424`.
pub const OVERLAY: Color32 = rgba(10, 10, 10, 0.82);

// ---------------------------------------------------------------------------------------------
// The stress–strain plot (`<svg viewBox="0 0 560 400">` on `.graph`)

/// Axes `<path d="M56 20 V350 H544" stroke="#5A5A5A" stroke-width="1.2">`.
pub const AXIS: Color32 = hex(0x5A5A5A);
/// Grid lines `stroke="#2A2A2A"`.
pub const GRID: Color32 = hex(0x2A2A2A);
/// The other four curves `stroke="#6A6A6A" stroke-opacity="0.55"` (1.5 px).
pub const GHOST_CURVE: Color32 = rgba(0x6A, 0x6A, 0x6A, 0.55);
/// The done-so-far trace `stroke="#D6C7A1"` (3 px) and the marker `fill="#D6C7A1"`.
pub const DONE: Color32 = crate::ACCENT;
/// The marker's ring `stroke="#0A0A0A" stroke-width="2"`.
pub const MARKER_RING: Color32 = hex(0x0A0A0A);
/// Axis titles "stress MPa" / "strain %" `color: #D9D9D9`.
pub const AXIS_TITLE: Color32 = crate::TEXT_DIM;
/// Tick labels `color: #A3A3A3`.
pub const TICK: Color32 = crate::TEXT_MUTED;
/// The selected curve's label `#F5F5F5` (weight 600); the others `#8A8A8A` (400).
pub const LABEL_ON: Color32 = crate::TEXT_STRONG;
/// See [`LABEL_ON`].
pub const LABEL_OFF: Color32 = crate::TEXT_FAINT;

// ---------------------------------------------------------------------------------------------
// Readouts

/// Strength per kilogram: the selected bar `#D6C7A1`, the others `#6A6A6A`.
pub const BAR_ON: Color32 = crate::ACCENT;
/// See [`BAR_ON`].
pub const BAR_OFF: Color32 = hex(0x6A6A6A);
/// The selected bar's name `#F5F5F5` (600); the others `#A3A3A3`.
pub const BAR_NAME_ON: Color32 = crate::TEXT_STRONG;
/// See [`BAR_NAME_ON`].
pub const BAR_NAME_OFF: Color32 = crate::TEXT_MUTED;
/// A broken sample's challenge chip `#5FBF8A`.
pub const CHIP_LIT: Color32 = crate::OK;
/// A not-yet-broken chip's dot `#6A6A6A`.
pub const CHIP_DOT_OFF: Color32 = hex(0x6A6A6A);
/// A not-yet-broken chip's text. The mockup draws it `#6A6A6A` (3.4:1 on the `#141414` pill,
/// below LOOK-4's 4.5:1), so the text uses `#8A8A8A` (5.3:1) and only the dot keeps
/// `#6A6A6A`.
pub const CHIP_TEXT_OFF: Color32 = crate::TEXT_FAINT;

// ---------------------------------------------------------------------------------------------
// Sample swatches (`box-shadow: inset 0 -3px 0 rgba(0,0,0,.25), 0 4px 0 <side>, 0 8px 10px
// rgba(0,0,0,.6)`)

/// The swatch's inset bottom band `rgba(0,0,0,.25)` (3 px).
pub const SWATCH_INSET: Color32 = rgba(0, 0, 0, 0.25);
/// The swatch's drop shadow `rgba(0,0,0,.6)` (0 8px 10px).
pub const SWATCH_SHADOW: Color32 = rgba(0, 0, 0, 0.6);

/// The five samples' swatches in the mockup's order (Steel, Aluminum, Concrete, Timber,
/// Carbon fiber): `fill` the face and the sample, `side` the 2.5D edge, `line` the plotted
/// curve (`defs()` `fill` / `side` / `color`).
pub const SAMPLES: [Swatch; 5] = [
    crate::STEEL,
    crate::ALUMINUM,
    crate::CONCRETE,
    crate::TIMBER,
    crate::CARBON,
];

#[cfg(test)]
mod tests {
    use super::*;
    use lab_theme::contrast::{over, ratio};

    #[test]
    fn values_match_the_mockup_css() {
        assert_eq!(SAMPLES[0].fill, hex(0x9AA3AD));
        assert_eq!(SAMPLES[4].fill, hex(0x3B3F46));
        assert_eq!(SAMPLES[4].line, hex(0x7D8590));
        assert_eq!(GHOST_CURVE.a(), 140);
    }

    #[test]
    fn readout_text_reaches_4_5() {
        let pill = crate::CONTROL;
        assert!(ratio(CHIP_TEXT_OFF, pill) >= 4.5);
        assert!(ratio(CHIP_LIT, pill) >= 4.5);
        let graph = crate::GRAPH.base;
        for c in [LABEL_ON, LABEL_OFF, TICK, AXIS_TITLE] {
            assert!(ratio(c, graph) >= 4.5, "{c:?}");
        }
        let overlay = over(OVERLAY, graph);
        assert!(ratio(crate::ACCENT, overlay) >= 4.5);
        let side = over(crate::SIDE, crate::PAPER.base);
        for c in [BAR_NAME_ON, BAR_NAME_OFF, crate::CAUTION, crate::ACCENT] {
            assert!(ratio(c, side) >= 4.5, "{c:?}");
        }
    }

    #[test]
    fn the_crack_and_trace_stand_out() {
        let graph = crate::GRAPH.base;
        assert!(ratio(CRACK, graph) >= 3.0);
        assert!(ratio(DONE, graph) >= 3.0);
        for s in SAMPLES {
            assert!(ratio(s.line, graph) >= 3.0, "{:?}", s.line);
        }
    }
}
