//! Widgets of the Playground worlds' columns (Tower, Ground; usable by Materials), drawn to the
//! mockups' CSS (`Tower.dc.html`, `Ground.dc.html`). Added by the `worlds` agent.
//!
//! | widget | CSS |
//! |---|---|
//! | [`title_block`] | the main column's `h1` (Space Grotesk 600 22) + `p.mut` with the status `.pill` |
//! | [`status_pill`] | `.pill` with its `.dot` |
//! | [`picture_tiles`] | `.ptile` grid (2 columns, gap 8): an SVG icon or a soil swatch over a label |
//! | [`stepper`] | Floors: `−` · well (mono 20 accent + note) · `+` (44 px) |
//! | [`value_row`] | a slider's label / value row (`display: flex; justify-content: space-between`) |
//! | [`readout_grid`] | the aside's `.well` tiles: `.lbl` · mono 17 value · 11 px note |
//! | [`band_bar`] | the comfort / safety bar: bands, a marker and the tick row |
//! | [`compare_row`] | a "Same storm, four shapes" bar row |
//! | [`challenge`] + [`goal_row`] | the challenge `.panel` with its goal boxes |
//! | [`wrap_button`] | a full-width `.ghost` whose label wraps (scenarios) |
//! | [`stage_box`] | the stage's floating info box ("Wind at the top") |
//! | [`paint_path`] | SVG path data painted into a rect (tile icons) |

use super::icons::{fill_polygon, parse_path, stroke_polyline};
use super::text::{css_line_height, galley, galley_spaced, lbl_galley, text_top, wrapped};
use super::{hit, interact};
use eframe::egui::{
    Align2, Color32, CornerRadius, Painter, Pos2, Rect, Response, Sense, Stroke, StrokeKind, Ui,
    Vec2, WidgetInfo, accesskit::Role, pos2, vec2,
};
use lab_theme::font;
use lab_ui::paint::focus_ring_if;
use sp_theme as t;

/// Chrome's `line-height: normal` box for Inter at `size` px: `round(0.96875·size) +
/// round(0.2421875·size)` (the font's ascender / descender), e.g. 13 px → 16, 11 px → 14.
pub fn inter_line(size: f32) -> f32 {
    (0.96875 * size).round() + (0.2421875 * size).round()
}

/// Paints SVG path data `d` (in a `viewbox`-sized box) into `rect`, stroked and/or filled.
pub fn paint_path(
    painter: &Painter,
    d: &str,
    viewbox: Vec2,
    rect: Rect,
    stroke: Option<Stroke>,
    fill: Option<Color32>,
) {
    let sx = rect.width() / viewbox.x.max(1e-3);
    let sy = rect.height() / viewbox.y.max(1e-3);
    for sp in parse_path(d) {
        let pts: Vec<Pos2> = sp
            .points
            .iter()
            .map(|p| pos2(rect.left() + p.x * sx, rect.top() + p.y * sy))
            .collect();
        if let Some(c) = fill
            && sp.closed
        {
            fill_polygon(painter, &pts, c);
        }
        if let Some(s) = stroke {
            stroke_polyline(painter, &pts, sp.closed, s);
        }
    }
}

// ============================================================================================
// Title, pill

/// The `.pill`: `padding: 4px 10px; border-radius: 999px; border: 1px solid #2E2E2E;
/// background: #141414; font: 600 12px`, an 8 px dot and the label, both in `color`.
pub fn pill_size(ui: &Ui, label: &str) -> Vec2 {
    let g = galley(ui, label, font::semibold(12.0), t::TEXT);
    let line = css_line_height(ui, &font::semibold(12.0));
    vec2(2.0 + 20.0 + 8.0 + 6.0 + g.size().x, 2.0 + 8.0 + line)
}

/// Paints the status pill into `rect` (sized with [`pill_size`]). AccessKit: a `Label`
/// "Status: <label>".
pub fn status_pill(ui: &Ui, rect: Rect, label: &str, color: Color32) -> Response {
    let p = ui.painter();
    p.rect(
        rect,
        CornerRadius::same(t::radius::PILL),
        t::CONTROL,
        Stroke::new(1.0, t::BORDER_STRONG),
        StrokeKind::Inside,
    );
    let cy = rect.center().y;
    p.circle_filled(pos2(rect.left() + 11.0 + 4.0, cy), 4.0, color);
    let g = galley(ui, label, font::semibold(12.0), color);
    p.galley(pos2(rect.left() + 11.0 + 14.0, text_top(cy, &g)), g, color);
    let name = format!("Status: {label}");
    let resp = ui.interact(rect, ui.scope_id().with(("pill", label)), Sense::hover());
    resp.widget_info(|| WidgetInfo::labeled(Role::Label, true, &name));
    resp
}

