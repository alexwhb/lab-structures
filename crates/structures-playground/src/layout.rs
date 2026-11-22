//! Page layout: the mockup's body rows are `display: flex; flex-wrap: wrap` with columns
//! (`flex: <grow> <shrink> <basis>`, `min-width`, `max-width`, `box-sizing: border-box`). At
//! 1440 they sit side by side and fill the window under the chrome (header + world tab row);
//! at 390 they stack, each line as tall as its content, and the page scrolls.
//!
//! ```ignore
//! // the Tower / Ground / Materials row
//! const COLUMNS: [Column; 3] = [
//!     Column::parts("Parts"),
//!     Column::main("Stage"),
//!     Column::readouts("Readouts"),
//! ];
//! layout::columns(ui, ctx, &COLUMNS, |ui, ctx, i| match i {
//!     0 => parts(ui, ctx),
//!     1 => stage(ui, ctx),
//!     _ => readouts(ui, ctx),
//! });
//! ```

use crate::ctx::Ctx;
use eframe::egui::{
    Align, Color32, CornerRadius, Id, Layout, Rect, ScrollArea, Stroke, Ui, UiBuilder, Vec2,
    accesskit::Role, pos2, vec2,
};
use lab_ui::layout::{FlexItem, flex_lines};
use sp_theme as t;

/// The smallest height side-by-side columns get (shorter windows scroll the page).
pub const MIN_PANEL_H: f32 = 480.0;

/// One column of a page row.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Column {
    /// The region's accessible name (`<aside aria-label>`).
    pub label: &'static str,
    /// `flex-grow`.
    pub grow: f32,
    /// `flex-shrink`.
    pub shrink: f32,
    /// `flex-basis` (border box).
    pub basis: f32,
    /// `max-width`.
    pub max: Option<f32>,
    /// `min-width`.
    pub min: Option<f32>,
    /// Background (asides: `rgba(17,17,17,.84)`).
    pub fill: Option<Color32>,
    /// `border-left: 1px solid #242424`.
    pub border_left: bool,
    /// `border-right: 1px solid #242424`.
    pub border_right: bool,
    /// Vertical padding.
    pub pad_y: f32,
    /// Horizontal padding.
    pub pad_x: f32,
    /// `gap` between children (`flex-direction: column`).
    pub gap: f32,
    /// `overflow: auto` (the column scrolls on its own).
    pub scroll: bool,
}

impl Column {
    /// A side panel: `flex: 0 0 <basis>`, aside fill, `padding: 16px`, `gap: 14px`, scrolls.
    pub const fn aside(label: &'static str, basis: f32) -> Self {
        Self {
            label,
            grow: 0.0,
            shrink: 0.0,
            basis,
            max: None,
            min: None,
            fill: Some(t::SIDE),
            border_left: false,
            border_right: false,
            pad_y: 16.0,
            pad_x: 16.0,
            gap: 14.0,
            scroll: true,
        }
    }

    /// The Bridge's tools rail: `<nav aria-label="Tools" style="flex: 0 0 76px; border-right;
    /// padding: 12px 8px; gap: 6px">`.
    pub const fn rail(label: &'static str) -> Self {
        Self::aside(label, t::size::RAIL_W)
            .pad(12.0, 8.0)
            .gap(6.0)
            .border_right()
    }

    /// The left control column (Tower, Ground, Materials): `<aside aria-label="Parts"
    /// style="flex: 1 1 250px; max-width: 300px; min-width: 220px; border-right; padding: 14px
    /// 12px; gap: 16px">`.
    pub const fn parts(label: &'static str) -> Self {
        Self::aside(label, 250.0)
            .flex(1.0, 1.0, 250.0)
            .min(220.0)
            .max(300.0)
            .pad(14.0, 12.0)
            .gap(16.0)
            .border_right()
    }

    /// The right-hand aside (every Playground world): `flex: 1 1 300px; max-width: 360px;
    /// min-width: 260px; border-left; padding: 14px 12px; gap: 14px`.
    pub const fn readouts(label: &'static str) -> Self {
        Self::aside(label, 300.0)
            .flex(1.0, 1.0, 300.0)
            .min(t::size::ASIDE_MIN)
            .max(t::size::ASIDE_MAX)
            .pad(14.0, 12.0)
            .gap(14.0)
            .border_left()
    }

