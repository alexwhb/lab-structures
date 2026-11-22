//! The aside (ADV-6, ADV-7, PROV-1): "Member results" sorted by use (`max-height: 470px;
//! overflow: auto`, a 12 px table: ID · Nodes · N kN · σ MPa · Limit · Use), the Checks panel
//! (a dot, the label in Inter 600 13, the value in mono, the note in 12 px muted;
//! `line-height: 1.45`), and the provenance badges.

use super::controls::{HEAD_PAD, h2_font, panel_header};
use super::elevation::level_color;
use crate::Badge;
use crate::widgets::badge::{BadgeLook, badge_size, paint_badge};
use crate::widgets::text::{galley, text_top};
use eframe::egui::{
    Align, Color32, Rect, ScrollArea, Sense, Stroke, Ui, WidgetInfo,
    accesskit::Role,
    epaint::text::{LayoutJob, TextFormat, TextWrapping},
    pos2, vec2,
};
use lab_theme::font;
use sp_engine::influence::{Report, Tone, UseTone};
use sp_theme::{self as t, advanced as a};

/// `max-height: 470px` of the table's scroller.
pub const TABLE_MAX_H: f32 = 470.0;
/// Header cells `padding: 8px …`, body cells `padding: 6px …` around a 12 px line.
const HEAD_H: f32 = 31.0;
const ROW_H: f32 = 29.0;

fn n_color(tone: Tone) -> Color32 {
    match tone {
        Tone::Tension => a::LABEL_TENSION,
        Tone::Compression => a::LABEL_COMPRESSION,
        _ => a::MUTED,
    }
}

fn use_color(u: UseTone) -> Color32 {
    match u {
        UseTone::Normal => a::USE,
        UseTone::Near => a::USE_NEAR,
        UseTone::Over => a::USE_OVER,
    }
}

/// The Member results panel's content.
pub fn table_panel(ui: &mut Ui, report: &Report) {
    let note = galley(ui, "sorted by use", font::body(12.0), t::TEXT_MUTED);
    let nsize = note.size();
    panel_header(ui, "Member results", nsize, |ui, r| {
        ui.painter().galley(
            pos2(r.left(), text_top(r.center().y, &note)),
            note.clone(),
            t::TEXT_MUTED,
        );
    });
    let width = ui.available_width();
    // column widths: content (with padding) + the rest shared in proportion (auto table layout)
    let heads = ["ID", "Nodes", "N kN", "σ MPa", "Limit", "Use"];
    let pads = [
        (10.0, 10.0),
        (6.0, 6.0),
        (6.0, 6.0),
        (6.0, 6.0),
        (6.0, 6.0),
        (6.0, 10.0),
    ];
    let mut widths: Vec<f32> = heads
        .iter()
        .map(|h| galley(ui, *h, font::semibold(12.0), t::TEXT).size().x)
        .collect();
    let cells = |r: &sp_engine::influence::Row| {
        [
            r.id.clone(),
            r.nodes.clone(),
            r.force.clone(),
            r.stress.clone(),
            r.limit.clone(),
            r.utilisation.clone(),
        ]
    };
    for row in &report.rows {
        for (k, c) in cells(row).iter().enumerate() {
            let w = galley(ui, c.as_str(), font::mono(12.0), t::TEXT).size().x;
            widths[k] = widths[k].max(w);
        }
    }
    for (k, w) in widths.iter_mut().enumerate() {
        *w += pads[k].0 + pads[k].1;
    }
    let total: f32 = widths.iter().sum();
    if total < width {
        let extra = width - total;
        for w in widths.iter_mut() {
            *w += extra * *w / total;
        }
    }
    let xs: Vec<f32> = widths
        .iter()
        .scan(0.0, |acc, w| {
            let x = *acc;
            *acc += w;
            Some(x)
        })
        .collect();
    let left_align = [true, true, false, false, false, false];
    let content_h = HEAD_H + ROW_H * report.rows.len() as f32;
    ScrollArea::vertical()
        .id_salt("member-results")
        .max_height(TABLE_MAX_H)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            let (rect, _) = ui.allocate_exact_size(vec2(width, content_h), Sense::hover());
            let p = ui.painter();
            let cell_text = |p: &eframe::egui::Painter,
                             k: usize,
                             top: f32,
                             h: f32,
                             g: std::sync::Arc<eframe::egui::Galley>,
                             c: Color32| {
                let x0 = rect.left() + xs[k];
                let x = if left_align[k] {
                    x0 + pads[k].0
                } else {
                    x0 + widths[k] - pads[k].1 - g.size().x
                };
                p.galley(pos2(x, text_top(top + h / 2.0, &g)), g, c);
            };
            for (k, h) in heads.iter().enumerate() {
                let g = galley(ui, *h, font::semibold(12.0), a::MUTED);
                cell_text(p, k, rect.top(), HEAD_H, g, a::MUTED);
            }
            for (i, row) in report.rows.iter().enumerate() {
                let top = rect.top() + HEAD_H + i as f32 * ROW_H;
                p.hline(rect.x_range(), top + 0.5, Stroke::new(1.0, a::ROW_RULE));
                let colors = [
                    a::ID,
                    a::NODES,
                    n_color(row.tone),
                    t::TEXT,
                    a::MUTED,
                    use_color(row.use_tone),
                ];
                for (k, c) in cells(row).iter().enumerate() {
                    let g = galley(ui, c.as_str(), font::mono(12.0), colors[k]);
                    cell_text(p, k, top + 0.5, ROW_H - 0.5, g, colors[k]);
                }
                let label = format!(
                    "{} {}: N {} kN, σ {} MPa, limit {}, use {}",
                    row.id, row.nodes, row.force, row.stress, row.limit, row.utilisation
                );
                let r = Rect::from_min_size(pos2(rect.left(), top), vec2(width, ROW_H));
                ui.interact(
                    r,
                    ui.scope_id().with(("member-row", &row.id)),
                    Sense::hover(),
                )
                .widget_info(|| WidgetInfo::labeled(Role::Row, true, &label));
            }
        });
}