/// The main column's heading row: `h1` (Space Grotesk 600 22) over the `p.mut` (13 px), with
/// the status pill on the right (`flex-wrap: wrap; justify-content: space-between; gap: 8px
/// 12px`; the pill wraps under the text in a narrow column).
pub fn title_block(ui: &mut Ui, title: &str, sub: &str, status: &str, color: Color32) {
    let width = ui.available_width();
    let pill = pill_size(ui, status);
    let hf = font::heading(22.0);
    // `letter-spacing: -0.01em`
    let h = galley_spaced(ui, title, hf.clone(), t::TEXT, -0.22);
    let text_w = (width - pill.x - 12.0).max(120.0);
    // CSS: the text block's width is its content (the sub-line can be long); it wraps only
    // when the line is full.
    let sub_full = galley(ui, sub, font::body(13.0), t::TEXT_MUTED);
    let content_w = h.size().x.max(sub_full.size().x);
    let side_by_side = content_w + 12.0 + pill.x <= width;
    let block_w = if side_by_side {
        content_w
    } else {
        width.min(text_w.max(width))
    };
    let s = wrapped(ui, sub, font::body(13.0), t::TEXT_MUTED, block_w, None);
    let h_line = css_line_height(ui, &hf);
    let s_line = css_line_height(ui, &font::body(13.0));
    let text_h = h_line + 2.0 + s.size().y.max(s_line);
    let total_h = if side_by_side {
        text_h.max(pill.y)
    } else {
        text_h + 8.0 + pill.y
    };
    let (rect, _) = ui.allocate_exact_size(vec2(width, total_h), Sense::hover());
    let top = if side_by_side {
        rect.top() + (total_h - text_h) / 2.0
    } else {
        rect.top()
    };
    let p = ui.painter();
    p.galley(
        pos2(rect.left(), text_top(top + h_line / 2.0, &h)),
        h,
        t::TEXT,
    );
    let sub_top = top + h_line + 2.0;
    p.galley(pos2(rect.left(), sub_top), s, t::TEXT_MUTED);
    let title_rect = Rect::from_min_size(pos2(rect.left(), top), vec2(block_w, h_line));
    let tr = ui.interact(
        title_rect,
        ui.scope_id().with(("h1", title)),
        Sense::hover(),
    );
    let ts = title.to_string();
    tr.widget_info(|| WidgetInfo::labeled(Role::Heading, true, &ts));
    let sr = ui.interact(
        Rect::from_min_size(pos2(rect.left(), sub_top), vec2(block_w, s_line)),
        ui.scope_id().with(("sub", title)),
        Sense::hover(),
    );
    let ss = sub.to_string();
    sr.widget_info(|| WidgetInfo::labeled(Role::Label, true, &ss));
    let pill_rect = if side_by_side {
        Align2::RIGHT_CENTER.anchor_size(pos2(rect.right(), rect.center().y), pill)
    } else {
        Rect::from_min_size(pos2(rect.left(), top + text_h + 8.0), pill)
    };
    status_pill(ui, pill_rect, status, color);
}

// ============================================================================================
// Picture tiles

/// What a picture tile shows above its label.
#[derive(Debug, Clone, Copy)]
pub enum TileArt<'a> {
    /// An SVG icon (`stroke="#D9D9D9" stroke-width="1.5"`) in a `viewbox`-sized box, with an
    /// optional path drawn first in another colour (the foundations' ground line).
    Icon {
        /// Path data.
        d: &'a str,
        /// The `<svg>` size (40 × 40, or 44 × 34 for foundations).
        viewbox: Vec2,
        /// A path painted under it, in its own colour.
        under: Option<(&'a str, Color32)>,
    },
    /// A soil swatch: `52 × 22`, radius 4, `inset 0 -3px 0 rgba(0,0,0,.25), 0 4px 0 <side>,
    /// 0 8px 10px rgba(0,0,0,.6)`.
    Swatch {
        /// The face.
        fill: Color32,
        /// The 2.5D side under it.
        side: Color32,
    },
}

