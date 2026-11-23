//! The builder's own small controls, drawn to `Builder.dc.html`'s CSS: `.chip` buttons (piece
//! palette with swatch bars, foundations, crates, conditions with soil dots, inspector chips,
//! "Show forces"), `.pill` (status, info, HUD), `.lvl` level rows with their star row, `.tool`
//! rail buttons, the `.bar` utilisation bar, and the aside's section / frame / paragraph blocks.
//! Every control is an AccessKit node with the mockup's name and pressed state. (Forked from
//! the retired v1 Bridge's `bits.rs`.)

use crate::widgets::text::{css_line_height, galley, lbl_galley, text_top, wrapped};
use crate::widgets::{Icon, hit, interact, set_bounds};
use eframe::egui::{
    Align, Color32, CornerRadius, Id, Layout, Painter, Pos2, Rect, Response, Sense, Shape, Stroke,
    StrokeKind, Ui, UiBuilder, Vec2, WidgetInfo, accesskit::Role, epaint::RectShape, pos2, vec2,
};
use lab_ui::paint::focus_ring_if;
use sp_theme::{self as t, builder as b};

/// `.chip` height (`min-height: 34px`).
pub const CHIP_H: f32 = t::size::CHIP;
/// Gap between chips in a `.chips` group (`gap: 4px`).
pub const CHIP_GAP: f32 = 4.0;

/// A `.chip` button's look.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ChipLook {
    /// `.chip-on`.
    pub on: bool,
    /// `.chip-dim` (opacity .45; still clickable, like the mockup).
    pub dim: bool,
    /// A leading 8 px `.dot` (soil swatch).
    pub dot: Option<Color32>,
    /// A leading 16 px swatch bar of this height (the piece palette, radius 2).
    pub bar: Option<(Color32, f32)>,
}

fn chip_font() -> eframe::egui::FontId {
    lab_theme::font::medium(12.0)
}

/// The drawn size of a chip: `padding: 0 12px`, 1 px border, a dot (8) or bar (16) then
/// `gap: 6px`.
pub fn chip_size(ui: &Ui, label: &str, look: &ChipLook) -> Vec2 {
    let w = galley(ui, label, chip_font(), Color32::WHITE).size().x;
    let lead = if look.bar.is_some() {
        22.0
    } else if look.dot.is_some() {
        14.0
    } else {
        0.0
    };
    vec2(2.0 + 24.0 + w + lead, CHIP_H)
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
        let (fill, border, fg) = if look.on {
            (t::TINT, t::EDGE_ON, t::INK)
        } else if hovered {
            (t::CONTROL, b::CHIP_HOVER_BORDER, t::TEXT_STRONG)
        } else {
            (t::CONTROL, t::BORDER_STRONG, t::TEXT_CHIP)
        };
        let k = if look.dim { 0.45 } else { 1.0 };
        let painter = ui.painter();
        painter.rect(
            rect,
            CornerRadius::same(t::radius::PILL),
            fill.gamma_multiply(k),
            Stroke::new(1.0, border.gamma_multiply(k)),
            StrokeKind::Inside,
        );
        let mut x = rect.left() + 13.0;
        let cy = rect.center().y;
        if let Some((c, h)) = look.bar {
            painter.rect_filled(
                Rect::from_min_size(pos2(x, cy - h / 2.0), vec2(16.0, h)),
                CornerRadius::same(2),
                c.gamma_multiply(k),
            );
            x += 22.0;
        } else if let Some(d) = look.dot {
            painter.circle_filled(pos2(x + 4.0, cy), 4.0, d.gamma_multiply(k));
            x += 14.0;
        }
        let g = galley(ui, label, chip_font(), fg.gamma_multiply(k));
        painter.galley(pos2(x, text_top(cy, &g)), g, fg);
        focus_ring_if(ui, &resp, rect, CornerRadius::same(t::radius::PILL));
    }
    resp
}

/// One chip in a group: its label, accessible name and look.
#[derive(Debug, Clone)]
pub struct ChipSpec {
    /// Visible text.
    pub label: String,
    /// Accessible name (usually the label).
    pub name: String,
    /// Look.
    pub look: ChipLook,
}

