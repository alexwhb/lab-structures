//! `sp-theme` — Structures Playground's visual tokens.
//!
//! The lab series shares one look (labkit's [`lab_theme`]: fonts, the egui style builder,
//! contrast helpers). Structures Playground uses the series' neutral greys (`#0A0A0A` paper,
//! `#242424` lines) and adds **one sand accent ramp** (`#D6C7A1`), the **force colours**
//! (tension blue, compression orange, "will break" red), status colours and material swatches
//! (mockup `docs/design/mockup/Series.dc.html`: "Same shell, same engines, one new accent").
//!
//! Every value names its CSS source in `docs/design/mockup/*.dc.html`. Screens never hard-code
//! a colour: they use the constants here (or [`lab_theme::pal`] for the generic roles).
//!
//! Layout: this file holds the shared tokens (lead-owned). Each screen's scene colours live in
//! its own module ([`bridge`], [`advanced`], [`tower`], [`ground`], [`materials`]), owned by
//! that screen's agent (see `PROGRESS.md`); add tokens there with their CSS source.
//!
//! ```ignore
//! sp_theme::install(&cc.egui_ctx); // fonts + egui style + the palette below
//! ```

use egui::Color32;
pub use lab_theme::{self, Grid, font, hex, mix, rgba};

pub mod advanced;
pub mod bridge;
pub mod ground;
pub mod materials;
pub mod tower;

/// Installs the series fonts and the egui style built from [`PALETTE`].
pub fn install(ctx: &egui::Context) {
    lab_theme::install(ctx, PALETTE);
}

/// Each RGB channel × `k`, rounded (darker side faces of 2.5D solids).
pub fn shade(c: Color32, k: f32) -> Color32 {
    let [r, g, b, a] = c.to_srgba_unmultiplied();
    let f = |v: u8| (v as f32 * k).round().clamp(0.0, 255.0) as u8;
    Color32::from_rgba_unmultiplied(f(r), f(g), f(b), a)
}

// ============================================================================================
// Grounds (graph paper)

/// `.paper` — the window ground: `#0A0A0A`, 60 px lines at 4 % white, 12 px at 1.6 %.
pub const PAPER: Grid = Grid {
    base: hex(0x0A0A0A),
    major: rgba(255, 255, 255, 0.04),
    major_px: 60.0,
    minor: rgba(255, 255, 255, 0.016),
    minor_px: 12.0,
};
/// `.graph` — the stage behind the tower / ground / test rig / plots: `#0B0C0E`, 120 px lines
/// at 5 %, 24 px at 2 %.
pub const GRAPH: Grid = Grid {
    base: hex(0x0B0C0E),
    major: rgba(255, 255, 255, 0.05),
    major_px: 120.0,
    minor: rgba(255, 255, 255, 0.02),
    minor_px: 24.0,
};

// ============================================================================================
// Neutrals (shared with Breadbox / Automata)

