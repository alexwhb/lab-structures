//! Advanced (truss netlist) colours — `Advanced.dc.html`. Owned by the `advanced` agent: every
//! token names its CSS source.
//!
//! The force colours (tension / compression / zero / over) and the neutrals live in the crate
//! root ([`crate::TENSION`], [`crate::BORDER`], …); this module names their Advanced roles.

use crate::{hex, rgba};
use egui::Color32;

// ---- netlist editor (`.mono` 12 px / 1.75 lines)

/// Line numbers `color: #4A4A4A`.
pub const GUTTER: Color32 = crate::SLASH;
/// Directives (`l.dc` for a `.` line) `#D6C7A1`.
pub const DIRECTIVE: Color32 = crate::ACCENT;
/// `*` comment lines (`l.dc` for a `*` line) and `; …` tails `#7E8A7E`.
pub const COMMENT: Color32 = crate::COMMENT;
/// Arguments `color: #D9D9D9`.
pub const ARGS: Color32 = crate::TEXT_DIM;
/// An error line's band: the series' danger red `#E5534B` at 14 % (the mockup's editor has no
/// error state; the band follows Automata's script editor).
pub const ERROR_BAND: Color32 = rgba(229, 83, 75, 0.14);
/// An error's underline, gutter number and message: `#E5534B`.
pub const ERROR: Color32 = crate::DANGER;
/// The inline error message chip behind its text: `rgba(11,12,14,.92)` (the `.chip` fill).
pub const ERROR_CHIP: Color32 = crate::CHIP;
/// A warning's underline and message (`.wind` …): the series' warning `#D8B86A`.
pub const WARNING: Color32 = crate::WARN;
/// Opacity of the last good result while the netlist has errors (ADV-2 "dimmed").
pub const DIMMED: f32 = 0.45;

// ---- elevation (`<svg viewBox="0 0 1000 270">`)

/// Near-zero members `stroke="#5E5E5E"`.
pub const MEMBER_ZERO: Color32 = crate::ZERO_FORCE;
/// Tension members `stroke="#7FB0E0"`.
pub const MEMBER_TENSION: Color32 = crate::TENSION;
/// Compression members `stroke="#E0905E"`.
pub const MEMBER_COMPRESSION: Color32 = crate::COMPRESSION;
/// Over-limit / no-solution members `stroke="#E5534B" stroke-dasharray="8 5"`.
pub const MEMBER_OVER: Color32 = crate::OVER;
/// Member label text: near-zero `#A3A3A3`.
pub const LABEL_ZERO: Color32 = crate::TEXT_MUTED;
/// Member label text: tension `#7FB0E0`.
pub const LABEL_TENSION: Color32 = crate::TENSION;
/// Member label text: compression `#E7A47C`.
pub const LABEL_COMPRESSION: Color32 = crate::COMPRESSION_TEXT;
/// Member label text: over the limit `#E8A39E`.
pub const LABEL_OVER: Color32 = crate::DANGER_INK;
/// Member label background `background: rgba(11,12,14,.85)`.
pub const LABEL_BG: Color32 = rgba(11, 12, 14, 0.85);
/// Node label text `color: #A3A3A3`.
pub const NODE_LABEL: Color32 = crate::TEXT_MUTED;
/// Node circles `fill="#0A0A0A" stroke="#D4D4D4"`.
pub const NODE_FILL: Color32 = hex(0x0A0A0A);
/// See [`NODE_FILL`].
pub const NODE_RING: Color32 = crate::TEXT_BODY;
/// Supports and the ground line `stroke="#8A8A8A" fill="#8A8A8A"`.
pub const SUPPORT: Color32 = crate::TEXT_FAINT;
/// The truck arrow `stroke="#D6C7A1"`.
pub const TRUCK: Color32 = crate::ACCENT;

// ---- influence line (`<svg viewBox="0 0 640 220">`)

/// The zero axis `stroke="#4A4A4A"`.
pub const IL_AXIS: Color32 = crate::SLASH;
/// The limit dashes `stroke="#E5534B" stroke-dasharray="5 5"`.
pub const IL_LIMIT: Color32 = crate::DANGER;
/// The cursor line and dot `#D6C7A1`.
pub const IL_CURSOR: Color32 = crate::ACCENT;
/// The fill under the line: the line's colour at `fill-opacity="0.14"`.
pub const IL_FILL_OPACITY: f32 = 0.14;
/// The plot's corner labels `color: #A3A3A3`.
pub const IL_NOTE: Color32 = crate::TEXT_MUTED;

// ---- member results table

/// Row rule `border-top: 1px solid #1E1E1E`.
pub const ROW_RULE: Color32 = crate::TRACK;
/// ID column `color: #D6C7A1`.
pub const ID: Color32 = crate::ACCENT;
/// Nodes column `color: #D9D9D9`.
pub const NODES: Color32 = crate::TEXT_DIM;
/// Limit column and headings `color: #A3A3A3`.
pub const MUTED: Color32 = crate::TEXT_MUTED;
/// Use column: normal `#EAEAEA`.
pub const USE: Color32 = crate::TEXT;
/// Use column: over 80 % `#D6C7A1`.
pub const USE_NEAR: Color32 = crate::ACCENT;
/// Use column: over 100 % `#E8A39E`.
pub const USE_OVER: Color32 = crate::DANGER_INK;

// ---- checks and badges

/// Check dots: good `#5FBF8A`.
pub const CHECK_GOOD: Color32 = crate::OK;
/// Check dots: worth a look `#D6C7A1`.
pub const CHECK_ACCENT: Color32 = crate::ACCENT;
/// Check dots: bad `#E5534B`.
pub const CHECK_BAD: Color32 = crate::DANGER;
/// Check dots: information `#A3A3A3`.
pub const CHECK_NEUTRAL: Color32 = crate::TEXT_MUTED;
/// The "educational, not for design" badge dot `background: #E8A39E`.
pub const EDUCATIONAL: Color32 = crate::DANGER_INK;

// ---- toolbar controls (Chrome's native controls in the mockup)

/// `<select>` `background: #121212; border: 1px solid #2E2E2E; color: #EAEAEA`.
pub const SELECT: Color32 = crate::ROW;
/// See [`SELECT`].
pub const SELECT_BORDER: Color32 = crate::BORDER_STRONG;
/// The select's chevron as Chrome draws it (sampled `#B4B4B4` in the reference).
pub const SELECT_CHEVRON: Color32 = hex(0xB4B4B4);
/// The slider outputs `color: #D6C7A1` (mono 12).
pub const OUTPUT: Color32 = crate::ACCENT;

#[cfg(test)]
mod tests {
    use super::*;
    use lab_theme::contrast::ratio;

    #[test]
    fn text_and_graphics_contrast() {
        let panel = hex(0x111111);
        for c in [
            DIRECTIVE, ARGS, COMMENT, ID, NODES, MUTED, USE, USE_NEAR, USE_OVER,
        ] {
            assert!(ratio(c, panel) >= 4.5, "{c:?}");
        }
        for c in [
            LABEL_ZERO,
            LABEL_TENSION,
            LABEL_COMPRESSION,
            LABEL_OVER,
            NODE_LABEL,
        ] {
            assert!(ratio(c, hex(0x0B0C0E)) >= 4.5, "{c:?}");
        }
        for c in [
            MEMBER_TENSION,
            MEMBER_COMPRESSION,
            MEMBER_OVER,
            SUPPORT,
            TRUCK,
        ] {
            assert!(ratio(c, hex(0x0B0C0E)) >= 3.0, "{c:?}");
        }
        assert!(ratio(ERROR, panel) >= 3.0);
    }
}
