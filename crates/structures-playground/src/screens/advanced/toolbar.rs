//! The toolbar row (ADV-1): `display: flex; flex-wrap: wrap; align-items: center; gap: 10px
//! 16px; padding: 8px 16px; border-bottom: 1px solid #242424; background: rgba(12,12,12,.85)`.
//! File name · Truss style · Cross-braced · Material · Size · Truck · x.

use super::controls::{inline_slider, select};
use super::doc::Doc;
use crate::widgets::text::{galley, text_top};
use crate::widgets::{Checkbox, SegGroup, SegItem, SegStyle};
use eframe::egui::{
    Align, Layout, Rect, Sense, Shape, Stroke, Ui, UiBuilder, WidgetInfo, accesskit::Role, pos2,
    vec2,
};
use lab_theme::font;
use lab_ui::layout::{FlexItem, flex_lines};
use sp_engine::generate::{MaterialKind, SIZE_RANGE, Style, TRUCK_RANGE};
use sp_engine::netlist::text::js_number;
use sp_theme as t;

/// What the user changed in the toolbar.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Action {
    /// Truss style.
    Style(Style),
    /// Cross-braced on / off.
    Braced(bool),
    /// Material.
    Material(MaterialKind),
    /// Size 1–20.
    Size(u32),
    /// Truck weight, t.
    Truck(f64),
    /// Truck position, m (pauses the drive).
    X(f64),
}

const GAP_X: f32 = 16.0;
const GAP_Y: f32 = 10.0;
const PAD: f32 = 16.0;

/// The select's options (`<option>` labels).
const MATERIALS: [&str; 4] = ["Steel", "Aluminum", "Glulam", "Carbon fiber"];

