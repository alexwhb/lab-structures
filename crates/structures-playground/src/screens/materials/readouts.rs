//! MT-6 — the right column (`<aside aria-label="Readouts">`): the six readout tiles, Strength
//! per kilogram bars, "Challenge · Break all five" with chips that light as each sample
//! breaks, and What's happening.

use super::draw::{lbl_wrapped, paint_pill, para, pill_size};
use crate::widgets::flow::{FlowItem, flow};
use crate::widgets::text::{css_line_height, galley, text_top};
use crate::widgets::{Callout, set_bounds};
use eframe::egui::{
    CornerRadius, Galley, Rect, Sense, Stroke, StrokeKind, Ui, UiBuilder, WidgetInfo,
    accesskit::Role, pos2, vec2,
};
use sp_engine::materials::View;
use sp_engine::materials::view::Tile;
use sp_theme as t;
use sp_theme::materials as mt;
use std::sync::Arc;

/// Wrapping slack for the tile texts (see [`tiles`]).
const CHROME_SLACK: f32 = 2.0;

/// Draws the column.
pub fn show(ui: &mut Ui, v: &View) {
    tiles(ui, &v.tiles);
    strength_per_kg(ui, v);
    challenge(ui, v);
    Callout::happening(&v.explain).show(ui);
}

/// One tile's galleys: `.lbl`, mono 17 value, 11 px muted note (each wraps).
struct TileText {
    key: Arc<Galley>,
    value: Arc<Galley>,
    note: Option<Arc<Galley>>,
}

impl TileText {
    fn height(&self) -> f32 {
        self.key.size().y
            + 2.0
            + self.value.size().y
            + 2.0
            + self.note.as_ref().map_or(0.0, |n| n.size().y)
    }
}

/// The six tiles: `grid-template-columns: repeat(2, 1fr); gap: 8px`, each a `.well` with
/// `padding: 10px 12px; gap: 2px`; rows take their taller tile's height.
fn tiles(ui: &mut Ui, tiles: &[Tile]) {
    let width = ui.available_width();
    let gap = 8.0;
    let col_w = ((width - gap) / 2.0).max(0.0);
    // Chrome lays these 11 px notes out a hair wider than egui ("area under the curve" wraps
    // in the reference): wrap 2 px early so the tiles break where the mockup's do.
    let inner = (col_w - 2.0 - 24.0 - CHROME_SLACK).max(1.0);
    let key_font = lab_theme::font::semibold(11.0);
    let val_font = lab_theme::font::mono(17.0);
    let note_font = lab_theme::font::body(11.0);
    let (kh, vh, nh) = (
        css_line_height(ui, &key_font),
        css_line_height(ui, &val_font),
        css_line_height(ui, &note_font),
    );
    let texts: Vec<TileText> = tiles
        .iter()
        .map(|tile| TileText {
            key: css_rows(lbl_wrapped(ui, tile.key, t::TEXT_MUTED, inner), kh),
            value: para(ui, &tile.value, val_font.clone(), t::TEXT, inner, Some(vh)),
            note: (!tile.note.is_empty()).then(|| {
                para(
                    ui,
                    tile.note,
                    note_font.clone(),
                    t::TEXT_MUTED,
                    inner,
                    Some(nh),
                )
            }),
        })
        .collect();
    let rows = tiles.len().div_ceil(2);
    let row_h: Vec<f32> = (0..rows)
        .map(|r| {
            texts[2 * r..(2 * r + 2).min(texts.len())]
                .iter()
                .map(|x| x.height() + 20.0 + 2.0)
                .fold(0.0, f32::max)
        })
        .collect();
    let height = row_h.iter().sum::<f32>() + gap * rows.saturating_sub(1) as f32;
    let (grid, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
    let mut y = grid.top();
    for (r, &rh) in row_h.iter().enumerate() {
        for c in 0..2 {
            let i = 2 * r + c;
            let (Some(tile), Some(x)) = (tiles.get(i), texts.get(i)) else {
                continue;
            };
            let rect = Rect::from_min_size(
                pos2(grid.left() + c as f32 * (col_w + gap), y),
                vec2(col_w, rh),
            );
            let p = ui.painter();
            p.rect(
                rect,
                CornerRadius::same(t::radius::WELL),
                t::WELL,
                Stroke::new(1.0, t::BORDER),
                StrokeKind::Inside,
            );
            let left = rect.left() + 13.0;
            let mut ty = rect.top() + 11.0;
            p.galley(pos2(left, ty), x.key.clone(), t::TEXT_MUTED);
            ty += x.key.size().y + 2.0;
            p.galley(pos2(left, ty), x.value.clone(), t::TEXT);
            ty += x.value.size().y + 2.0;
            if let Some(n) = &x.note {
                p.galley(pos2(left, ty), n.clone(), t::TEXT_MUTED);
            }
            let resp = ui.interact(rect, ui.scope_id().with(("tile", i)), Sense::hover());
            let label = if tile.note.is_empty() {
                format!("{}: {}", tile.key, tile.value)
            } else {
                format!("{}: {}, {}", tile.key, tile.value, tile.note)
            };
            resp.widget_info(|| WidgetInfo::labeled(Role::Label, true, &label));
        }
        y += rh + gap;
    }
}

/// Re-places a wrapped galley on CSS line boxes.
fn css_rows(g: Arc<Galley>, line_h: f32) -> Arc<Galley> {
    crate::widgets::text::css_rows(g, line_h)
}

/// "Strength per kilogram": rows of name (86 px), bar (8 px track, flex 1) and mono value
/// (min-width 52, right-aligned), `gap: 10px`, then the note.
fn strength_per_kg(ui: &mut Ui, v: &View) {
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.y = 8.0;
        crate::widgets::lbl(ui, "Strength per kilogram");
        let width = ui.available_width();
        let name_w = 86.0;
        let val_w = 52.0;
        let row_h = 16.0;
        for bar in &v.spec {
            let (rect, resp) = ui.allocate_exact_size(vec2(width, row_h), Sense::hover());
            let cy = rect.center().y;
            let (font, colour) = if bar.selected {
                (lab_theme::font::semibold(12.0), mt::BAR_NAME_ON)
            } else {
                (lab_theme::font::body(12.0), mt::BAR_NAME_OFF)
            };
            let p = ui.painter();
            let ng = galley(ui, bar.name, font, colour);
            p.galley(pos2(rect.left(), text_top(cy, &ng)), ng, colour);
            let vg = galley(ui, &bar.label, lab_theme::font::mono(12.0), t::TEXT);
            p.galley(
                pos2(rect.right() - vg.size().x, text_top(cy, &vg)),
                vg,
                t::TEXT,
            );
            let track = Rect::from_min_max(
                pos2(rect.left() + name_w + 10.0, cy - 4.0),
                pos2(rect.right() - val_w - 10.0, cy + 4.0),
            );
            p.rect_filled(track, CornerRadius::same(4), t::TRACK);
            let fill_w = track.width() * bar.fill as f32;
            if fill_w > 0.0 {
                let fill = Rect::from_min_size(track.min, vec2(fill_w, track.height()));
                let c = if bar.selected {
                    mt::BAR_ON
                } else {
                    mt::BAR_OFF
                };
                // `overflow: hidden` on the rounded track clips the square fill.
                p.with_clip_rect(fill.intersect(ui.clip_rect()))
                    .rect_filled(track, CornerRadius::same(4), c);
            }
            let label = format!("{}: {} kN\u{b7}m/kg", bar.name, bar.label);
            resp.widget_info(|| WidgetInfo::labeled(Role::Label, true, &label));
        }
        let note_font = lab_theme::font::body(11.0);
        let note_h = css_line_height(ui, &note_font);
        crate::widgets::text::text_block(
            ui,
            "Peak strength \u{f7} density, kN\u{b7}m/kg. Why aircraft love carbon fiber.",
            note_font,
            t::TEXT_MUTED,
            Some(note_h),
        );
    });
}