/// Header and tab-row fill `rgba(12,12,12,0.85)`.
pub const BAR: Color32 = rgba(12, 12, 12, 0.85);
/// Side panels (tool rail, `aside`) `rgba(17,17,17,0.84)`.
pub const SIDE: Color32 = rgba(17, 17, 17, 0.84);
/// `.panel` `rgba(17,17,17,.88)` with a `#242424` border, radius 12.
pub const PANEL: Color32 = rgba(17, 17, 17, 0.88);
/// `.well` `#0D0D0D` (border `#242424`, radius 8): stat tiles, inspector, netlist.
pub const WELL: Color32 = hex(0x0D0D0D);
/// Header wells (mode switcher, segmented groups) `rgba(6,6,6,0.88)`.
pub const INSET_BAR: Color32 = rgba(6, 6, 6, 0.88);
/// `.ptile` fill `rgba(18,18,18,.92)` (picture tiles: systems, shapes, soils, samples).
pub const TILE: Color32 = rgba(18, 18, 18, 0.92);
/// `.lvl` (level rows) fill `#121212`.
pub const ROW: Color32 = hex(0x121212);
/// `.ghost` button fill `#171717`.
pub const GHOST: Color32 = hex(0x171717);
/// `.ghost:hover` `#1F1F1F`.
pub const GHOST_HOVER: Color32 = hex(0x1F1F1F);
/// `.chip`, `.pill`, `.mh` fill `#141414`.
pub const CONTROL: Color32 = hex(0x141414);
/// `.sw:hover` `#1C1C1C`.
pub const SW_HOVER: Color32 = hex(0x1C1C1C);
/// `.bar` (progress / utilisation track) `#1E1E1E`.
pub const TRACK: Color32 = hex(0x1E1E1E);
/// Segmented "on" fill (`.seg-on`) `#2A2A2A`.
pub const SEG_ON: Color32 = hex(0x2A2A2A);
/// Panel separators, tiles, card borders `#242424`.
pub const BORDER: Color32 = hex(0x242424);
/// Button / chip / badge borders `#2E2E2E`.
pub const BORDER_STRONG: Color32 = hex(0x2E2E2E);
/// Toast / overlay border `#3A3A3A`.
pub const BORDER_OVERLAY: Color32 = hex(0x3A3A3A);
/// Overlay fill (toast, loading bar, result banner) `rgba(10,10,10,.92)`.
pub const OVERLAY: Color32 = rgba(10, 10, 10, 0.92);
/// Primary text `#EAEAEA`.
pub const TEXT: Color32 = hex(0xEAEAEA);
/// Emphasis text (selected segment, active tab) `#F5F5F5`.
pub const TEXT_STRONG: Color32 = hex(0xF5F5F5);
/// Body copy in panels (hint, goal, explain) `#D4D4D4`.
pub const TEXT_BODY: Color32 = hex(0xD4D4D4);
/// Secondary labels, ghost-button text `#D9D9D9`.
pub const TEXT_DIM: Color32 = hex(0xD9D9D9);
/// Chip text `#C9C9C9`.
pub const TEXT_CHIP: Color32 = hex(0xC9C9C9);
/// Captions, section eyebrows (`.lbl`), inactive tabs, `.mut` `#A3A3A3`.
pub const TEXT_MUTED: Color32 = hex(0xA3A3A3);
/// Plot curve labels for the inactive materials `#8A8A8A`.
pub const TEXT_FAINT: Color32 = hex(0x8A8A8A);
/// Unchecked goal boxes, inactive bars `#5A5A5A` / `#6A6A6A` (decorative).
pub const TEXT_DISABLED: Color32 = hex(0x5E5E5E);
/// The `/` between brand and document name `#4A4A4A` (decorative).
pub const SLASH: Color32 = hex(0x4A4A4A);
/// Netlist comments (`* …`, `; …`) `#7E8A7E`.
pub const COMMENT: Color32 = hex(0x7E8A7E);
/// Pinned measurement chip `.chip` (the floating one): `rgba(11,12,14,.92)`, border `#2A2D33`,
/// text `#ECEAE6`.
pub const CHIP: Color32 = rgba(11, 12, 14, 0.92);
pub const CHIP_BORDER: Color32 = hex(0x2A2D33);
pub const CHIP_TEXT: Color32 = hex(0xECEAE6);

// ============================================================================================
// The sand ramp (Structures' one accent)

/// Accent `#D6C7A1`: primary actions, selection, active tab underline, sliders, checkboxes,
/// netlist directives, the logo.
pub const ACCENT: Color32 = hex(0xD6C7A1);
/// Ink `#E9DFC6`: text on tinted fills (`.prim`, `.chip-on`, `.tool-on`, `.lvl-on`).
pub const INK: Color32 = hex(0xE9DFC6);
/// Tint `#24221C`: primary button / selected chip / selected tool fill.
pub const TINT: Color32 = hex(0x24221C);
/// Edge `#4D4636`: primary button / selected tool / hovered tile border.
pub const EDGE: Color32 = hex(0x4D4636);
/// Selected chip / level border `#6A604A`.
pub const EDGE_ON: Color32 = hex(0x6A604A);
/// `.prim:hover` `#2E2B23`.
pub const TINT_HOVER: Color32 = hex(0x2E2B23);
/// `.ptile:hover` / `.ptile-on` fill `#1E1C17`.
pub const TILE_ON: Color32 = hex(0x1E1C17);
/// Load tags under crates `#E6D6B8`.
pub const LOAD_TAG: Color32 = hex(0xE6D6B8);

// ============================================================================================
// Status (pills, tiles, checks) — the mockups' traffic-light ramp

