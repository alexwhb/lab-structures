//! Under the stage (BLD-1, PROV-1): the legend ("Pulled", "Pushed", "Slack / idle", "Will
//! break", "Brighter means closer to breaking.") and the zoom controls (−, zoom %, +, Fit) on
//! one wrapping row (`justify-content: space-between; gap: 8px 16px`), then the quiet
//! provenance line with its coloured dots.

use super::Action;
use super::bits::label_node;
use super::paint;
use crate::screens::Provenance;
use crate::widgets::ghost;
use crate::widgets::text::{css_line_height, galley, text_top};
use eframe::egui::{Color32, CornerRadius, Rect, Ui, UiBuilder, accesskit::Role, pos2, vec2};
use sp_theme::{self as t, builder as b};

/// The legend's swatches.
#[derive(Clone, Copy)]
enum Swatch {
    Bar(Color32),
    Dashed,
    None,
}

const LEGEND: [(Swatch, &str); 5] = [
    (Swatch::Bar(t::TENSION), "Pulled"),
    (Swatch::Bar(t::COMPRESSION), "Pushed"),
    (Swatch::Bar(b::LEGEND_IDLE), "Slack / idle"),
    (Swatch::Dashed, "Will break"),
    (Swatch::None, "Brighter means closer to breaking."),
];

/// PROV-1: the builder's provenance line.
pub const PROVENANCE: [(Provenance, &str); 4] = [
    (
        Provenance::Simplified,
        "Physics: simplified dynamic settling",
    ),
    (Provenance::Simplified, "Materials: representative"),
    (
        Provenance::Illustrative,
        "Soil, weather, fire: illustrative",
    ),
    (Provenance::Illustrative, "Game costs"),
];

/// Flex-wrap lines of items with widths `ws` in `max_w` (gap `gap`): the index ranges per line.
fn wrap(ws: &[f32], max_w: f32, gap: f32) -> Vec<std::ops::Range<usize>> {
    let mut lines = Vec::new();
    let (mut start, mut x) = (0, 0.0f32);
    for (i, w) in ws.iter().enumerate() {
        if i > start && x + gap + w > max_w {
            lines.push(start..i);
            start = i;
            x = *w;
        } else {
            x = if i == start { *w } else { x + gap + w };
        }
    }
    if start < ws.len() {
        lines.push(start..ws.len());
    }
    lines
}