    /// The centre column: `flex: 999 1 560px; min-width: 0`, no fill, `padding: 14px 16px;
    /// gap: 10px` (the Bridge; Tower / Ground / Materials use `.flex(999.0, 1.0, 520.0)
    /// .pad(16.0, 16.0).gap(12.0)`).
    pub const fn main(label: &'static str) -> Self {
        Self {
            label,
            grow: 999.0,
            shrink: 1.0,
            basis: 560.0,
            max: None,
            min: None,
            fill: None,
            border_left: false,
            border_right: false,
            pad_y: 14.0,
            pad_x: 16.0,
            gap: 10.0,
            scroll: false,
        }
    }

    /// `flex: grow shrink basis`.
    pub const fn flex(mut self, grow: f32, shrink: f32, basis: f32) -> Self {
        self.grow = grow;
        self.shrink = shrink;
        self.basis = basis;
        self
    }
    /// `max-width`.
    pub const fn max(mut self, max: f32) -> Self {
        self.max = Some(max);
        self
    }
    /// `min-width`.
    pub const fn min(mut self, min: f32) -> Self {
        self.min = Some(min);
        self
    }
    /// `padding: <y> <x>`.
    pub const fn pad(mut self, y: f32, x: f32) -> Self {
        self.pad_y = y;
        self.pad_x = x;
        self
    }
    /// `gap`.
    pub const fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }
    /// `border-left`.
    pub const fn border_left(mut self) -> Self {
        self.border_left = true;
        self
    }
    /// `border-right`.
    pub const fn border_right(mut self) -> Self {
        self.border_right = true;
        self
    }
    /// `overflow: auto` on (the column scrolls by itself).
    pub const fn scroll(mut self, on: bool) -> Self {
        self.scroll = on;
        self
    }
}

/// A column placed on a line: index, x offset from the row's left, width.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placed {
    /// Index into the columns.
    pub index: usize,
    /// Offset from the row's left edge.
    pub x: f32,
    /// Resolved width.
    pub width: f32,
}

/// CSS flex-wrap line breaking and width resolution for `cols` in `width` px (no gap), with
/// `max-width` / `min-width` clamping (the excess or deficit goes to the line's other
/// columns).
pub fn resolve(width: f32, cols: &[Column]) -> Vec<Vec<Placed>> {
    // line breaking uses the hypothetical size: the basis clamped by min / max
    let items: Vec<FlexItem> = cols
        .iter()
        .map(|c| {
            let mut basis = c.basis;
            if let Some(m) = c.max {
                basis = basis.min(m);
            }
            if let Some(m) = c.min {
                basis = basis.max(m);
            }
            FlexItem::new(c.grow, c.shrink, basis)
        })
        .collect();
    flex_lines(width, 0.0, &items)
        .into_iter()
        .map(|line| {
            let mut widths: Vec<f32> = line.iter().map(|p| p.width).collect();
            let mut excess = 0.0;
            let mut frozen = vec![false; line.len()];
            for (k, p) in line.iter().enumerate() {
                if let Some(m) = cols[p.index].max
                    && widths[k] > m
                {
                    excess += widths[k] - m;
                    widths[k] = m;
                    frozen[k] = true;
                }
                // a stacked column wider than the window keeps its min-width (CSS overflows)
                if let Some(m) = cols[p.index].min
                    && widths[k] < m
                {
                    excess -= m - widths[k];
                    widths[k] = m;
                    frozen[k] = true;
                }
            }
            let grow_sum: f32 = line
                .iter()
                .enumerate()
                .filter(|(k, _)| !frozen[*k])
                .map(|(_, p)| cols[p.index].grow)
                .sum();
            if excess < 0.0 {
                // taken from the line's other shrinkable columns, by their basis
                let shrink_sum: f32 = line
                    .iter()
                    .enumerate()
                    .filter(|(k, _)| !frozen[*k])
                    .map(|(_, p)| cols[p.index].shrink * cols[p.index].basis)
                    .sum();
                if shrink_sum > 0.0 {
                    for (k, p) in line.iter().enumerate() {
                        if !frozen[k] {
                            let c = &cols[p.index];
                            widths[k] =
                                (widths[k] + excess * c.shrink * c.basis / shrink_sum).max(0.0);
                        }
                    }
                }
            } else if excess > 0.0 && grow_sum > 0.0 {
                for (k, p) in line.iter().enumerate() {
                    if !frozen[k] {
                        widths[k] += excess * cols[p.index].grow / grow_sum;
                    }
                }
            }
            let mut x = 0.0;
            line.iter()
                .zip(widths)
                .map(|(p, w)| {
                    let placed = Placed {
                        index: p.index,
                        x,
                        width: w,
                    };
                    x += w;
                    placed
                })
                .collect()
        })
        .collect()
}