impl ChipSpec {
    /// A plain chip named by its label.
    pub fn new(label: impl Into<String>, on: bool) -> Self {
        let label = label.into();
        Self {
            name: label.clone(),
            label,
            look: ChipLook {
                on,
                ..Default::default()
            },
        }
    }
    /// `.chip-dim`.
    pub fn dim(mut self, dim: bool) -> Self {
        self.look.dim = dim;
        self
    }
    /// With a soil dot.
    pub fn dot(mut self, c: Color32) -> Self {
        self.look.dot = Some(c);
        self
    }
    /// With a swatch bar `h` px tall.
    pub fn bar(mut self, c: Color32, h: f32) -> Self {
        self.look.bar = Some((c, h));
        self
    }
}

/// Lays out a `.chips` group wrapping at `width` (gap 4); returns the rects (from the group's
/// origin) and the group's size.
pub fn chips_layout(ui: &Ui, specs: &[ChipSpec], width: f32, gap: Vec2) -> (Vec<Rect>, Vec2) {
    let mut rects = Vec::with_capacity(specs.len());
    let (mut x, mut y, mut w_max) = (0.0f32, 0.0f32, 0.0f32);
    for s in specs {
        let size = chip_size(ui, &s.label, &s.look);
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
    specs: &[ChipSpec],
    rects: &[Rect],
) -> Option<usize> {
    let placed: Vec<Rect> = rects
        .iter()
        .map(|r| r.translate(origin.to_vec2()))
        .collect();
    let targets = hit::tile(&placed);
    let mut clicked = None;
    for (i, s) in specs.iter().enumerate() {
        let id = Id::unique(("builder-chip", salt, i));
        if chip_at(ui, id, placed[i], targets[i], &s.label, &s.name, s.look).clicked() {
            clicked = Some(i);
        }
    }
    clicked
}

/// A wrapping `.chips` group (`role="group" aria-label`) at the cursor, full width; allocates
/// its height and returns the index clicked.
pub fn chip_group(ui: &mut Ui, salt: &str, group: &str, specs: &[ChipSpec]) -> Option<usize> {
    let w = ui.available_width();
    let (rects, size) = chips_layout(ui, specs, w, vec2(CHIP_GAP, CHIP_GAP));
    let top = ui.cursor().min;
    let rect = Rect::from_min_size(top, vec2(w, size.y));
    let mut child = ui.new_child(
        UiBuilder::new()
            .max_rect(rect)
            .accessibility_role(Role::Group)
            .accessibility_label(group),
    );
    set_bounds(&child, rect);
    let clicked = chips_show(&mut child, salt, top, specs, &rects);
    ui.advance_cursor_after_rect(rect);
    clicked
}

/// A Conditions row (`flex-direction: column; gap: 4px`): the `.mut` 12 px caption over one
/// or more wrapping chip groups (`role="group" aria-label`), e.g. Wind's speeds and
/// directions. Allocates its height; returns `(group, chip)` clicked.
pub fn chip_rows(
    ui: &mut Ui,
    caption: &str,
    groups: &[(&str, &str, &[ChipSpec])],
) -> Option<(usize, usize)> {
    let w = ui.available_width();
    let top = ui.cursor().min;
    let font = lab_theme::font::body(12.0);
    let ch = css_line_height(ui, &font);
    let cg = galley(ui, caption, font, t::TEXT_MUTED);
    ui.painter().galley(
        pos2(top.x, top.y + (ch - cg.size().y) / 2.0),
        cg,
        t::TEXT_MUTED,
    );
    label_node(
        ui,
        ("builder-row-caption", caption),
        Rect::from_min_size(top, vec2(w, ch)),
        caption,
    );
    // place every group, then tile the hit targets of all their chips together (the groups
    // sit 4 px apart)
    let mut y = top.y + ch;
    let mut placed: Vec<(Rect, Vec<Rect>)> = Vec::new();
    for (_, _, specs) in groups {
        y += 4.0;
        let (rects, size) = chips_layout(ui, specs, w, vec2(CHIP_GAP, CHIP_GAP));
        let rect = Rect::from_min_size(pos2(top.x, y), vec2(w, size.y));
        let rects: Vec<Rect> = rects
            .iter()
            .map(|r| r.translate(rect.min.to_vec2()))
            .collect();
        placed.push((rect, rects));
        y += size.y;
    }
    let all: Vec<Rect> = placed.iter().flat_map(|(_, r)| r.iter().copied()).collect();
    let targets = hit::tile(&all);
    let mut k = 0;
    let mut clicked = None;
    for (gi, ((salt, group, specs), (rect, rects))) in groups.iter().zip(&placed).enumerate() {
        let mut child = ui.new_child(
            UiBuilder::new()
                .max_rect(*rect)
                .accessibility_role(Role::Group)
                .accessibility_label(*group),
        );
        set_bounds(&child, *rect);
        for (i, s) in specs.iter().enumerate() {
            let id = Id::unique(("builder-chip", *salt, i));
            if chip_at(
                &mut child, id, rects[i], targets[k], &s.label, &s.name, s.look,
            )
            .clicked()
            {
                clicked = Some((gi, i));
            }
            k += 1;
        }
    }
    ui.advance_cursor_after_rect(Rect::from_min_max(top, pos2(top.x + w, y)));
    clicked
}

/// [`chip_rows`] with one group named like the caption.
pub fn chip_row(ui: &mut Ui, salt: &str, caption: &str, specs: &[ChipSpec]) -> Option<usize> {
    chip_rows(ui, caption, &[(salt, caption, specs)]).map(|(_, i)| i)
}

/// Paints the star row as five-point stars in the accent, `size` px tall, from `left` at
/// centre height `cy`: filled for earned, outlined for not yet (MB-2: at the row's 12 px, not
/// tiny glyphs). Returns its width.
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
    stars_width(size)
}