/// Fine / holding / done `#5FBF8A`.
pub const OK: Color32 = hex(0x5FBF8A);
/// Near the limit (Playground status) `#D8B86A`. Other screens use [`ACCENT`] for "marginal".
pub const WARN: Color32 = hex(0xD8B86A);
/// Leaning / too much / uneasy `#E7A47C`.
pub const CAUTION: Color32 = hex(0xE7A47C);
/// Failed / unstable / will break `#E5534B`.
pub const DANGER: Color32 = hex(0xE5534B);
/// Danger text on dark (over budget, lock-in, liquefied) `#E8A39E`.
pub const DANGER_INK: Color32 = hex(0xE8A39E);
/// Danger banner / badge border `#5C2A25`.
pub const DANGER_EDGE: Color32 = hex(0x5C2A25);
/// Success banner border `#2B5A3C`.
pub const OK_EDGE: Color32 = hex(0x2B5A3C);
/// Over-limit member handle text `#F2B8B3`.
pub const DANGER_SOFT: Color32 = hex(0xF2B8B3);
/// Info values (wind, plot cursors) `#4CC3F0`.
pub const INFO: Color32 = hex(0x4CC3F0);
/// Provenance dots: exact `#5FBF8A`, simplified `#D6C7A1`, illustrative `#A3A3A3`.
pub const PROV_EXACT: Color32 = OK;
pub const PROV_SIMPLIFIED: Color32 = ACCENT;
pub const PROV_ILLUSTRATIVE: Color32 = TEXT_MUTED;

// ============================================================================================
// Forces (members, results, plots)

/// Tension (stretched) `#7FB0E0`.
pub const TENSION: Color32 = hex(0x7FB0E0);
/// Compression (squeezed) in the Playground stage `#E0905E`.
pub const COMPRESSION: Color32 = hex(0xE0905E);
/// Compression in the Advanced elevation and results table `#E7A47C`.
pub const COMPRESSION_TEXT: Color32 = hex(0xE7A47C);
/// A member carrying almost nothing (|N| < 500 N) `#5E5E5E`.
pub const ZERO_FORCE: Color32 = hex(0x5E5E5E);
/// A member with no solution yet (structure unstable) `#9AA0A8`.
pub const NO_FORCE: Color32 = hex(0x9AA0A8);
/// Will break / over its limit (dashed) = [`DANGER`].
pub const OVER: Color32 = DANGER;
/// Opacity of a force colour by utilisation band: < 35 %, < 70 %, ≥ 70 %
/// (`stroke-opacity` .45 / .75 / 1).
pub const FORCE_BANDS: [f32; 3] = [0.45, 0.75, 1.0];

// ============================================================================================
// Materials (swatches; Playground `mats()`, Advanced `mats()`, Materials `defs()`)

/// A material's swatch: the face, the darker 2.5D side, and the plot line.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Swatch {
    pub fill: Color32,
    pub side: Color32,
    pub line: Color32,
}
/// Steel `#9AA3AD` / side `#4E545B`.
pub const STEEL: Swatch = Swatch {
    fill: hex(0x9AA3AD),
    side: hex(0x4E545B),
    line: hex(0x9AA3AD),
};
/// Aluminum `#D5DAE0` / side `#7C838B`.
pub const ALUMINUM: Swatch = Swatch {
    fill: hex(0xD5DAE0),
    side: hex(0x7C838B),
    line: hex(0xD5DAE0),
};
/// Timber / glulam `#C49A5E` / side `#6E5231`.
pub const TIMBER: Swatch = Swatch {
    fill: hex(0xC49A5E),
    side: hex(0x6E5231),
    line: hex(0xC49A5E),
};
/// Carbon fiber: Playground dot `#5B616B`; Advanced / Materials face `#3B3F46`, side
/// `#1C1E22`, plot line `#7D8590`.
pub const CARBON: Swatch = Swatch {
    fill: hex(0x3B3F46),
    side: hex(0x1C1E22),
    line: hex(0x7D8590),
};
/// Carbon fiber's dot in the Playground "Built from" chips `#5B616B`.
pub const CARBON_DOT: Color32 = hex(0x5B616B);
/// Concrete `#B9B4A8` / side `#66625A`.
pub const CONCRETE: Swatch = Swatch {
    fill: hex(0xB9B4A8),
    side: hex(0x66625A),
    line: hex(0xB9B4A8),
};

// ============================================================================================
// App identities (Series screen)