impl TileArt<'_> {
    fn size(&self) -> Vec2 {
        match self {
            TileArt::Icon { viewbox, .. } => *viewbox,
            TileArt::Swatch { .. } => vec2(52.0, 22.0),
        }
    }

    fn paint(&self, painter: &Painter, rect: Rect) {
        match *self {
            TileArt::Icon { d, viewbox, under } => {
                if let Some((u, c)) = under {
                    paint_path(painter, u, viewbox, rect, Some(Stroke::new(1.5, c)), None);
                }
                paint_path(
                    painter,
                    d,
                    viewbox,
                    rect,
                    Some(Stroke::new(1.5, t::TEXT_DIM)),
                    None,
                );
            }
            TileArt::Swatch { fill, side } => {
                let r = CornerRadius::same(4);
                // drop shadow, then the side (offset 4 px down), then the face
                for (i, a) in [(10.0, 0.10), (6.0, 0.18), (3.0, 0.25)] {
                    painter.rect_filled(
                        rect.translate(vec2(0.0, 8.0)).expand(i * 0.5),
                        CornerRadius::same(8),
                        t::ground::SWATCH_SHADOW.gamma_multiply(a),
                    );
                }
                painter.rect_filled(rect.translate(vec2(0.0, 4.0)), r, side);
                painter.rect_filled(rect, r, fill);
                let inset = Rect::from_min_max(pos2(rect.left(), rect.bottom() - 3.0), rect.max);
                painter.rect_filled(
                    inset,
                    CornerRadius {
                        nw: 0,
                        ne: 0,
                        sw: 4,
                        se: 4,
                    },
                    t::ground::SWATCH_INSET,
                );
            }
        }
    }
}

