//! Widgets kept from the v1 Tower / Ground columns for the builder's fold-ins (CMF-1 comfort
//! meter, CMF-2 "Same storm, four shapes"), drawn to the v1 mockups' CSS (`Tower.dc.html`).
//! The rest of the v1 world widgets went with their screens (git history before v2).
//!
//! | widget | CSS |
//! |---|---|
//! | [`band_bar`] | the comfort / safety bar: bands, a marker and the tick row |
//! | [`compare_row`] | a "Same storm, four shapes" bar row |

use super::text::{css_line_height, galley, text_top};
use eframe::egui::{
    Color32, CornerRadius, Rect, Sense, Ui, WidgetInfo, accesskit::Role, pos2, vec2,
};
use lab_theme::font;
use sp_theme as t;

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