/// Lays out a page row of `cols` at the cursor across the available width and calls
/// `add(ui, ctx, i)` for each column inside its padding (a top-down `Ui` with `gap` spacing).
///
/// Side by side, columns fill [`Ctx::body_height`] and a `scroll` column scrolls on its own
/// (`overflow: auto`). Stacked (narrow windows), each line is as tall as its tallest
/// column's content — what the column *allocated* (measured the pass before) — and nothing
/// scrolls but the page; if the lines together are shorter than the route's
/// [`panel_height`](crate::Route::panel_height) the rest is shared out equally (CSS
/// `align-content: stretch`). A column that only paints into `ui.available_rect_before_wrap()`
/// must allocate what it drew (e.g. `ui.advance_cursor_after_rect(rect)`) to keep its height. One
/// scroller means `scroll_to_rect`, keyboard focus and AccessKit `ScrollIntoView` always
/// reach the page (an inner egui `ScrollArea` would swallow them). Returns the rect used.
pub fn columns(
    ui: &mut Ui,
    ctx: &mut Ctx,
    cols: &[Column],
    mut add: impl FnMut(&mut Ui, &mut Ctx, usize),
) -> Rect {
    let origin = ui.cursor().min;
    let width = ui.available_width();
    let lines = resolve(width, cols);
    let stacked = lines.len() > 1;
    let panel_h = if stacked {
        ctx.route().panel_height()
    } else {
        ctx.body_height()
    }
    .max(MIN_PANEL_H);
    let route_id = ctx.route().id();
    let content_id = |i: usize| Id::unique(("column-content", route_id, i));
    // Stacked (CSS `flex-wrap` in a `min-height: <board>` page): each line is as tall as its
    // tallest column's content (measured the pass before; the panel height until then), and
    // only when all lines together are shorter than the board's body does `align-content:
    // stretch` share the rest out equally.
    let heights: Vec<f32> = if stacked {
        let mut h: Vec<f32> = lines
            .iter()
            .map(|line| {
                let measured: Vec<f32> = line
                    .iter()
                    .filter_map(|p| ui.data(|d| d.get_temp::<f32>(content_id(p.index))))
                    .collect();
                if measured.len() < line.len() {
                    panel_h
                } else {
                    measured.into_iter().fold(0.0, f32::max)
                }
            })
            .collect();
        let total: f32 = h.iter().sum();
        if total < panel_h {
            let extra = (panel_h - total) / h.len() as f32;
            h.iter_mut().for_each(|x| *x += extra);
        }
        h
    } else {
        vec![panel_h; lines.len()]
    };
    let mut y = origin.y;
    for (line, &height) in lines.iter().zip(&heights) {
        for p in line {
            let c = &cols[p.index];
            let rect = Rect::from_min_size(pos2(origin.x + p.x, y), vec2(p.width, height));
            let mut col = ui.new_child(
                UiBuilder::new()
                    .id_salt(("column", p.index))
                    .max_rect(rect)
                    .layout(Layout::top_down(Align::Min))
                    .accessibility_role(Role::Region)
                    .accessibility_label(c.label),
            );
            col.set_clip_rect(rect.intersect(ui.clip_rect()));
            crate::widgets::set_bounds(&col, rect);
            paint_column(&col, rect, c);
            let inner = Rect::from_min_max(
                pos2(
                    rect.left() + c.pad_x + c.border_left as u8 as f32,
                    rect.top() + c.pad_y,
                ),
                pos2(
                    rect.right() - c.pad_x - c.border_right as u8 as f32,
                    rect.bottom() - c.pad_y,
                ),
            );
            if c.scroll && !stacked {
                let mut area_ui = col.new_child(
                    UiBuilder::new()
                        .max_rect(Rect::from_min_max(
                            pos2(rect.left() + c.border_left as u8 as f32, rect.top()),
                            pos2(rect.right() - c.border_right as u8 as f32, rect.bottom()),
                        ))
                        .layout(Layout::top_down(Align::Min)),
                );
                ScrollArea::vertical()
                    .id_salt(("column-scroll", ctx.route().id(), p.index))
                    .auto_shrink([false, false])
                    .show(&mut area_ui, |ui| {
                        eframe::egui::Frame::new()
                            .inner_margin(eframe::egui::Margin::symmetric(
                                c.pad_x.round() as i8,
                                c.pad_y.round() as i8,
                            ))
                            .show(ui, |ui| {
                                ui.set_width(inner.width());
                                ui.spacing_mut().item_spacing = vec2(c.gap, c.gap);
                                add(ui, ctx, p.index);
                            });
                    });
            } else {
                let mut inner_ui = col.new_child(
                    UiBuilder::new()
                        .max_rect(inner)
                        .layout(Layout::top_down(Align::Min)),
                );
                inner_ui.spacing_mut().item_spacing = vec2(c.gap, c.gap);
                add(&mut inner_ui, ctx, p.index);
                if stacked {
                    // remember the content's height (what it allocated) for its line
                    let need = inner_ui.min_rect().height() + 2.0 * c.pad_y;
                    let id = content_id(p.index);
                    let old = ui.data(|d| d.get_temp::<f32>(id));
                    if old.is_none_or(|o| (o - need).abs() > 0.5) {
                        ui.data_mut(|d| d.insert_temp(id, need));
                        ui.ctx()
                            .request_discard("stacked column content height changed");
                    }
                }
            }
        }
        y += height;
    }
    let used = Rect::from_min_max(origin, pos2(origin.x + width, y));
    lab_ui::paint::advance_block(ui, used);
    used
}

