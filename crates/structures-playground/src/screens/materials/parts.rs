//! MT-1 — the left column (`<aside aria-label="Parts">`): Sample tiles with their 2.5D
//! swatches and the note, the Temperature slider with its note, the Strain axis segmented
//! control and the "Coming to this bench" chips.

use super::draw::tone_color;
use crate::screens::Badge;
use crate::widgets::badge::{badge_size, paint_badge};
use crate::widgets::flow::{FlowItem, flow};
use crate::widgets::text::{galley, lbl_row, text_top};
use crate::widgets::{BadgeLook, Slider, interact, lbl, set_bounds};
use eframe::egui::{
    CornerRadius, Rect, Sense, Shadow, Stroke, StrokeKind, Ui, UiBuilder, accesskit::Role, pos2,
    vec2,
};
use lab_ui::paint::focus_ring_if;
use sp_engine::materials::curves::{TEMP_MAX_C, TEMP_MIN_C, TEMP_STEP_C};
use sp_engine::materials::{Bench, Kind, View, Zoom};
use sp_theme as t;
use sp_theme::materials as mt;

/// "Coming to this bench" (static chips in the mockup).
pub const COMING: [&str; 4] = ["Fatigue", "Creep", "Phase diagrams", "Grain growth"];

/// Draws the column; changes go straight to `bench`.
pub fn show(ui: &mut Ui, bench: &mut Bench, v: &View) {
    section(ui, 8.0, |ui| {
        lbl(ui, "Sample");
        sample_tiles(ui, bench);
        text(ui, v.note, 12.0, Some(18.0));
    });
    section(ui, 2.0, |ui| {
        lbl_row(
            ui,
            "Temperature",
            &v.temp_label,
            lab_theme::font::mono(12.0),
            tone_color(v.temp_tone),
        );
        let mut temp = bench.temp() as f32;
        if Slider::new(
            "Temperature",
            &mut temp,
            TEMP_MIN_C as f32..=TEMP_MAX_C as f32,
        )
        .step(TEMP_STEP_C as f32)
        .show(ui)
        .changed()
        {
            bench.set_temp(temp as f64);
        }
        text(ui, &v.temp_note, 12.0, Some(18.0));
    });
    section(ui, 8.0, |ui| {
        lbl(ui, "Strain axis");
        if let Some(z) = zoom_seg(ui, bench.zoom()) {
            bench.set_zoom(z);
        }
    });
    section(ui, 6.0, |ui| {
        lbl(ui, "Coming to this bench");
        let badges: Vec<Badge> = COMING.iter().map(|c| Badge::plain(*c)).collect();
        let items: Vec<FlowItem> = badges
            .iter()
            .map(|b| FlowItem::fixed(badge_size(ui, b)))
            .collect();
        let width = ui.available_width();
        let f = flow(&items, width, vec2(6.0, 6.0), 0.0);
        let (rect, _) = ui.allocate_exact_size(vec2(width, f.height), Sense::hover());
        for (b, r) in badges.iter().zip(&f.rects) {
            paint_badge(ui, r.translate(rect.min.to_vec2()), b, BadgeLook::default());
        }
    });
}

/// A flex column with its own `gap`.
fn section(ui: &mut Ui, gap: f32, add: impl FnOnce(&mut Ui)) {
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.y = gap;
        add(ui);
    });
}

/// A muted paragraph (`.mut`, `font-size: N px; line-height: 1.5`).
fn text(ui: &mut Ui, s: &str, size: f32, line_h: Option<f32>) {
    crate::widgets::text::text_block(ui, s, lab_theme::font::body(size), t::TEXT_MUTED, line_h);
}