/// A grid of `.ptile`s (2 columns, gap 8, min-height 84, radius 12, `padding: 10px 6px`,
/// art, gap 8, label 600 12 px / 1.25 centred). The selected tile has the accent border on
/// `#1E1C17`; hover the edge border. AccessKit: each a `Button` named by its label with its
/// pressed state (`aria-pressed`). Returns the index clicked.
pub fn picture_tiles(ui: &mut Ui, items: &[(&str, TileArt<'_>)], selected: usize) -> Option<usize> {
    let width = ui.available_width();
    let gap = 8.0;
    let col_w = ((width - gap) / 2.0).max(40.0);
    let lf = font::semibold(12.0);
    let line = 15.0; // 12 px × 1.25
    let mut clicked = None;
    let rows = items.len().div_ceil(2);
    let origin = ui.cursor().min;
    let mut y = origin.y;
    for r in 0..rows {
        // a row is as tall as its tallest tile (grid rows stretch)
        let cells: Vec<usize> = (r * 2..(r * 2 + 2).min(items.len())).collect();
        let heights: Vec<f32> = cells
            .iter()
            .map(|&i| {
                let g = wrapped(
                    ui,
                    items[i].0,
                    lf.clone(),
                    t::TEXT,
                    col_w - 14.0,
                    Some(line),
                );
                (2.0 + 20.0 + items[i].1.size().y + 8.0 + g.size().y).max(t::size::PTILE)
            })
            .collect();
        let row_h = heights.iter().copied().fold(t::size::PTILE, f32::max);
        for (k, &i) in cells.iter().enumerate() {
            let rect = Rect::from_min_size(
                pos2(origin.x + k as f32 * (col_w + gap), y),
                vec2(col_w, row_h),
            );
            let (label, art) = &items[i];
            let id = ui.scope_id().with(("ptile", *label));
            let on = i == selected;
            let resp = interact(ui, id, rect, true, Role::Button, label, Some(on));
            if resp.clicked() {
                clicked = Some(i);
            }
            if ui.is_rect_visible(rect) {
                let hovered = resp.hovered();
                let (fill, border) = if on {
                    (t::TILE_ON, t::ACCENT)
                } else if hovered {
                    (t::TILE_ON, t::EDGE)
                } else {
                    (t::TILE, t::BORDER)
                };
                let p = ui.painter();
                p.rect(
                    rect,
                    CornerRadius::same(t::radius::PANEL),
                    fill,
                    Stroke::new(1.0, border),
                    StrokeKind::Inside,
                );
                let g = wrapped(ui, label, lf.clone(), t::TEXT, col_w - 14.0, Some(line));
                let art_size = art.size();
                let content_h = art_size.y + 8.0 + g.size().y;
                let top = rect.center().y - content_h / 2.0;
                let art_rect =
                    Rect::from_min_size(pos2(rect.center().x - art_size.x / 2.0, top), art_size);
                art.paint(p, art_rect);
                let gy = top + art_size.y + 8.0;
                // centre each wrapped row
                let gx = rect.center().x - g.size().x / 2.0;
                p.galley(pos2(gx, gy), g, t::TEXT);
                focus_ring_if(ui, &resp, rect, CornerRadius::same(t::radius::PANEL));
            }
        }
        y += row_h + if r + 1 < rows { gap } else { 0.0 };
    }
    let used = Rect::from_min_max(origin, pos2(origin.x + width, y));
    ui.advance_cursor_after_rect(used);
    clicked
}

// ============================================================================================
// Stepper, value rows

fn square_button(ui: &Ui, rect: Rect, label: &str, name: &str, enabled: bool) -> Response {
    let id = ui.scope_id().with(("square", name));
    let resp = interact(ui, id, rect, enabled, Role::Button, name, None);
    let hovered = resp.hovered() && enabled;
    let p = ui.painter();
    p.rect(
        rect,
        CornerRadius::same(t::radius::BUTTON),
        if hovered { t::GHOST_HOVER } else { t::GHOST },
        Stroke::new(1.0, t::BORDER_STRONG),
        StrokeKind::Inside,
    );
    let mut fg = if hovered { t::TEXT_STRONG } else { t::TEXT_DIM };
    if !enabled {
        fg = fg.gamma_multiply(0.45);
    }
    let g = galley(ui, label, font::medium(18.0), fg);
    p.galley(
        pos2(
            rect.center().x - g.size().x / 2.0,
            text_top(rect.center().y, &g),
        ),
        g,
        fg,
    );
    focus_ring_if(ui, &resp, rect, CornerRadius::same(t::radius::BUTTON));
    resp
}

/// What the stepper's buttons did this frame.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct StepperResponse {
    /// `−` clicked.
    pub minus: bool,
    /// `+` clicked.
    pub plus: bool,
}

/// The floors stepper: a 44 × 44 `.ghost` `−` (font 18), a `.well` (min-height 44) with the
/// value in mono 20 accent and a muted 12 px note, a `+`. AccessKit: the buttons are named
/// `minus_name` / `plus_name`; the well is a `Label` "<value> <note>".
pub fn stepper(
    ui: &mut Ui,
    value: &str,
    note: &str,
    minus_name: &str,
    plus_name: &str,
    can: (bool, bool),
) -> StepperResponse {
    let width = ui.available_width();
    let h = t::size::MIN_TARGET;
    let (rect, _) = ui.allocate_exact_size(vec2(width, h), Sense::hover());
    let minus = Rect::from_min_size(rect.min, vec2(h, h));
    let plus = Rect::from_min_size(pos2(rect.right() - h, rect.top()), vec2(h, h));
    let well = Rect::from_min_max(
        pos2(minus.right() + 8.0, rect.top()),
        pos2(plus.left() - 8.0, rect.bottom()),
    );
    let minus_clicked = square_button(ui, minus, "−", minus_name, can.0).clicked();
    let p = ui.painter();
    p.rect(
        well,
        CornerRadius::same(t::radius::WELL),
        t::WELL,
        Stroke::new(1.0, t::BORDER),
        StrokeKind::Inside,
    );
    let vg = galley(ui, value, font::mono(20.0), t::ACCENT);
    let ng = galley(ui, note, font::body(12.0), t::TEXT_MUTED);
    let w = vg.size().x + 6.0 + ng.size().x;
    let x = well.center().x - w / 2.0;
    let cy = well.center().y;
    // baselines aligned (flex `align-items: center` on spans of different sizes: centred)
    p.galley(pos2(x, text_top(cy, &vg)), vg.clone(), t::ACCENT);
    p.galley(
        pos2(x + vg.size().x + 6.0, text_top(cy, &ng)),
        ng,
        t::TEXT_MUTED,
    );
    let label = format!("{value} {note}");
    let wr = ui.interact(
        well,
        ui.scope_id().with(("stepper-well", minus_name)),
        Sense::hover(),
    );
    wr.widget_info(|| WidgetInfo::labeled(Role::Label, true, &label));
    StepperResponse {
        minus: minus_clicked,
        plus: square_button(ui, plus, "+", plus_name, can.1).clicked(),
    }
}

/// The label font of a [`value_row`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowLabel {
    /// `font-size: 13px; font-weight: 600` (the bottom bars' sliders).
    Strong,
    /// `font-size: 13px` (Pile length).
    Plain,
    /// `.lbl` (Building).
    Eyebrow,
}