fn paint_column(ui: &Ui, rect: Rect, c: &Column) {
    let painter = ui.painter();
    if let Some(fill) = c.fill {
        painter.rect_filled(rect, CornerRadius::ZERO, fill);
    }
    let stroke = Stroke::new(1.0, t::BORDER);
    if c.border_left {
        painter.vline(rect.left() + 0.5, rect.y_range(), stroke);
    }
    if c.border_right {
        painter.vline(rect.right() - 0.5, rect.y_range(), stroke);
    }
}

/// Passes a nested scroll area's scrolling on to the page. egui hands a scroll request
/// (`scroll_to_me`, keyboard focus, AccessKit `ScrollIntoView`) to the innermost `ScrollArea`
/// only; Chrome's `scrollIntoView` scrolls every ancestor. Call it after showing a scroll area
/// that sits inside the page (a scrolling stage or list) with its id, offset and viewport:
/// when the area scrolled this frame without the user's wheel or drag, the page brings its
/// viewport into view too.
pub fn bubble_scroll(ui: &Ui, id: Id, offset: Vec2, viewport: Rect) {
    let key = id.with("bubble-scroll");
    let before = ui.data(|d| d.get_temp::<Vec2>(key));
    ui.data_mut(|d| d.insert_temp(key, offset));
    let by_user =
        ui.input(|i| i.smooth_scroll_delta != Vec2::ZERO || i.pointer.is_decidedly_dragging());
    if before.is_some_and(|b| (b - offset).length() > 0.5) && !by_user {
        ui.scroll_to_rect(viewport, None);
    }
}

/// Paints the `.graph` ground (`#0B0C0E`, 120 / 24 px grid anchored at its corner) over the
/// rest of `ui`'s rect and returns that rect.
pub fn stage(ui: &mut Ui) -> Rect {
    let rect = ui.available_rect_before_wrap();
    lab_ui::paint::paint_grid(ui.painter(), rect, rect.min, &t::GRAPH);
    rect
}

