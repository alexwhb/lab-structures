//! The Bridge's own small controls, drawn to `Playground.dc.html`'s CSS: `.chip` buttons (Size,
//! Weight, toggles, materials, supports), `.pill` (status), `.lvl` level rows with the star
//! row, `.tool` rail buttons, stat tiles, and the `.bar` utilisation bar. Every control is an
//! AccessKit node with the mockup's name and pressed state.

use crate::widgets::text::{galley, text_top};
use crate::widgets::{Icon, hit, interact};
use eframe::egui::{
    Color32, CornerRadius, Id, Painter, Pos2, Rect, Response, Sense, Shape, Stroke, StrokeKind, Ui,
    Vec2, WidgetInfo, accesskit::Role, pos2, vec2,
};
use lab_ui::paint::focus_ring_if;
use sp_theme as t;

/// `.chip` height (`min-height: 34px`).
pub const CHIP_H: f32 = t::size::CHIP;
/// Gap between chips in a `.chips` group.
pub const CHIP_GAP: f32 = 4.0;

/// A `.chip` button's look.
#[derive(Debug, Clone, Copy, Default)]
pub struct ChipLook {
    /// `.chip-on`.
    pub on: bool,
    /// `.chip-dim` (opacity .45; still clickable, like the mockup).
    pub dim: bool,
    /// A leading 8 px `.dot` (material swatch).
    pub dot: Option<Color32>,
}

fn chip_font() -> eframe::egui::FontId {
    lab_theme::font::medium(12.0)
}

/// The drawn size of a chip labelled `label`: `padding: 0 12px`, 1 px border, `gap: 6px` after
/// a dot.
pub fn chip_size(ui: &Ui, label: &str, dot: bool) -> Vec2 {
    let w = galley(ui, label, chip_font(), Color32::WHITE).size().x;
    vec2(2.0 + 24.0 + w + if dot { 14.0 } else { 0.0 }, CHIP_H)
}

/// Draws a `.chip` button into `rect` with hit area `target`. AccessKit: a `Button` named
/// `name` with `pressed` reported.
pub fn chip_at(
    ui: &mut Ui,
    id: Id,
    rect: Rect,
    target: Rect,
    label: &str,
    name: &str,
    look: ChipLook,
) -> Response {
    let resp = interact(ui, id, target, true, Role::Button, name, Some(look.on));
    if ui.is_rect_visible(rect) {
        let hovered = resp.hovered();
        let (fill, border, mut fg) = if look.on {
            (t::TINT, t::EDGE_ON, t::INK)
        } else if hovered {
            (t::CONTROL, t::bridge::CHIP_HOVER_BORDER, t::TEXT_STRONG)
        } else {
            (t::CONTROL, t::BORDER_STRONG, t::TEXT_CHIP)
        };
        let k = if look.dim { 0.45 } else { 1.0 };
        let (fill, border) = (fill.gamma_multiply(k), border.gamma_multiply(k));
        fg = fg.gamma_multiply(k);
        let painter = ui.painter();
        painter.rect(
            rect,
            CornerRadius::same(t::radius::PILL),
            fill,
            Stroke::new(1.0, border),
            StrokeKind::Inside,
        );
        let mut x = rect.left() + 13.0;
        let cy = rect.center().y;
        if let Some(d) = look.dot {
            painter.circle_filled(pos2(x + 4.0, cy), 4.0, d.gamma_multiply(k));
            x += 14.0;
        }
        let g = galley(ui, label, chip_font(), fg);
        painter.galley(pos2(x, text_top(cy, &g)), g, fg);
        focus_ring_if(ui, &resp, rect, CornerRadius::same(t::radius::PILL));
    }
    resp
}

/// One chip in a group: its label, accessible name and look.
pub struct ChipSpec<'a> {
    /// Visible text.
    pub label: &'a str,
    /// Accessible name (usually the label).
    pub name: &'a str,
    /// Look.
    pub look: ChipLook,
}