/// A slider's label / value row: the label on the left, the mono 12 px value in `color` on the
/// right, baselines aligned. AccessKit: a `Label` "<label>: <value>".
pub fn value_row(ui: &mut Ui, label: &str, style: RowLabel, value: &str, color: Color32) {
    let width = ui.available_width();
    let lg = match style {
        RowLabel::Strong => galley(ui, label, font::semibold(13.0), t::TEXT),
        RowLabel::Plain => galley(ui, label, font::body(13.0), t::TEXT),
        RowLabel::Eyebrow => lbl_galley(ui, label, t::TEXT_MUTED),
    };
    let lcolor = if style == RowLabel::Eyebrow {
        t::TEXT_MUTED
    } else {
        t::TEXT
    };
    let vg = galley(ui, value, font::mono(12.0), color);
    let base = |g: &eframe::egui::Galley| {
        g.rows.first().map_or(g.size().y, |r| {
            r.row.glyphs.first().map_or(r.size.y, |gl| gl.pos.y)
        })
    };
    let lh = match style {
        RowLabel::Eyebrow => css_line_height(ui, &font::semibold(11.0)).max(inter_line(11.0)),
        RowLabel::Strong => css_line_height(ui, &font::semibold(13.0)).max(inter_line(13.0)),
        RowLabel::Plain => css_line_height(ui, &font::body(13.0)).max(inter_line(13.0)),
    };
    let h = lh.max(vg.size().y);
    let (rect, _) = ui.allocate_exact_size(vec2(width, h), Sense::hover());
    // the label's baseline sits where Chrome puts it in its line box (half-leading)
    let lg_top = rect.top() + (lh - lg.size().y) / 2.0;
    let baseline = (lg_top + base(&lg)).max(rect.top() + base(&vg));
    let p = ui.painter();
    p.galley(pos2(rect.left(), baseline - base(&lg)), lg, lcolor);
    p.galley(
        pos2(rect.right() - vg.size().x, baseline - base(&vg)),
        vg,
        color,
    );
    let name = format!("{label}: {value}");
    let r = ui.interact(
        rect,
        ui.scope_id().with(("value-row", label)),
        Sense::hover(),
    );
    r.widget_info(|| WidgetInfo::labeled(Role::Label, true, &name));
}

// ============================================================================================
// Readouts

/// One readout tile.
#[derive(Debug, Clone, Copy)]
pub struct Readout<'a> {
    /// The `.lbl` eyebrow.
    pub key: &'a str,
    /// The mono 17 px value.
    pub value: &'a str,
    /// The muted 11 px note.
    pub note: &'a str,
    /// The value's colour.
    pub color: Color32,
}