/// The Checks panel's content (`padding: 12px 14px; gap: 10px`).
pub fn checks_panel(ui: &mut Ui, report: &Report) {
    let width = ui.available_width();
    let inner = width - 2.0 * HEAD_PAD.x;
    ui.add_space(12.0);
    let hg = galley(ui, "Checks", h2_font(), t::TEXT);
    let (hr, _) = ui.allocate_exact_size(vec2(width, 19.0), Sense::hover());
    ui.painter().galley(
        pos2(hr.left() + HEAD_PAD.x, text_top(hr.center().y, &hg)),
        hg,
        t::TEXT,
    );
    ui.interact(hr, ui.scope_id().with("checks-h2"), Sense::hover())
        .widget_info(|| WidgetInfo::labeled(Role::Heading, true, "Checks"));
    for c in &report.checks {
        ui.add_space(10.0);
        // dot 8 + gap 10; Chrome's JetBrains Mono advance is a hair narrower than egui's, so
        // give the wrap the few px that keep its line breaks
        let text_w = inner - 8.0 - 10.0 + 4.0;
        let mut job = LayoutJob::default();
        let f = |font, color| TextFormat {
            font_id: font,
            color,
            line_height: Some(13.0 * 1.45),
            valign: Align::Center,
            ..Default::default()
        };
        job.append(&c.label, 0.0, f(font::semibold(13.0), t::TEXT));
        job.append(" ", 0.0, f(font::body(13.0), t::TEXT));
        job.append(&c.value, 0.0, f(font::mono(13.0), a::NODES));
        job.append("\n", 0.0, f(font::body(13.0), t::TEXT));
        let mut nf = f(font::body(12.0), t::TEXT_MUTED);
        nf.line_height = Some(13.0 * 1.45);
        job.append(&c.note, 0.0, nf);
        job.wrap = TextWrapping {
            max_width: text_w.max(20.0),
            ..Default::default()
        };
        let g = ui.painter().layout_job(job);
        // the first line mixes Inter and JetBrains Mono, which makes Chrome's line box ~1.3 px
        // taller than the 1.45 line height
        let (r, _) = ui.allocate_exact_size(vec2(width, g.size().y + 1.3), Sense::hover());
        let x = r.left() + HEAD_PAD.x;
        ui.painter().circle_filled(
            pos2(x + 4.0, r.top() + 6.0 + 4.0),
            4.0,
            level_color(c.level),
        );
        ui.painter()
            .galley(pos2(x + 8.0 + 10.0, r.top()), g, t::TEXT);
        let label = format!("{} {}. {}", c.label, c.value, c.note);
        ui.interact(r, ui.scope_id().with(("check", &c.label)), Sense::hover())
            .widget_info(|| WidgetInfo::labeled(Role::Label, true, &label));
    }
    ui.add_space(12.0);
}

/// The provenance badges (PROV-1), wrapping (`flex-wrap: wrap; gap: 6px`).
pub fn badges(ui: &mut Ui, k_factor: f64) {
    let k = sp_engine::netlist::text::tidy(k_factor);
    let list = [
        Badge::dot("direct stiffness · linear · small δ", t::PROV_EXACT),
        Badge::dot(format!("Euler buckling, K = {k}"), t::PROV_EXACT),
        Badge::dot("material values representative", t::PROV_SIMPLIFIED),
        Badge::dot("educational, not for design", a::EDUCATIONAL),
    ];
    let width = ui.available_width();
    let origin = ui.cursor().min;
    let (mut x, mut y, mut row_h) = (0.0f32, 0.0f32, 0.0f32);
    for b in &list {
        let s = badge_size(ui, b);
        if x > 0.0 && x + s.x > width {
            x = 0.0;
            y += row_h + 6.0;
            row_h = 0.0;
        }
        let r = Rect::from_min_size(origin + vec2(x, y), s);
        paint_badge(ui, r, b, BadgeLook::default());
        x += s.x + 6.0;
        row_h = row_h.max(s.y);
    }
    let used = Rect::from_min_size(origin, vec2(width, y + row_h));
    ui.advance_cursor_after_rect(used);
}