/// Width of a star row of `size`.
pub fn stars_width(size: f32) -> f32 {
    size + 2.0 * size * 0.62
}

/// `.lvl` level row (`min-height: 40px; padding: 0 12px`, radius 8): the name left, the best
/// stars right (levels with a budget). AccessKit: a `Button` named by the level with its
/// pressed state; the best stars are its description ("Best: 1 of 3 stars").
pub fn level_row(
    ui: &mut Ui,
    id: Id,
    rect: Rect,
    name: &str,
    stars: Option<u8>,
    on: bool,
) -> Response {
    let resp = interact(ui, id, rect, true, Role::Button, name, Some(on));
    if let Some(n) = stars {
        let d = format!("Best: {n} of 3 stars");
        ui.ctx()
            .accesskit_node_builder(resp.id, |nb| nb.set_description(d));
    }
    let hovered = resp.hovered();
    let (fill, border, fg) = if on {
        (t::TINT, t::EDGE_ON, t::INK)
    } else if hovered {
        (t::ROW, b::CHIP_HOVER_BORDER, t::TEXT_STRONG)
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

/// A rail tool's icon: one of the shared icons, or the Found tool's (drawn here: the mockup's
/// `M11 5L4 13H18Z M8 13V20M14 13V20M3 13H19`, a pad on two piles).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolIcon {
    /// A shared icon (22 px tool paths).
    Shared(Icon),
    /// The Found tool.
    Found,
}

impl ToolIcon {
    fn paint(self, painter: &Painter, rect: Rect, color: Color32) {
        match self {
            ToolIcon::Shared(icon) => icon.paint(painter, rect, color),
            ToolIcon::Found => {
                let s = rect.width() / 22.0;
                let p = |x: f32, y: f32| rect.min + vec2(x * s, y * s);
                let stroke = Stroke::new(1.6 * s, color);
                painter.add(Shape::closed_line(
                    vec![p(11.0, 5.0), p(4.0, 13.0), p(18.0, 13.0)],
                    stroke,
                ));
                for (a, c) in [
                    (p(8.0, 13.0), p(8.0, 20.0)),
                    (p(14.0, 13.0), p(14.0, 20.0)),
                    (p(3.0, 13.0), p(19.0, 13.0)),
                ] {
                    painter.line_segment([a, c], stroke);
                    painter.circle_filled(a, 0.8 * s, color);
                    painter.circle_filled(c, 0.8 * s, color);
                }
            }
        }
    }
}

/// `.tool` rail button: 60 × 56, radius 10, the 22 px icon over the 11 px label (gap 4).
/// AccessKit: a `Button` named by the label; tools report pressed, Undo / Clear don't.
pub fn tool_button(
    ui: &mut Ui,
    id: Id,
    rect: Rect,
    icon: ToolIcon,
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

/// The height of a `.pill`: `padding: 4px 10px`, 1 px border, Inter 600 12 px line.
pub fn pill_height(ui: &Ui) -> f32 {
    2.0 + 8.0 + css_line_height(ui, &lab_theme::font::semibold(12.0))
}

/// `.pill` with an 8 px `.dot` (the status): `#141414`, `#2E2E2E` border, Inter 600 12 px in
/// `color`, wrapped at `max_w` (two lines when the aside is narrow, as Chrome does). Returns
/// its rect; AccessKit: a `Label`.
pub fn status_pill(ui: &mut Ui, origin: Pos2, label: &str, color: Color32, max_w: f32) -> Rect {
    let font = lab_theme::font::semibold(12.0);
    let lh = css_line_height(ui, &font);
    let text_max = (max_w - 2.0 - 20.0 - 8.0 - 6.0).max(20.0);
    let g = wrapped(ui, label, font, color, text_max, Some(lh));
    let rect = Rect::from_min_size(
        origin,
        vec2(2.0 + 20.0 + 8.0 + 6.0 + g.size().x, 2.0 + 8.0 + g.size().y),
    );
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
    painter.galley(pos2(rect.left() + 11.0 + 14.0, rect.top() + 5.0), g, color);
    label_node(ui, ("builder-pill", label), rect, label);
    rect
}

/// The width of an info `.pill` (text only).
pub fn info_pill_width(ui: &Ui, label: &str) -> f32 {
    2.0 + 20.0
        + galley(ui, label, lab_theme::font::semibold(12.0), t::TEXT_DIM)
            .size()
            .x
}

/// An info `.pill` without a dot (the goal panel's "Car · 1.5 t", "Reach 64 m"), `#D9D9D9`.
pub fn info_pill(ui: &mut Ui, origin: Pos2, label: &str) -> Rect {
    let g = galley(ui, label, lab_theme::font::semibold(12.0), t::TEXT_DIM);
    let rect = Rect::from_min_size(origin, vec2(2.0 + 20.0 + g.size().x, pill_height(ui)));
    let painter = ui.painter();
    painter.rect(
        rect,
        CornerRadius::same(t::radius::PILL),
        t::CONTROL,
        Stroke::new(1.0, t::BORDER_STRONG),
        StrokeKind::Inside,
    );
    painter.galley(
        pos2(rect.left() + 11.0, text_top(rect.center().y, &g)),
        g,
        t::TEXT_DIM,
    );
    label_node(ui, ("builder-info", label, origin.x as i32), rect, label);
    rect
}

/// A HUD `.pill` (`background: rgba(10,10,10,.85)`): the `.mut` key (weight 500), `gap: 6px`,
/// the value in `color`. Returns its size without drawing when `at` is `None`.
pub fn hud_pill(
    ui: &Ui,
    at: Option<Pos2>,
    key: &str,
    value: &str,
    color: Color32,
) -> (Vec2, String) {
    let kg = galley(ui, key, lab_theme::font::medium(12.0), t::TEXT_MUTED);
    let vg = galley(ui, value, lab_theme::font::semibold(12.0), color);
    let size = vec2(
        2.0 + 20.0 + kg.size().x + 6.0 + vg.size().x,
        pill_height(ui),
    );
    let label = format!("{key} {value}");
    if let Some(o) = at {
        let rect = Rect::from_min_size(o, size);
        let painter = ui.painter();
        painter.rect(
            rect,
            CornerRadius::same(t::radius::PILL),
            b::HUD,
            Stroke::new(1.0, t::BORDER_STRONG),
            StrokeKind::Inside,
        );
        let cy = rect.center().y;
        let x = rect.left() + 11.0;
        painter.galley(pos2(x, text_top(cy, &kg)), kg.clone(), t::TEXT_MUTED);
        painter.galley(pos2(x + kg.size().x + 6.0, text_top(cy, &vg)), vg, color);
        label_node(ui, ("builder-hud", key), rect, &label);
    }
    (size, label)
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

/// A labelled region for a section (AccessKit `Group`), drawn by `add` from the cursor with
/// `gap` between its children; allocates what it drew.
pub fn section(ui: &mut Ui, label: &str, gap: f32, add: impl FnOnce(&mut Ui)) {
    let w = ui.available_width();
    let top = ui.cursor().min;
    let mut child = ui.new_child(
        UiBuilder::new()
            .max_rect(Rect::from_min_size(top, vec2(w, f32::INFINITY)))
            .layout(Layout::top_down(Align::Min))
            .accessibility_role(Role::Group)
            .accessibility_label(label),
    );
    child.spacing_mut().item_spacing = vec2(gap, gap);
    add(&mut child);
    let used = Rect::from_min_size(top, vec2(w, child.min_rect().height()));
    set_bounds(&child, used);
    ui.advance_cursor_after_rect(used);
}

/// A framed block (`.panel` / `.well`, `padding: 12px; gap: 10px`), an AccessKit `Group`.
pub fn framed(ui: &mut Ui, label: &str, fill: Color32, radius: u8, add: impl FnOnce(&mut Ui)) {
    let w = ui.available_width();
    let top = ui.cursor().min;
    let bg = ui.painter().add(Shape::Noop);
    let inner = Rect::from_min_size(top + vec2(13.0, 13.0), vec2(w - 26.0, f32::INFINITY));
    let mut child = ui.new_child(
        UiBuilder::new()
            .max_rect(inner)
            .layout(Layout::top_down(Align::Min))
            .accessibility_role(Role::Group)
            .accessibility_label(label),
    );
    child.spacing_mut().item_spacing = vec2(10.0, 10.0);
    add(&mut child);
    let h = child.min_rect().height() + 26.0;
    let rect = Rect::from_min_size(top, vec2(w, h));
    ui.painter().set(
        bg,
        RectShape::new(
            rect,
            CornerRadius::same(radius),
            fill,
            Stroke::new(1.0, t::BORDER),
            StrokeKind::Inside,
        ),
    );
    set_bounds(&child, rect);
    ui.advance_cursor_after_rect(rect);
}

/// Extra wrap width for paragraphs: egui's glyph advances come out a hair wider than
/// Chrome's, so a line Chrome just fits would wrap a word early; 2 px stays in the padding.
pub const WRAP_SLACK: f32 = 2.0;

/// A paragraph at the cursor (wrapped at the available width, CSS line height `size × lh`).
pub fn para(ui: &mut Ui, text: &str, size: f32, lh: f32, color: Color32) {
    let w = ui.available_width();
    let g = wrapped(
        ui,
        text,
        lab_theme::font::body(size),
        color,
        w + WRAP_SLACK,
        Some(size * lh),
    );
    let rect = Rect::from_min_size(ui.cursor().min, vec2(w, g.size().y));
    ui.painter().galley(rect.min, g, color);
    label_node(ui, ("builder-para", text), rect, text);
    ui.advance_cursor_after_rect(rect);
}

/// A `.lbl` eyebrow's galley wrapped at `width` (uppercase, 11 px semibold, `.08em`).
pub fn lbl_wrapped(ui: &Ui, text: &str, width: f32) -> std::sync::Arc<eframe::egui::Galley> {
    let g = lbl_galley(ui, text, t::TEXT_MUTED);
    if g.size().x <= width {
        return g;
    }
    crate::widgets::text::wrapped_job(
        ui,
        &text.to_uppercase(),
        lab_theme::font::semibold(11.0),
        t::TEXT_MUTED,
        width,
        None,
        crate::widgets::text::LBL_SPACING,
        false,
    )
}

/// A `.lbl` eyebrow at the cursor (`<h2 class="lbl">`), wrapped when long.
pub fn eyebrow(ui: &mut Ui, text: &str) {
    let w = ui.available_width();
    let g = lbl_galley(ui, text, t::TEXT_MUTED);
    if g.size().x <= w {
        crate::widgets::lbl(ui, text);
    } else {
        let g = lbl_wrapped(ui, text, w);
        let rect = Rect::from_min_size(ui.cursor().min, vec2(w, g.size().y));
        ui.painter().galley(rect.min, g, t::TEXT_MUTED);
        label_node(ui, ("builder-lbl", text), rect, text);
        ui.advance_cursor_after_rect(rect);
    }
}