/// The aside's 2-column grid of `.well` tiles (`gap: 8px`; each `padding: 10px 12px; gap:
/// 2px`: `.lbl`, mono 17 px value, 11 px muted note that wraps). Rows are as tall as their
/// tallest tile. AccessKit: each tile a `Label` "<key>: <value>, <note>".
pub fn readout_grid(ui: &mut Ui, tiles: &[Readout<'_>]) {
    let width = ui.available_width();
    let gap = 8.0;
    let col_w = ((width - gap) / 2.0).max(40.0);
    // Chrome's Inter (the variable font at small optical sizes) sets 11 px text a little wider
    // than ours: wrap the notes 3 px early so they break where the mockup's do.
    let inner_w = col_w - 2.0 - 24.0 - 3.0;
    let lbl_h = css_line_height(ui, &font::semibold(11.0));
    let vf = font::mono(17.0);
    let val_h = css_line_height(ui, &vf);
    let nf = font::body(11.0);
    let note_line = css_line_height(ui, &nf);
    let origin = ui.cursor().min;
    let mut y = origin.y;
    let rows = tiles.len().div_ceil(2);
    for r in 0..rows {
        let cells: Vec<usize> = (r * 2..(r * 2 + 2).min(tiles.len())).collect();
        let notes: Vec<_> = cells
            .iter()
            .map(|&i| {
                wrapped(
                    ui,
                    tiles[i].note,
                    nf.clone(),
                    t::TEXT_MUTED,
                    inner_w,
                    Some(note_line),
                )
            })
            .collect();
        let row_h = notes
            .iter()
            .map(|g| 2.0 + 20.0 + lbl_h + 2.0 + val_h + 2.0 + g.size().y)
            .fold(0.0, f32::max);
        for (k, &i) in cells.iter().enumerate() {
            let tile = &tiles[i];
            let rect = Rect::from_min_size(
                pos2(origin.x + k as f32 * (col_w + gap), y),
                vec2(col_w, row_h),
            );
            let p = ui.painter();
            p.rect(
                rect,
                CornerRadius::same(t::radius::WELL),
                t::WELL,
                Stroke::new(1.0, t::BORDER),
                StrokeKind::Inside,
            );
            let x = rect.left() + 13.0;
            let mut ty = rect.top() + 11.0;
            let lg = lbl_galley(ui, tile.key, t::TEXT_MUTED);
            p.galley(pos2(x, text_top(ty + lbl_h / 2.0, &lg)), lg, t::TEXT_MUTED);
            ty += lbl_h + 2.0;
            let vg = galley(ui, tile.value, vf.clone(), tile.color);
            p.galley(pos2(x, text_top(ty + val_h / 2.0, &vg)), vg, tile.color);
            ty += val_h + 2.0;
            p.galley(pos2(x, ty), notes[k].clone(), t::TEXT_MUTED);
            let name = format!("{}: {}, {}", tile.key, tile.value, tile.note);
            let resp = ui.interact(
                rect,
                ui.scope_id().with(("readout", tile.key)),
                Sense::hover(),
            );
            resp.widget_info(|| WidgetInfo::labeled(Role::Label, true, &name));
        }
        y += row_h + if r + 1 < rows { gap } else { 0.0 };
    }
    ui.advance_cursor_after_rect(Rect::from_min_max(origin, pos2(origin.x + width, y)));
}

/// The comfort / safety bar: a 14 px bar (radius 7) of `bands` (flex weight, colour), a
/// 4 × 18 marker at `marker_pct` % (`margin-left: -2px; top: -2px`), and the mono 10 px tick
/// row spread with `justify-content: space-between`. AccessKit: the bar is a `Label` `name`.
pub fn band_bar(
    ui: &mut Ui,
    bands: &[(f32, Color32)],
    marker_pct: f32,
    ticks: &[&str],
    name: &str,
) {
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(width, 14.0), Sense::hover());
    let p = ui.painter();
    let total: f32 = bands.iter().map(|b| b.0).sum::<f32>().max(1e-3);
    let mut x = rect.left();
    let n = bands.len();
    for (i, (w, c)) in bands.iter().enumerate() {
        let bw = rect.width() * w / total;
        let r = Rect::from_min_size(pos2(x, rect.top()), vec2(bw, rect.height()));
        let cr = CornerRadius {
            nw: if i == 0 { 7 } else { 0 },
            sw: if i == 0 { 7 } else { 0 },
            ne: if i + 1 == n { 7 } else { 0 },
            se: if i + 1 == n { 7 } else { 0 },
        };
        p.rect_filled(r, cr, *c);
        x += bw;
    }
    let mx = rect.left() + rect.width() * marker_pct.clamp(0.0, 100.0) / 100.0;
    // the marker is clipped by the bar's `overflow: hidden` to the bar's height
    let marker =
        Rect::from_min_size(pos2(mx - 2.0, rect.top() - 2.0), vec2(4.0, 18.0)).intersect(rect);
    p.rect_filled(marker, CornerRadius::same(2), t::tower::BAR_MARKER);
    let s = name.to_string();
    let r = ui.interact(rect, ui.scope_id().with(("band-bar", name)), Sense::hover());
    r.widget_info(|| WidgetInfo::labeled(Role::Label, true, &s));
    ui.add_space(8.0 - ui.spacing().item_spacing.y);
    let tf = font::mono(10.0);
    let gs: Vec<_> = ticks
        .iter()
        .map(|s| galley(ui, *s, tf.clone(), t::TEXT_MUTED))
        .collect();
    let line = css_line_height(ui, &tf);
    let (row, _) = ui.allocate_exact_size(vec2(width, line), Sense::hover());
    let tw: f32 = gs.iter().map(|g| g.size().x).sum();
    let gap = if gs.len() > 1 {
        (width - tw) / (gs.len() - 1) as f32
    } else {
        0.0
    };
    let mut x = row.left();
    for g in gs {
        let w = g.size().x;
        ui.painter()
            .galley(pos2(x, text_top(row.center().y, &g)), g, t::TEXT_MUTED);
        x += w + gap;
    }
}