/// One app's identity: name, one-line domain and its accent (the card's 3 px top border).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Identity {
    pub name: &'static str,
    pub domain: &'static str,
    pub accent: Color32,
}
pub const BREADBOX: Identity = Identity {
    name: "Breadbox",
    domain: "Circuits. Breadboard playground, SPICE schematic.",
    accent: hex(0xF2A33A),
};
pub const CHEM: Identity = Identity {
    name: "Chem Playground",
    domain: "Reactions. 2.5D bench, kinetics netlist, mechanisms.",
    accent: hex(0x86A2FF),
};
pub const BIO: Identity = Identity {
    name: "Bio Playground",
    domain: "Living systems. 2.5D lab, pathway models.",
    accent: hex(0x4BE3A0),
};
pub const WEATHER: Identity = Identity {
    name: "Weather Playground",
    domain: "The sky. Air parcels, skew-T, grid atmosphere.",
    accent: hex(0xEE84E0),
};
pub const STRUCTURES: Identity = Identity {
    name: "Structures Playground",
    domain: "Things that stand up. Bridge, tower, ground, materials.",
    accent: ACCENT,
};
/// The Structures card on the Series screen: fill `#1C1B17`, border `#3A372E`, body `#D9D3C2`.
pub const SELF_CARD: Color32 = hex(0x1C1B17);
pub const SELF_CARD_BORDER: Color32 = hex(0x3A372E);
pub const SELF_CARD_TEXT: Color32 = hex(0xD9D3C2);
/// The series in the Series screen's order ("app 5 of 5").
pub const SERIES: [Identity; 5] = [BREADBOX, CHEM, BIO, WEATHER, STRUCTURES];

// ============================================================================================
// Sizes (points; 1 pt = 1 CSS px at 1440 × 900)

pub mod size {
    /// Header min-height 56.
    pub const HEADER: f32 = 56.0;
    /// World tab row: tabs are 44 tall.
    pub const TAB_ROW: f32 = 44.0;
    /// `.ghost` / `.prim` / `.seg` min-height 36; the big "Test it" button 44.
    pub const BUTTON: f32 = 36.0;
    pub const BUTTON_BIG: f32 = 44.0;
    /// `.chip` min-height 34.
    pub const CHIP: f32 = 34.0;
    /// `.lvl` level rows min-height 40.
    pub const ROW: f32 = 40.0;
    /// `.tool` rail buttons 60 × 56 in a 76 px rail.
    pub const TOOL_W: f32 = 60.0;
    pub const TOOL_H: f32 = 56.0;
    pub const RAIL_W: f32 = 76.0;
    /// `.ptile` min-height 84.
    pub const PTILE: f32 = 84.0;
    /// Joint hit target 30 × 30 (dot 11); member handle 14 (20 with a number).
    pub const JOINT_HIT: f32 = 30.0;
    pub const JOINT_DOT: f32 = 11.0;
    pub const HANDLE: f32 = 14.0;
    pub const HANDLE_NUM: f32 = 20.0;
    /// Range input height 28; checkbox 18.
    pub const RANGE: f32 = 28.0;
    pub const CHECKBOX: f32 = 18.0;
    /// Smallest pointer target (WCAG 2.5.5).
    pub const MIN_TARGET: f32 = 44.0;
    /// Right aside: flex 1 1 300, 260–360. Left control column (Tower / Ground / Materials) 248.
    pub const ASIDE_MIN: f32 = 260.0;
    pub const ASIDE_MAX: f32 = 360.0;
    pub const CONTROLS_W: f32 = 248.0;
    /// Below this width the columns stack (the mockup's `flex-wrap: wrap` rows).
    pub const NARROW: f32 = 720.0;
}

pub mod radius {
    /// Buttons, segmented pills 6; badges 5; wells 8; level rows 8; tool buttons 10; panels and
    /// picture tiles 12; chips / pills 999.
    pub const BUTTON: u8 = 6;
    pub const BADGE: u8 = 5;
    pub const WELL: u8 = 8;
    pub const ROW: u8 = 8;
    pub const TOOL: u8 = 10;
    pub const PANEL: u8 = 12;
    pub const PILL: u8 = 255;
}

// ============================================================================================
// The palette installed into egui (generic roles for labkit helpers and egui widgets)