/// A stage panel: `rect` filled with `.graph` (or `fill` when given, e.g. the Bridge's
/// `#0C0D0F`), clipped to radius 12 with the `1px solid #242424` border (`border-radius: 12px;
/// overflow: hidden`). Returns a child `Ui` clipped to the panel to draw the scene in.
pub fn graph_panel(ui: &mut Ui, rect: Rect, fill: Option<Color32>) -> Ui {
    let painter = ui.painter();
    let r = CornerRadius::same(t::radius::PANEL);
    match fill {
        Some(c) => {
            painter.rect_filled(rect, r, c);
        }
        None => {
            painter.rect_filled(rect, r, t::GRAPH.base);
            lab_ui::paint::paint_grid(
                &painter.with_clip_rect(rect.shrink(1.0).intersect(ui.clip_rect())),
                rect,
                rect.min,
                &t::GRAPH,
            );
        }
    }
    let mut child = ui.new_child(
        UiBuilder::new()
            .id_salt(("graph-panel", rect.min.x as i32, rect.min.y as i32))
            .max_rect(rect)
            .layout(Layout::top_down(Align::Min)),
    );
    child.set_clip_rect(rect.intersect(ui.clip_rect()));
    child
}

/// Paints the border of a [`graph_panel`] (call after drawing the scene, so it sits on top).
pub fn graph_panel_border(ui: &Ui, rect: Rect) {
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(t::radius::PANEL),
        Stroke::new(1.0, t::BORDER),
        eframe::egui::StrokeKind::Inside,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The Bridge row: tools rail · main · aside.
    const BRIDGE: [Column; 3] = [
        Column::rail("Tools"),
        Column::main("Stage"),
        Column::readouts("Level and inspector"),
    ];
    /// The Tower / Ground / Materials row.
    const WORLD: [Column; 3] = [
        Column::parts("Parts"),
        Column::main("Stage").flex(999.0, 1.0, 520.0),
        Column::readouts("Readouts"),
    ];

    fn widths(lines: &[Vec<Placed>]) -> Vec<Vec<f32>> {
        lines
            .iter()
            .map(|l| l.iter().map(|p| p.width).collect())
            .collect()
    }

    /// 1440: 76 · 1063.7 · 300.3 (free 504 shared 999 : 1 between main and the aside).
    #[test]
    fn bridge_at_1440() {
        let lines = resolve(1440.0, &BRIDGE);
        assert_eq!(lines.len(), 1);
        let w = &widths(&lines)[0];
        assert_eq!(w[0], 76.0);
        assert!((w[2] - (300.0 + 504.0 / 1000.0)).abs() < 1e-3, "{w:?}");
        assert!((w.iter().sum::<f32>() - 1440.0).abs() < 1e-3);
    }

    /// 390: the rail, main and aside each on their own line; the aside capped at 360.
    #[test]
    fn bridge_stacks_at_390() {
        let lines = resolve(390.0, &BRIDGE);
        assert_eq!(widths(&lines), vec![vec![76.0], vec![390.0], vec![360.0]]);
    }

    /// Tower at 1440: 250.4 · 889.3 · 300.4; at 390 the parts stop at 300, the aside at 360.
    #[test]
    fn world_rows() {
        let lines = resolve(1440.0, &WORLD);
        assert_eq!(lines.len(), 1);
        let w = &widths(&lines)[0];
        assert!(
            (w[0] - 250.37).abs() < 0.01 && (w[2] - 300.37).abs() < 0.01,
            "{w:?}"
        );
        let lines = resolve(390.0, &WORLD);
        assert_eq!(widths(&lines), vec![vec![300.0], vec![390.0], vec![360.0]]);
    }

    /// min-width holds: a 240 px window keeps the aside at 260 (CSS overflows).
    #[test]
    fn min_width_holds() {
        let lines = resolve(240.0, &BRIDGE);
        assert_eq!(lines.last().unwrap()[0].width, 260.0);
    }

    /// A medium window: the aside wraps onto its own line and stops at 360.
    #[test]
    fn aside_wraps_and_caps() {
        let lines = resolve(800.0, &BRIDGE);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[1][0].width, 360.0);
    }
}