/// A "Same storm, four shapes" row: the name (74 px, 12 px, 600 when selected), the `.bar`
/// track (8 px, `#1E1E1E`, radius 4) filled to `pct` % in `bar`, the mono value (min-width 64,
/// right-aligned). AccessKit: a `Label` "<name>: <value>".
pub fn compare_row(ui: &mut Ui, name: &str, selected: bool, pct: f32, bar: Color32, value: &str) {
    let width = ui.available_width();
    let nf = if selected {
        font::semibold(12.0)
    } else {
        font::body(12.0)
    };
    let nc = if selected {
        t::TEXT_STRONG
    } else {
        t::TEXT_MUTED
    };
    let ng = galley(ui, name, nf.clone(), nc);
    let vg = galley(ui, value, font::mono(12.0), t::TEXT);
    let h = css_line_height(ui, &nf).max(vg.size().y);
    let (rect, _) = ui.allocate_exact_size(vec2(width, h), Sense::hover());
    let cy = rect.center().y;
    let p = ui.painter();
    p.galley(pos2(rect.left(), text_top(cy, &ng)), ng, nc);
    let vw = vg.size().x.max(64.0);
    p.galley(
        pos2(rect.right() - vg.size().x, text_top(cy, &vg)),
        vg,
        t::TEXT,
    );
    let track = Rect::from_min_max(
        pos2(rect.left() + 74.0 + 10.0, cy - 4.0),
        pos2(rect.right() - vw - 10.0, cy + 4.0),
    );
    if track.width() > 0.0 {
        p.rect_filled(track, CornerRadius::same(4), t::TRACK);
        let fw = track.width() * (pct / 100.0).clamp(0.0, 1.0);
        if fw > 0.0 {
            let full = fw >= track.width() - 0.5;
            p.rect_filled(
                Rect::from_min_size(track.min, vec2(fw, 8.0)),
                CornerRadius {
                    nw: 4,
                    sw: 4,
                    ne: if full { 4 } else { 0 },
                    se: if full { 4 } else { 0 },
                },
                bar,
            );
        }
    }
    let s = format!("{name}: {value}");
    let r = ui.interact(rect, ui.scope_id().with(("compare", name)), Sense::hover());
    r.widget_info(|| WidgetInfo::labeled(Role::Label, true, &s));
}

/// One challenge goal: an 18 × 18 box (radius 5, 1.5 px border; done: filled `#5FBF8A`) and
/// the 13 px text (done `#EAEAEA`, else muted), `gap: 10px`. AccessKit: a read-only
/// `CheckBox` named by the text, checked when done.
pub fn goal_row(ui: &mut Ui, text: &str, done: bool) {
    let width = ui.available_width();
    let tc = if done { t::TEXT } else { t::TEXT_MUTED };
    let g = wrapped(ui, text, font::body(13.0), tc, width - 28.0, None);
    let h = g.size().y.max(18.0);
    let (rect, resp) = ui.allocate_exact_size(vec2(width, h), Sense::hover());
    let p = ui.painter();
    let bx = Rect::from_min_size(pos2(rect.left(), rect.center().y - 9.0), vec2(18.0, 18.0));
    let c = if done { t::OK } else { t::tower::GOAL_OPEN };
    p.rect(
        bx,
        CornerRadius::same(5),
        if done { t::OK } else { Color32::TRANSPARENT },
        Stroke::new(1.5, c),
        StrokeKind::Inside,
    );
    p.galley(
        pos2(bx.right() + 10.0, rect.center().y - g.size().y / 2.0),
        g,
        tc,
    );
    let s = text.to_string();
    resp.widget_info(|| WidgetInfo::selected(Role::CheckBox, true, done, &s));
}

/// The challenge `.panel` (`padding: 12px; gap: 8px`): the `h2.h` title (Space Grotesk 600
/// 15) with the mono 11 px rung ("7 / 12") on the right, then `body` (description, goals).
/// AccessKit: a `Group` named `title`.
pub fn challenge(ui: &mut Ui, title: &'static str, rung: &str, body: impl FnOnce(&mut Ui)) {
    super::Card::panel()
        .padding(12.0)
        .gap(8.0)
        .label(title)
        .show(ui, |ui| {
            let width = ui.available_width();
            let tf = font::heading(15.0);
            let tg = galley_spaced(ui, title, tf.clone(), t::TEXT, -0.15);
            let rg = galley(ui, rung, font::mono(11.0), t::TEXT_MUTED);
            let h = css_line_height(ui, &tf);
            let (rect, _) = ui.allocate_exact_size(vec2(width, h), Sense::hover());
            let p = ui.painter();
            p.galley(
                pos2(rect.left(), text_top(rect.center().y, &tg)),
                tg,
                t::TEXT,
            );
            p.galley(
                pos2(rect.right() - rg.size().x, text_top(rect.center().y, &rg)),
                rg,
                t::TEXT_MUTED,
            );
            let s = format!("{title} {rung}");
            let r = ui.interact(
                rect,
                ui.scope_id().with(("challenge", title)),
                Sense::hover(),
            );
            r.widget_info(|| WidgetInfo::labeled(Role::Heading, true, &s));
            body(ui);
        });
}