/// [`lab_theme::LAB_DARK`] with the series greys and the sand ramp in the accent roles
/// (`amber*` fields are the series' "action / selection" accent).
pub const PALETTE: lab_theme::Palette = lab_theme::Palette {
    ground: PAPER.base,
    ground_grid: PAPER,
    stage_grid: GRAPH,
    floor_grid: GRAPH,
    bar: BAR,
    panel: SIDE,
    panel_soft: SIDE,
    surface: GHOST,
    tile: WELL,
    tag: CONTROL,
    seg_on: SEG_ON,
    chip: CHIP,
    mode_on: SEG_ON,
    border: BORDER,
    border_hover: BORDER_STRONG,
    axis: TEXT_FAINT,
    text: TEXT,
    text_secondary: TEXT_DIM,
    text_mode: TEXT_MUTED,
    text_muted: TEXT_MUTED,
    text_code: TEXT_DIM,
    text_faint: TEXT_FAINT,
    on_light: TINT,
    amber: ACCENT,
    amber_hover: INK,
    amber_soft: INK,
    amber_bg: TINT,
    amber_border: EDGE,
    amber_text: INK,
    cyan: INFO,
    ..lab_theme::LAB_DARK
};

#[cfg(test)]
mod tests {
    use super::*;
    use lab_theme::contrast::{over, ratio};

    #[test]
    fn values_match_the_mockup_css() {
        assert_eq!(ACCENT, Color32::from_rgb(0xD6, 0xC7, 0xA1));
        assert_eq!(SIDE.to_srgba_unmultiplied(), [17, 17, 17, 214]);
        assert_eq!(PAPER.major.a(), 10);
        assert_eq!(TENSION, Color32::from_rgb(0x7F, 0xB0, 0xE0));
        assert_eq!(COMPRESSION, Color32::from_rgb(0xE0, 0x90, 0x5E));
    }

    /// Body text reaches 4.5:1 on every surface it sits on (WCAG AA).
    #[test]
    fn text_contrast_is_at_least_4_5() {
        let paper = PAPER.base;
        let surfaces = [
            ("paper", paper),
            ("bar", over(BAR, paper)),
            ("side", over(SIDE, paper)),
            ("panel", over(PANEL, paper)),
            ("well", WELL),
            ("ghost", GHOST),
            ("control", CONTROL),
            ("row", ROW),
            ("tile", over(TILE, paper)),
            ("graph", GRAPH.base),
            ("chip", over(CHIP, GRAPH.base)),
            ("overlay", over(OVERLAY, GRAPH.base)),
        ];
        let texts = [
            ("text", TEXT),
            ("text_body", TEXT_BODY),
            ("text_dim", TEXT_DIM),
            ("text_chip", TEXT_CHIP),
            ("text_muted", TEXT_MUTED),
            ("accent", ACCENT),
            ("ink", INK),
            ("ok", OK),
            ("warn", WARN),
            ("caution", CAUTION),
            ("danger_ink", DANGER_INK),
            ("tension", TENSION),
            ("compression", COMPRESSION),
            ("info", INFO),
        ];
        for (sn, s) in surfaces {
            for (tn, t) in texts {
                let r = ratio(t, s);
                assert!(r >= 4.5, "{tn} on {sn}: {r:.2}:1");
            }
        }
        for (n, t, s) in [
            ("ink on tint", INK, TINT),
            ("ink on tint hover", INK, TINT_HOVER),
            ("ink on tile_on", INK, TILE_ON),
            ("text_strong on seg_on", TEXT_STRONG, SEG_ON),
            ("self card text", SELF_CARD_TEXT, SELF_CARD),
        ] {
            let r = ratio(t, s);
            assert!(r >= 4.5, "{n}: {r:.2}:1");
        }
    }

    /// The danger red is used for strokes, dots and dashed members: 3:1 for graphics.
    #[test]
    fn danger_graphics_reach_3_to_1() {
        for (n, s) in [("graph", GRAPH.base), ("well", WELL), ("paper", PAPER.base)] {
            let r = ratio(DANGER, s);
            assert!(r >= 3.0, "danger on {n}: {r:.2}:1");
        }
    }

    #[test]
    fn shade_darkens() {
        assert_eq!(
            shade(hex(0x9AA3AD), 0.5),
            Color32::from_rgb(0x4D, 0x52, 0x57)
        );
    }
}