/// Draws the toolbar across the available width; returns the user's changes.
pub fn toolbar(ui: &mut Ui, doc: &Doc, select_open: &mut bool) -> Vec<Action> {
    let mut actions = Vec::new();
    let width = ui.available_width();
    let origin = ui.cursor().min;
    let inner_w = (width - 2.0 * PAD).max(0.0);
    let bg = ui.painter().add(Shape::Noop);

    // intrinsic sizes of the fixed items
    let file = doc.file_name();
    let file_g = galley(ui, &file, font::mono(13.0), t::TEXT_DIM);
    let styles: Vec<SegItem> = Style::ALL.iter().map(|s| SegItem::new(s.name())).collect();
    let seg = SegGroup::new("Truss style", &styles, None).style(SegStyle::Header);
    let seg_size = seg.one_row_size(ui);
    let cb_w = 18.0
        + 8.0
        + galley(ui, "Cross-braced", font::body(13.0), t::TEXT)
            .size()
            .x;
    let mat_label = galley(ui, "Material", font::body(13.0), t::TEXT);
    let widest = MATERIALS
        .iter()
        .map(|m| galley(ui, *m, font::body(13.0), t::TEXT).size().x)
        .fold(0.0, f32::max);
    let select_w = (widest + 36.0).round();
    let mat_w = mat_label.size().x + 8.0 + select_w;

    let items = [
        FlexItem::new(0.0, 1.0, file_g.size().x),
        FlexItem::new(0.0, 1.0, seg_size.x),
        FlexItem::new(0.0, 1.0, cb_w),
        FlexItem::new(0.0, 1.0, mat_w),
        FlexItem::new(1.0, 1.0, 200.0),
        FlexItem::new(1.0, 1.0, 200.0),
        FlexItem::new(1.0, 1.0, 220.0),
    ];
    let max = [
        None,
        None,
        None,
        None,
        Some(260.0),
        Some(260.0),
        Some(300.0),
    ];
    let heights = [20.0, seg_size.y, 40.0, 36.0, 28.0, 28.0, 28.0];
    let lines = flex_lines(inner_w, GAP_X, &items);

    let mut y = origin.y + 8.0;
    for (li, line) in lines.iter().enumerate() {
        if li > 0 {
            y += GAP_Y;
        }
        // max-width clamp, the excess to the line's other growing items
        let mut widths: Vec<f32> = line.iter().map(|p| p.width).collect();
        let mut excess = 0.0;
        let mut frozen = vec![false; line.len()];
        for (k, p) in line.iter().enumerate() {
            if let Some(m) = max[p.index]
                && widths[k] > m
            {
                excess += widths[k] - m;
                widths[k] = m;
                frozen[k] = true;
            }
        }
        let grow: f32 = line
            .iter()
            .enumerate()
            .filter(|(k, p)| !frozen[*k] && items[p.index].grow > 0.0)
            .map(|(_, p)| items[p.index].grow)
            .sum();
        if excess > 0.0 && grow > 0.0 {
            for (k, p) in line.iter().enumerate() {
                if !frozen[k] && items[p.index].grow > 0.0 {
                    widths[k] += excess * items[p.index].grow / grow;
                }
            }
        }
        let line_h = line.iter().map(|p| heights[p.index]).fold(0.0, f32::max);
        let cy = y + line_h / 2.0;
        let mut x = origin.x + PAD;
        for (k, p) in line.iter().enumerate() {
            let w = widths[k];
            let h = heights[p.index];
            let rect = Rect::from_min_size(pos2(x, cy - h / 2.0), vec2(w, h));
            x += w + GAP_X;
            let mut cell = ui.new_child(
                UiBuilder::new()
                    .id_salt(("toolbar", p.index))
                    .max_rect(rect)
                    .layout(Layout::left_to_right(Align::Center)),
            );
            match p.index {
                0 => {
                    let g = file_g.clone();
                    cell.painter()
                        .galley(pos2(rect.left(), text_top(cy, &g)), g, t::TEXT_DIM);
                    let s = file.clone();
                    cell.interact(rect, cell.scope_id().with("file"), Sense::hover())
                        .widget_info(|| WidgetInfo::labeled(Role::Label, true, &s));
                }
                1 => {
                    let sel = doc
                        .generated
                        .map(|g| Style::ALL.iter().position(|s| *s == g.style).unwrap_or(0));
                    let seg = SegGroup::new("Truss style", &styles, sel).style(SegStyle::Header);
                    if let Some(i) = seg.show(&mut cell).clicked {
                        actions.push(Action::Style(Style::ALL[i]));
                    }
                }
                2 => {
                    let on = doc.generated.map(|g| g.braced).unwrap_or(doc.params.braced);
                    if Checkbox::new("Cross-braced", on)
                        .gap(8.0)
                        .height(40.0)
                        .show(&mut cell)
                        .clicked()
                    {
                        actions.push(Action::Braced(!on));
                    }
                }
                3 => {
                    let g = mat_label.clone();
                    cell.painter()
                        .galley(pos2(rect.left(), text_top(cy, &g)), g, t::TEXT);
                    cell.add_space(mat_label.size().x + 8.0);
                    let cur = MaterialKind::ALL
                        .iter()
                        .position(|m| *m == doc.params.material)
                        .unwrap_or(0);
                    let r = select(
                        &mut cell,
                        "Material",
                        &MATERIALS,
                        cur,
                        select_w,
                        select_open,
                    );
                    if let Some(i) = r.picked
                        && i != cur
                    {
                        actions.push(Action::Material(MaterialKind::ALL[i]));
                    }
                }
                4 => {
                    let mut v = doc.params.size as f32;
                    let out = format!("{}", doc.params.size);
                    let range = SIZE_RANGE.0 as f32..=SIZE_RANGE.1 as f32;
                    if inline_slider(
                        &mut cell, rect, "Size", &mut v, range, 1.0, &out, 22.0, true,
                    )
                    .changed()
                    {
                        actions.push(Action::Size(v.round() as u32));
                    }
                }
                5 => {
                    let truck = doc.truck_t();
                    let mut v = truck.unwrap_or(doc.params.truck_t) as f32;
                    let out = format!("{} t", js_number(v as f64));
                    let range = TRUCK_RANGE.0 as f32..=TRUCK_RANGE.1 as f32;
                    let r = inline_slider(
                        &mut cell,
                        rect,
                        "Truck",
                        &mut v,
                        range,
                        1.0,
                        &out,
                        36.0,
                        truck.is_some(),
                    );
                    if r.changed() {
                        actions.push(Action::Truck(v.round() as f64));
                    }
                }
                _ => {
                    let (lo, hi) = doc.x_range();
                    let mut v = doc.x() as f32;
                    let out = format!("{} m", js_number(doc.x()));
                    let r = inline_slider(
                        &mut cell,
                        rect,
                        "x",
                        &mut v,
                        lo as f32..=hi as f32,
                        0.5,
                        &out,
                        52.0,
                        doc.truck_t().is_some(),
                    );
                    if r.changed() {
                        actions.push(Action::X(v as f64));
                    }
                }
            }
        }
        y += line_h;
    }
    let rect = Rect::from_min_max(origin, pos2(origin.x + width, y + 8.0 + 1.0));
    // the bar's fill goes under what was just drawn
    ui.painter().set(bg, Shape::rect_filled(rect, 0.0, t::BAR));
    ui.painter().hline(
        rect.x_range(),
        rect.bottom() - 0.5,
        Stroke::new(1.0, t::BORDER),
    );
    ui.advance_cursor_after_rect(rect);
    actions
}