/// The five `.ptile` buttons in a 2-column grid (`gap: 8px`, min-height 84).
fn sample_tiles(ui: &mut Ui, bench: &mut Bench) {
    let width = ui.available_width();
    let gap = 8.0;
    let col_w = ((width - gap) / 2.0).max(0.0);
    let tile_h = t::size::PTILE;
    let rows = Kind::ALL.len().div_ceil(2);
    let height = rows as f32 * tile_h + (rows - 1) as f32 * gap;
    let (grid, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
    let font = lab_theme::font::semibold(12.0);
    let name_h = 15.0; // `font: 600 12px/1.25`
    for (i, kind) in Kind::ALL.into_iter().enumerate() {
        let (row, col) = (i / 2, i % 2);
        let rect = Rect::from_min_size(
            grid.min + vec2(col as f32 * (col_w + gap), row as f32 * (tile_h + gap)),
            vec2(col_w, tile_h),
        );
        let on = bench.kind() == kind;
        let id = ui.scope_id().with(("sample", kind.key()));
        let resp = interact(ui, id, rect, true, Role::Button, kind.name(), Some(on));
        if resp.clicked() {
            bench.pick(kind);
        }
        let (fill, border) = if on {
            (t::TILE_ON, t::ACCENT)
        } else if resp.hovered() {
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
        // Content (swatch 18 + gap 8 + name 15) centred in the padded box.
        let content_h = 18.0 + 8.0 + name_h;
        let top = rect.center().y - content_h / 2.0;
        let sw = Rect::from_center_size(pos2(rect.center().x, top + 9.0), vec2(52.0, 18.0));
        swatch(ui, sw, kind);
        let g = galley(ui, kind.name(), font.clone(), t::TEXT);
        let ny = top + 18.0 + 8.0 + name_h / 2.0;
        ui.painter().galley(
            pos2(rect.center().x - g.size().x / 2.0, text_top(ny, &g)),
            g,
            t::TEXT,
        );
        focus_ring_if(ui, &resp, rect, CornerRadius::same(t::radius::PANEL));
    }
}

/// The 2.5D swatch: `52 × 18`, radius 4, `box-shadow: inset 0 -3px 0 rgba(0,0,0,.25), 0 4px 0
/// <side>, 0 8px 10px rgba(0,0,0,.6)`.
pub fn swatch(ui: &Ui, rect: Rect, kind: Kind) {
    let s = mt::SAMPLES[kind.index()];
    let p = ui.painter();
    let r = CornerRadius::same(4);
    let shadow = Shadow {
        offset: [0, 8],
        blur: 10,
        spread: 0,
        color: mt::SWATCH_SHADOW,
    };
    p.add(shadow.as_shape(rect, r));
    p.rect_filled(rect.translate(vec2(0.0, 4.0)), r, s.side);
    p.rect_filled(rect, r, s.fill);
    let band = Rect::from_min_max(pos2(rect.left(), rect.bottom() - 3.0), rect.max);
    p.rect_filled(
        band,
        CornerRadius {
            nw: 0,
            ne: 0,
            sw: 4,
            se: 4,
        },
        mt::SWATCH_INSET,
    );
}

/// The Strain axis segmented control: `<div role="group" aria-label="Strain axis range">`, a
/// 3 px padded `rgba(6,6,6,.88)` well (border `#242424`, radius 8) with two `flex: 1` `.seg`
/// buttons (36 px, Inter 500 13, radius 6; on: `#2A2A2A` / `#F5F5F5`). Returns the clicked axis.
fn zoom_seg(ui: &mut Ui, current: Zoom) -> Option<Zoom> {
    let width = ui.available_width();
    let h = t::size::BUTTON + 2.0 * 3.0 + 2.0;
    let (rect, _) = ui.allocate_exact_size(vec2(width, h), Sense::hover());
    let group = ui.new_child(
        UiBuilder::new()
            .max_rect(rect)
            .accessibility_role(Role::Group)
            .accessibility_label("Strain axis range"),
    );
    set_bounds(&group, rect);
    group.painter().rect(
        rect,
        CornerRadius::same(t::radius::WELL),
        t::INSET_BAR,
        Stroke::new(1.0, t::BORDER),
        StrokeKind::Inside,
    );
    let inner = rect.shrink(4.0);
    let item_w = ((inner.width() - 2.0) / 2.0).max(0.0);
    let font = lab_theme::font::medium(13.0);
    let mut clicked = None;
    for (i, z) in Zoom::ALL.into_iter().enumerate() {
        let r = Rect::from_min_size(
            inner.min + vec2(i as f32 * (item_w + 2.0), 0.0),
            vec2(item_w, inner.height()),
        );
        let on = z == current;
        let resp = interact(
            &group,
            group.scope_id().with(("zoom", i)),
            r,
            true,
            Role::Button,
            z.label(),
            Some(on),
        );
        if resp.clicked() {
            clicked = Some(z);
        }
        let colour = if on {
            t::TEXT_STRONG
        } else if resp.hovered() {
            t::TEXT
        } else {
            t::TEXT_MUTED
        };
        let p = group.painter();
        if on {
            p.rect_filled(r, CornerRadius::same(t::radius::BUTTON), t::SEG_ON);
        }
        let g = galley(&group, z.label(), font.clone(), colour);
        p.galley(
            pos2(r.center().x - g.size().x / 2.0, text_top(r.center().y, &g)),
            g,
            colour,
        );
        focus_ring_if(&group, &resp, r, CornerRadius::same(t::radius::BUTTON));
    }
    clicked
}