/// Draws the legend row and the provenance line at the cursor, full width.
pub fn show(ui: &mut Ui, zoom_text: &str, actions: &mut Vec<Action>) {
    let w = ui.available_width();
    let top = ui.cursor().min;
    let font = lab_theme::font::body(12.0);
    let lh = css_line_height(ui, &font);
    // the legend items
    let galleys: Vec<_> = LEGEND
        .iter()
        .map(|(s, text)| {
            let c = if matches!(s, Swatch::None) {
                t::TEXT_MUTED
            } else {
                t::TEXT_DIM
            };
            (galley(ui, *text, font.clone(), c), c)
        })
        .collect();
    let item_ws: Vec<f32> = LEGEND
        .iter()
        .zip(&galleys)
        .map(|((s, _), (g, _))| g.size().x + if matches!(s, Swatch::None) { 0.0 } else { 24.0 })
        .collect();
    // the zoom group: − (36), % (min 48), + (36), Fit; gap 4
    let fit = ghost("Fit");
    let fit_w = fit.desired_size(ui).x;
    let mono = lab_theme::font::mono(12.0);
    let zg = galley(ui, zoom_text, mono, b::HINT_TEXT);
    let pct_w = zg.size().x.max(48.0);
    let zoom_w = 36.0 + 4.0 + pct_w + 4.0 + 36.0 + 4.0 + fit_w;
    let zoom_h = t::size::BUTTON;
    // the legend block's own wrapping (gap 6 14) at its shrink-to-fit width
    let legend_full: f32 = item_ws.iter().sum::<f32>() + 14.0 * (item_ws.len() - 1) as f32;
    let side_by_side = legend_full + 16.0 + zoom_w <= w;
    let legend_w = if side_by_side {
        legend_full
    } else {
        w.min(legend_full)
    };
    let lines = wrap(&item_ws, legend_w, 14.0);
    let legend_h = lines.len() as f32 * lh + 6.0 * (lines.len().saturating_sub(1)) as f32;
    let row_h = if side_by_side {
        legend_h.max(zoom_h)
    } else {
        legend_h + 8.0 + zoom_h
    };
    let legend_top = if side_by_side {
        top.y + (row_h - legend_h) / 2.0
    } else {
        top.y
    };
    let painter = ui.painter().clone();
    let legend_rect = Rect::from_min_size(pos2(top.x, legend_top), vec2(legend_w, legend_h));
    let mut y = legend_top;
    for line in &lines {
        let mut x = top.x;
        let cy = y + lh / 2.0;
        for i in line.clone() {
            let (g, c) = &galleys[i];
            match LEGEND[i].0 {
                Swatch::Bar(col) => {
                    painter.rect_filled(
                        Rect::from_min_size(pos2(x, cy - 2.0), vec2(18.0, 4.0)),
                        CornerRadius::same(2),
                        col,
                    );
                    x += 24.0;
                }
                Swatch::Dashed => {
                    // `border-top: 3px dashed`: Chrome draws two 6 px dashes in 18 px
                    paint::dashed(
                        &painter,
                        &[pos2(x, cy), pos2(x + 18.0, cy)],
                        3.0,
                        t::DANGER,
                        6.0,
                        6.0,
                        0.0,
                        false,
                    );
                    x += 24.0;
                }
                Swatch::None => {}
            }
            painter.galley(pos2(x, text_top(cy, g)), g.clone(), *c);
            x += g.size().x + 14.0;
        }
        y += lh + 6.0;
    }
    label_node(
        ui,
        "builder-legend",
        legend_rect,
        "Pulled, Pushed, Slack / idle, Will break. Brighter means closer to breaking.",
    );
    // the zoom group (`role="group" aria-label="Zoom"`): right of the legend, or at the start
    // of its own line when the row wraps (`justify-content: space-between`)
    let zx = if side_by_side {
        top.x + w - zoom_w
    } else {
        top.x
    };
    let zy = if side_by_side {
        top.y + (row_h - zoom_h) / 2.0
    } else {
        top.y + legend_h + 8.0
    };
    let zrect = Rect::from_min_size(pos2(zx, zy), vec2(zoom_w, zoom_h));
    let mut zui = ui.new_child(
        UiBuilder::new()
            .max_rect(zrect)
            .accessibility_role(Role::Group)
            .accessibility_label("Zoom"),
    );
    crate::widgets::set_bounds(&zui, zrect);
    let mut x = zx;
    let minus = Rect::from_min_size(pos2(x, zy), vec2(36.0, zoom_h));
    let plus = Rect::from_min_size(pos2(x + 40.0 + pct_w + 4.0, zy), vec2(36.0, zoom_h));
    let fit_r = Rect::from_min_size(pos2(plus.right() + 4.0, zy), vec2(fit_w, zoom_h));
    let targets = crate::widgets::hit::tile(&[minus, plus, fit_r]);
    if ghost("−")
        .centered()
        .accessible_name("Zoom out")
        .hit(targets[0])
        .show_at(&mut zui, minus)
        .clicked()
    {
        actions.push(Action::ZoomOut);
    }
    x += 40.0;
    let pct = Rect::from_min_size(pos2(x, zy), vec2(pct_w, zoom_h));
    zui.painter().galley(
        pos2(
            pct.center().x - zg.size().x / 2.0,
            text_top(pct.center().y, &zg),
        ),
        zg.clone(),
        b::HINT_TEXT,
    );
    label_node(&zui, "builder-zoom-pct", pct, &format!("Zoom {zoom_text}"));
    if ghost("+")
        .centered()
        .accessible_name("Zoom in")
        .hit(targets[1])
        .show_at(&mut zui, plus)
        .clicked()
    {
        actions.push(Action::ZoomIn);
    }
    if ghost("Fit")
        .hit(targets[2])
        .show_at(&mut zui, fit_r)
        .clicked()
    {
        actions.push(Action::Fit);
    }
    let row = Rect::from_min_size(top, vec2(w, row_h));
    ui.advance_cursor_after_rect(row);
    provenance(ui);
}

/// PROV-1: one quiet line of dot + text items (11 px, muted), wrapping.
fn provenance(ui: &mut Ui) {
    let w = ui.available_width();
    let top = ui.cursor().min;
    let font = lab_theme::font::body(11.0);
    let lh = css_line_height(ui, &font);
    let gs: Vec<_> = PROVENANCE
        .iter()
        .map(|(_, s)| galley(ui, *s, font.clone(), t::TEXT_MUTED))
        .collect();
    let ws: Vec<f32> = gs.iter().map(|g| 14.0 + g.size().x).collect();
    let lines = wrap(&ws, w, 14.0);
    let h = lines.len() as f32 * lh + 4.0 * (lines.len().saturating_sub(1)) as f32;
    let rect = Rect::from_min_size(top, vec2(w, h));
    let child = ui.new_child(
        UiBuilder::new()
            .max_rect(rect)
            .accessibility_role(Role::Group)
            .accessibility_label("Where the numbers come from"),
    );
    crate::widgets::set_bounds(&child, rect);
    let painter = child.painter();
    let mut y = top.y;
    for line in lines {
        let mut x = top.x;
        let cy = y + lh / 2.0;
        for i in line {
            let (kind, text) = PROVENANCE[i];
            painter.circle_filled(pos2(x + 4.0, cy), 4.0, kind.color());
            let g = gs[i].clone();
            let r = Rect::from_min_size(pos2(x + 14.0, text_top(cy, &g)), g.size());
            painter.galley(r.min, g, t::TEXT_MUTED);
            label_node(&child, ("builder-prov", i), r, text);
            x += ws[i] + 14.0;
        }
        y += lh + 4.0;
    }
    ui.advance_cursor_after_rect(rect);
}

#[cfg(test)]
mod tests {
    use super::wrap;

    #[test]
    fn wraps_like_flex() {
        assert_eq!(wrap(&[10.0, 10.0, 10.0], 100.0, 5.0), vec![0..3]);
        assert_eq!(wrap(&[60.0, 50.0, 10.0], 100.0, 5.0), vec![0..1, 1..3]);
        assert_eq!(wrap(&[200.0, 10.0], 100.0, 5.0), vec![0..1, 1..2]);
    }
}