/// Lays out a `.chips` group (gap 4, wrapping at `width`) at `origin`; returns the rects and
/// the group's size.
pub fn chips_layout(ui: &Ui, specs: &[ChipSpec<'_>], width: f32, gap: Vec2) -> (Vec<Rect>, Vec2) {
    let mut rects = Vec::with_capacity(specs.len());
    let (mut x, mut y, mut w_max) = (0.0f32, 0.0f32, 0.0f32);
    for s in specs {
        let size = chip_size(ui, s.label, s.look.dot.is_some());
        if x > 0.0 && x + size.x > width {
            x = 0.0;
            y += CHIP_H + gap.y;
        }
        rects.push(Rect::from_min_size(pos2(x, y), size));
        x += size.x + gap.x;
        w_max = w_max.max(x - gap.x);
    }
    let h = if specs.is_empty() { 0.0 } else { y + CHIP_H };
    (rects, vec2(w_max, h))
}

/// Draws a chip group at `origin` with rects from [`chips_layout`]; returns the index clicked.
pub fn chips_show(
    ui: &mut Ui,
    salt: &str,
    origin: Pos2,
    specs: &[ChipSpec<'_>],
    rects: &[Rect],
) -> Option<usize> {
    let placed: Vec<Rect> = rects
        .iter()
        .map(|r| r.translate(origin.to_vec2()))
        .collect();
    let targets = hit::tile(&placed);
    let mut clicked = None;
    for (i, s) in specs.iter().enumerate() {
        let id = Id::unique(("bridge-chip", salt, i));
        if chip_at(ui, id, placed[i], targets[i], s.label, s.name, s.look).clicked() {
            clicked = Some(i);
        }
    }
    clicked
}

/// Paints the star row (`★★☆`) as five-point stars in the accent, `size` px tall, from `left`
/// at centre height `cy`: filled for earned, outlined for not yet (MB-2: drawn at the row's
/// mono 12 px, not tiny).
pub fn paint_stars(painter: &Painter, left: f32, cy: f32, earned: u8, size: f32) -> f32 {
    let r = size * 0.5;
    let step = size * 0.62;
    for k in 0..3u8 {
        let c = pos2(left + r + k as f32 * step, cy);
        let pts: Vec<Pos2> = (0..10)
            .map(|i| {
                let a = -std::f32::consts::FRAC_PI_2 + i as f32 * std::f32::consts::PI / 5.0;
                let rr = if i % 2 == 0 { r } else { r * 0.45 };
                c + vec2(a.cos() * rr, a.sin() * rr)
            })
            .collect();
        if k < earned {
            // fill as a fan of triangles from the centre (the outline isn't convex)
            for i in 0..10 {
                painter.add(Shape::convex_polygon(
                    vec![c, pts[i], pts[(i + 1) % 10]],
                    t::ACCENT,
                    Stroke::NONE,
                ));
            }
        } else {
            painter.add(Shape::closed_line(pts, Stroke::new(0.9, t::ACCENT)));
        }
    }
    2.0 * r + 2.0 * step
}

/// Width of a star row of `size`.
pub fn stars_width(size: f32) -> f32 {
    size + 2.0 * size * 0.62
}

/// `.lvl` level row (`min-height: 40px; padding: 0 12px`, radius 8): the name left, the best
/// stars right. AccessKit: a `Button` named by the level with its pressed state.
pub fn level_row(
    ui: &mut Ui,
    id: Id,
    rect: Rect,
    name: &str,
    stars: Option<u8>,
    on: bool,
) -> Response {
    let resp = interact(ui, id, rect, true, Role::Button, name, Some(on));
    let hovered = resp.hovered();
    let (fill, border, fg) = if on {
        (t::TINT, t::EDGE_ON, t::INK)
    } else if hovered {
        (t::ROW, t::bridge::CHIP_HOVER_BORDER, t::TEXT_STRONG)
    } else {
        (t::ROW, t::BORDER, t::TEXT_CHIP)
    };
    let painter = ui.painter();
    painter.rect(
        rect,
        CornerRadius::same(t::radius::ROW),
        fill,
        Stroke::new(1.0, border),
        StrokeKind::Inside,
    );
    let g = galley(ui, name, lab_theme::font::medium(13.0), fg);
    let cy = rect.center().y;
    painter.galley(pos2(rect.left() + 13.0, text_top(cy, &g)), g, fg);
    if let Some(n) = stars {
        let w = stars_width(9.0);
        paint_stars(painter, rect.right() - 13.0 - w, cy, n, 9.0);
    }
    focus_ring_if(ui, &resp, rect, CornerRadius::same(t::radius::ROW));
    resp
}

/// `.tool` rail button: 60 × 56, radius 10, the 22 px icon over the 11 px label (gap 4).
/// AccessKit: a `Button` named by the label; tools report pressed, Undo / Clear don't.
pub fn tool_button(
    ui: &mut Ui,
    id: Id,
    rect: Rect,
    icon: Icon,
    label: &str,
    tip: &str,
    pressed: Option<bool>,
) -> Response {
    let resp = interact(ui, id, rect, true, Role::Button, label, pressed).on_hover_text(tip);
    let on = pressed == Some(true);
    let hovered = resp.hovered();
    let (fill, border, fg) = if on {
        (t::TINT, t::EDGE, t::INK)
    } else if hovered {
        (t::SW_HOVER, Color32::TRANSPARENT, t::TEXT)
    } else {
        (Color32::TRANSPARENT, Color32::TRANSPARENT, t::TEXT_MUTED)
    };
    let painter = ui.painter();
    painter.rect(
        rect,
        CornerRadius::same(t::radius::TOOL),
        fill,
        Stroke::new(1.0, border),
        StrokeKind::Inside,
    );
    let g = galley(ui, label, lab_theme::font::medium(11.0), fg);
    let content = 22.0 + 4.0 + g.size().y;
    let top = rect.center().y - content / 2.0;
    icon.paint(
        painter,
        Rect::from_min_size(pos2(rect.center().x - 11.0, top), vec2(22.0, 22.0)),
        fg,
    );
    painter.galley(pos2(rect.center().x - g.size().x / 2.0, top + 26.0), g, fg);
    focus_ring_if(ui, &resp, rect, CornerRadius::same(t::radius::TOOL));
    resp
}

/// `.pill` (status): `padding: 4px 10px`, radius 999, `#141414` with a `#2E2E2E` border, Inter
/// 600 12 px in `color` after an 8 px dot. Returns its size; AccessKit: a `Label`.
pub fn pill(ui: &mut Ui, origin: Pos2, label: &str, color: Color32) -> Rect {
    let g = galley(ui, label, lab_theme::font::semibold(12.0), color);
    let h = 2.0 + 8.0 + crate::widgets::text::css_line_height(ui, &lab_theme::font::semibold(12.0));
    let rect = Rect::from_min_size(origin, vec2(2.0 + 20.0 + 8.0 + 6.0 + g.size().x, h));
    let painter = ui.painter();
    painter.rect(
        rect,
        CornerRadius::same(t::radius::PILL),
        t::CONTROL,
        Stroke::new(1.0, t::BORDER_STRONG),
        StrokeKind::Inside,
    );
    let cy = rect.center().y;
    painter.circle_filled(pos2(rect.left() + 11.0 + 4.0, cy), 4.0, color);
    painter.galley(pos2(rect.left() + 11.0 + 14.0, text_top(cy, &g)), g, color);
    label_node(ui, ("bridge-pill", label), rect, label);
    rect
}

/// Registers an AccessKit `Label` over `rect`.
pub fn label_node(
    ui: &Ui,
    salt: impl std::hash::Hash + std::fmt::Debug,
    rect: Rect,
    text: &str,
) -> Response {
    let r = ui.interact(rect, Id::unique(salt), Sense::hover());
    let s = text.to_string();
    r.widget_info(|| WidgetInfo::labeled(Role::Label, true, &s));
    r
}

/// The baseline of a one-line galley, from its top.
fn baseline(g: &eframe::egui::Galley) -> f32 {
    g.rows
        .first()
        .map(|r| r.pos.y + r.row.glyphs.first().map_or(0.0, |gl| gl.pos.y))
        .unwrap_or(0.0)
}

/// A Bridge stat tile (`.well`, `padding: 8px 10px`): the `.lbl` caption over a mono 16 px
/// value. The caption is an inline span in a 16 px line box (the page's default font size),
/// so its line is as tall as 16 px Inter's and the caption sits on that line's baseline.
/// AccessKit: a `Label` "<caption>: <value>".
pub fn tile(ui: &mut Ui, rect: Rect, caption: &str, value: &str) {
    let painter = ui.painter();
    painter.rect(
        rect,
        CornerRadius::same(t::radius::WELL),
        t::WELL,
        Stroke::new(1.0, t::BORDER),
        StrokeKind::Inside,
    );
    let strut_font = lab_theme::font::body(16.0);
    let strut = galley(ui, "Jg", strut_font.clone(), t::TEXT);
    let strut_h = crate::widgets::text::css_line_height(ui, &strut_font);
    let lg = crate::widgets::text::lbl_galley(ui, caption, t::TEXT_MUTED);
    let vg = galley(ui, value, lab_theme::font::mono(16.0), t::TEXT);
    let x = rect.left() + 11.0;
    let line_top = rect.top() + 9.0;
    // the strut's baseline in its CSS line box (half-leading + ascent)
    let base = line_top + (strut_h - strut.size().y) / 2.0 + baseline(&strut);
    painter.galley(pos2(x, base - baseline(&lg)), lg.clone(), t::TEXT_MUTED);
    let value_top = line_top + strut_h;
    let vh = crate::widgets::text::css_line_height(ui, &lab_theme::font::mono(16.0));
    painter.galley(pos2(x, value_top + (vh - vg.size().y) / 2.0), vg, t::TEXT);
    label_node(
        ui,
        ("bridge-tile", caption),
        rect,
        &format!("{caption}: {value}"),
    );
}

/// Height of a [`tile`]: border + padding + the 16 px strut line + the mono 16 px line.
pub fn tile_height(ui: &Ui) -> f32 {
    2.0 + 16.0
        + crate::widgets::text::css_line_height(ui, &lab_theme::font::body(16.0))
        + crate::widgets::text::css_line_height(ui, &lab_theme::font::mono(16.0))
}

/// `.bar` (8 px, `#1E1E1E`, radius 4) filled to `frac` in `color`.
pub fn bar(painter: &Painter, rect: Rect, frac: f32, color: Color32) {
    painter.rect_filled(rect, CornerRadius::same(4), t::TRACK);
    let f = frac.clamp(0.0, 1.0);
    if f > 0.0 {
        let fill = Rect::from_min_size(rect.min, vec2(rect.width() * f, rect.height()));
        painter
            .with_clip_rect(rect.intersect(painter.clip_rect()))
            .rect_filled(fill, CornerRadius::same(4), color);
    }
}