/// A full-width `.ghost` (`justify-content: flex-start; min-height: 40px`) whose label wraps
/// (wrapped lines centre like Chrome's button text). AccessKit: a `Button` named `label`.
pub fn wrap_button(ui: &mut Ui, label: &str, min_h: f32) -> Response {
    let width = ui.available_width();
    let f = font::medium(13.0);
    let g = wrapped(ui, label, f.clone(), t::TEXT_DIM, width - 26.0, None);
    let h = min_h.max(g.size().y + 8.0);
    let (rect, _) = ui.allocate_exact_size(vec2(width, h), Sense::hover());
    let id = ui.scope_id().with(("wrap-button", label));
    let resp = interact(ui, id, hit::target(rect), true, Role::Button, label, None);
    let hovered = resp.hovered();
    let p = ui.painter();
    p.rect(
        rect,
        CornerRadius::same(t::radius::BUTTON),
        if hovered { t::GHOST_HOVER } else { t::GHOST },
        Stroke::new(1.0, t::BORDER_STRONG),
        StrokeKind::Inside,
    );
    let fg = if hovered { t::TEXT_STRONG } else { t::TEXT_DIM };
    let one_line = g.rows.len() <= 1;
    let x = if one_line {
        rect.left() + 13.0
    } else {
        rect.center().x - g.size().x / 2.0
    };
    let g = wrapped(ui, label, f, fg, width - 26.0, None);
    p.galley(pos2(x, rect.center().y - g.size().y / 2.0), g, fg);
    focus_ring_if(ui, &resp, rect, CornerRadius::same(t::radius::BUTTON));
    resp
}

/// The stage's floating info box at `pos` (its top-left): `padding: 8px 10px; radius 8;
/// rgba(10,10,10,.82)` with a `#242424` border, a `.lbl` over a mono 16 px value in `color`.
/// AccessKit: a `Label` "<label>: <value>".
pub fn stage_box(ui: &Ui, pos: Pos2, label: &str, value: &str, color: Color32) -> Rect {
    let lg = lbl_galley(ui, label, t::TEXT_MUTED);
    let vf = font::mono(16.0);
    let vg = galley(ui, value, vf.clone(), color);
    let lh = css_line_height(ui, &font::semibold(11.0));
    let vh = css_line_height(ui, &vf);
    let w = lg.size().x.max(vg.size().x) + 22.0;
    let h = lh + 2.0 + vh + 18.0;
    let rect = Rect::from_min_size(pos, vec2(w, h));
    let p = ui.painter();
    p.rect(
        rect,
        CornerRadius::same(t::radius::WELL),
        t::tower::STAGE_BOX,
        Stroke::new(1.0, t::BORDER),
        StrokeKind::Inside,
    );
    let x = rect.left() + 11.0;
    p.galley(
        pos2(x, text_top(rect.top() + 9.0 + lh / 2.0, &lg)),
        lg,
        t::TEXT_MUTED,
    );
    p.galley(
        pos2(x, text_top(rect.top() + 9.0 + lh + 2.0 + vh / 2.0, &vg)),
        vg,
        color,
    );
    let s = format!("{label}: {value}");
    let r = ui.interact(
        rect,
        ui.scope_id().with(("stage-box", label)),
        Sense::hover(),
    );
    r.widget_info(|| WidgetInfo::labeled(Role::Label, true, &s));
    rect
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tile_art_sizes() {
        let icon = TileArt::Icon {
            d: "M0 0H40",
            viewbox: vec2(40.0, 40.0),
            under: None,
        };
        assert_eq!(icon.size(), vec2(40.0, 40.0));
        let sw = TileArt::Swatch {
            fill: t::ACCENT,
            side: t::EDGE,
        };
        assert_eq!(sw.size(), vec2(52.0, 22.0));
    }
}