/// "Challenge · Break all five": a `.panel` (padding 12, gap 8) with the `.h` title and the
/// rung, one `.pill` per sample (green once broken), and the note.
fn challenge(ui: &mut Ui, v: &View) {
    let width = ui.available_width();
    let inner_w = width - 2.0 - 24.0;
    let title_font = lab_theme::font::heading(15.0);
    let title_h = css_line_height(ui, &title_font);
    let tg = galley(ui, "Challenge \u{b7} Break all five", title_font, t::TEXT);
    let rg = galley(
        ui,
        &v.challenge_rung,
        lab_theme::font::mono(11.0),
        t::TEXT_MUTED,
    );
    let names: Vec<String> = v.challenge.iter().map(|c| c.name.to_string()).collect();
    let items: Vec<FlowItem> = names
        .iter()
        .map(|n| FlowItem::fixed(pill_size(ui, n)))
        .collect();
    let f = flow(&items, inner_w, vec2(6.0, 6.0), 0.0);
    let note_font = lab_theme::font::body(12.0);
    let ng = para(
        ui,
        "Then guess which one broke with the least warning.",
        note_font.clone(),
        t::TEXT_MUTED,
        inner_w,
        Some(css_line_height(ui, &note_font)),
    );
    let h = 2.0 + 24.0 + title_h + 8.0 + f.height + 8.0 + ng.size().y;
    let (rect, _) = ui.allocate_exact_size(vec2(width, h), Sense::hover());
    let panel = ui.new_child(
        UiBuilder::new()
            .max_rect(rect)
            .accessibility_role(Role::Group)
            .accessibility_label("Challenge: Break all five"),
    );
    set_bounds(&panel, rect);
    let p = panel.painter();
    p.rect(
        rect,
        CornerRadius::same(t::radius::PANEL),
        t::PANEL,
        Stroke::new(1.0, t::BORDER),
        StrokeKind::Inside,
    );
    let (left, top) = (rect.left() + 13.0, rect.top() + 13.0);
    let cy = top + title_h / 2.0;
    p.galley(pos2(left, text_top(cy, &tg)), tg, t::TEXT);
    p.galley(
        pos2(rect.right() - 13.0 - rg.size().x, text_top(cy, &rg)),
        rg,
        t::TEXT_MUTED,
    );
    let chips_top = top + title_h + 8.0;
    for (chip, r) in v.challenge.iter().zip(&f.rects) {
        let r = r.translate(vec2(left, chips_top));
        let (text, dot) = if chip.lit {
            (mt::CHIP_LIT, mt::CHIP_LIT)
        } else {
            (mt::CHIP_TEXT_OFF, mt::CHIP_DOT_OFF)
        };
        paint_pill(&panel, r, chip.name, text, dot);
        let resp = panel.interact(
            r,
            panel.scope_id().with(("chip", chip.name)),
            Sense::hover(),
        );
        let label = format!(
            "{}: {}",
            chip.name,
            if chip.lit { "broken" } else { "not broken yet" }
        );
        resp.widget_info(|| WidgetInfo::labeled(Role::Label, true, &label));
    }
    p.galley(pos2(left, chips_top + f.height + 8.0), ng, t::TEXT_MUTED);
}
