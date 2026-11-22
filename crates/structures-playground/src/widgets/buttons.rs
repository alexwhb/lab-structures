//! `.ghost` / `.prim` (+ big) buttons and the pill toggle.

use super::icons::Icon;
use super::text::{galley, text_top};
use super::{hit, interact};
use eframe::egui::{
    Color32, CornerRadius, Rect, Response, Stroke, StrokeKind, Ui, accesskit::Role, pos2, vec2,
};
use lab_ui::paint::focus_ring_if;
use sp_theme as t;

/// The two button looks of the mockup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonKind {
    /// `.ghost`: `#171717`, 1 px `#2E2E2E`, `#D9D9D9` text; hover `#1F1F1F`.
    Ghost,
    /// `.prim`: tint `#24221C`, edge `#4D4636` border, ink `#E9DFC6` text; hover `#2E2B23`.
    Prim,
}

/// A `.ghost` button: `ghost("Step").icon(Icon::Step).show(ui)`.
pub fn ghost(label: &str) -> Button<'_> {
    Button::new(ButtonKind::Ghost, label)
}

/// A `.prim` button: `prim("Pause").icon(Icon::Pause).show(ui)`.
pub fn prim(label: &str) -> Button<'_> {
    Button::new(ButtonKind::Prim, label)
}

/// `.ghost` / `.prim`: 36 px tall, `padding: 0 12px`, radius 6, Inter 500 13 px, an optional
/// 12 px icon 8 px before the label. `.big` is 44 px, 14 px text, centred, full width.
/// AccessKit: a `Button` (or `Link` with [`Button::link`]) named by its label.
#[must_use = "call .show(ui)"]
pub struct Button<'a> {
    kind: ButtonKind,
    label: &'a str,
    icon: Option<Icon>,
    height: f32,
    font_size: f32,
    centered: bool,
    full_width: bool,
    min_width: f32,
    enabled: bool,
    pressed: Option<bool>,
    name: Option<&'a str>,
    link: bool,
    hit: Option<Rect>,
}

impl<'a> Button<'a> {
    /// A button of `kind` labelled `label`.
    pub fn new(kind: ButtonKind, label: &'a str) -> Self {
        Self {
            kind,
            label,
            icon: None,
            height: t::size::BUTTON,
            font_size: 13.0,
            centered: false,
            full_width: false,
            min_width: 0.0,
            enabled: true,
            pressed: None,
            name: None,
            link: false,
            hit: None,
        }
    }

    /// A leading icon (drawn at its CSS size, `currentColor` = the text colour).
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Big: 44 px tall, 14 px text, centred, full width (the Bridge's "Test it: add the
    /// load").
    pub fn big(mut self) -> Self {
        self.height = t::size::BUTTON_BIG;
        self.font_size = 14.0;
        self.centered = true;
        self.full_width = true;
        self
    }

    /// Another height (the mockup uses `style="height: 30px"` on some ghosts).
    pub fn height(mut self, h: f32) -> Self {
        self.height = h;
        self
    }

    /// Fill the available width (`<a class="ghost">` in a flex column), label left-aligned.
    pub fn full_width(mut self) -> Self {
        self.full_width = true;
        self
    }

    /// Centre the label (`justify-content: center`).
    pub fn centered(mut self) -> Self {
        self.centered = true;
        self
    }

    /// A minimum drawn width.
    pub fn min_width(mut self, w: f32) -> Self {
        self.min_width = w;
        self
    }

    /// Disabled: drawn at 45 % and reported disabled.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Report a pressed state (`aria-pressed`); pressed ghosts draw in the prim colours.
    pub fn pressed(mut self, pressed: bool) -> Self {
        self.pressed = Some(pressed);
        self
    }

    /// The accessible name when it differs from the visible label ("−1" → "Previous rule").
    pub fn accessible_name(mut self, name: &'a str) -> Self {
        self.name = Some(name);
        self
    }

    /// Report the AccessKit role `Link` (`<a class="ghost">`).
    pub fn link(mut self) -> Self {
        self.link = true;
        self
    }

    /// An explicit hit area (groups that tile their targets pass one).
    pub fn hit(mut self, target: Rect) -> Self {
        self.hit = Some(target);
        self
    }

    /// `.ghost` is Inter 500, `.prim` 600.
    fn font(&self) -> eframe::egui::FontId {
        match self.kind {
            ButtonKind::Ghost => lab_theme::font::medium(self.font_size),
            ButtonKind::Prim => lab_theme::font::semibold(self.font_size),
        }
    }

    /// The drawn size.
    pub fn desired_size(&self, ui: &Ui) -> eframe::egui::Vec2 {
        let text_w = galley(ui, self.label, self.font(), Color32::WHITE).size().x;
        let icon_w = self.icon.map_or(0.0, |i| i.css_size().x + 8.0);
        let mut w = (1.0 + 12.0 + icon_w + text_w + 12.0 + 1.0).max(self.min_width);
        if self.full_width {
            w = w.max(ui.available_width());
        }
        vec2(w, self.height)
    }

    /// Draws the button at the cursor.
    pub fn show(self, ui: &mut Ui) -> Response {
        let size = self.desired_size(ui);
        let (id, alloc) = ui.allocate_space(size);
        let rect = ui.layout().align_size_within_rect(size, alloc);
        self.paint_at(ui, id, rect)
    }

    /// Draws the button into `rect` (for custom layouts).
    pub fn show_at(self, ui: &mut Ui, rect: Rect) -> Response {
        let id = ui.next_auto_id();
        ui.skip_ahead_auto_ids(1);
        self.paint_at(ui, id, rect)
    }

    fn paint_at(self, ui: &mut Ui, id: eframe::egui::Id, rect: Rect) -> Response {
        let target = self.hit.unwrap_or_else(|| hit::target(rect));
        let role = if self.link { Role::Link } else { Role::Button };
        let name = self.name.unwrap_or(self.label);
        let resp = interact(ui, id, target, self.enabled, role, name, self.pressed);
        if ui.is_rect_visible(rect) {
            let hovered = resp.hovered() && self.enabled;
            let on = self.pressed == Some(true);
            let (fill, border, mut fg) = match (self.kind, on) {
                (ButtonKind::Prim, _) | (ButtonKind::Ghost, true) => (
                    if hovered { t::TINT_HOVER } else { t::TINT },
                    t::EDGE,
                    t::INK,
                ),
                (ButtonKind::Ghost, false) => (
                    if hovered { t::GHOST_HOVER } else { t::GHOST },
                    t::BORDER_STRONG,
                    t::TEXT_DIM,
                ),
            };
            if !self.enabled {
                fg = fg.gamma_multiply(0.45);
            }
            let painter = ui.painter();
            painter.rect(
                rect,
                CornerRadius::same(t::radius::BUTTON),
                fill,
                Stroke::new(1.0, border),
                StrokeKind::Inside,
            );
            let g = galley(ui, self.label, self.font(), fg);
            let icon_w = self.icon.map_or(0.0, |i| i.css_size().x + 8.0);
            let content = icon_w + g.size().x;
            let mut x = if self.centered {
                rect.center().x - content / 2.0
            } else {
                rect.left() + 13.0
            };
            let cy = rect.center().y;
            if let Some(icon) = self.icon {
                let s = icon.css_size();
                icon.paint(
                    painter,
                    Rect::from_center_size(pos2(x + s.x / 2.0, cy), s),
                    fg,
                );
                x += s.x + 8.0;
            }
            painter.galley(pos2(x, text_top(cy, &g)), g, fg);
            focus_ring_if(ui, &resp, rect, CornerRadius::same(t::radius::BUTTON));
        }
        resp
    }
}

/// `.chip` — a pill toggle (Size chips, "Exaggerate sag", "Show forces", the "Built from"
/// materials, Weight chips): `min-height: 34px; padding: 0 12px; gap: 6px; border: 1px solid
/// #2E2E2E; border-radius: 999px; background: #141414; color: #C9C9C9; font: 500 12px Inter`;
/// hover `#F5F5F5` text with a `#3A3A3A` border; on (`.chip-on`): tint `#24221C`, `#6A604A`
/// border, ink text; `.chip-dim` draws it at 45 % (e.g. a support the level doesn't allow).
/// An optional 8 px colour dot leads the label (material chips). AccessKit: a toggle `Button`
/// with its pressed state (`aria-pressed`).
#[must_use = "call .show(ui)"]
pub struct Toggle<'a> {
    label: &'a str,
    on: bool,
    name: Option<&'a str>,
    dot: Option<Color32>,
    dim: bool,
    enabled: bool,
}

impl<'a> Toggle<'a> {
    /// A chip showing `label`, currently `on`.
    pub fn new(label: &'a str, on: bool) -> Self {
        Self {
            label,
            on,
            name: None,
            dot: None,
            dim: false,
            enabled: true,
        }
    }

    /// Accessible name when it differs from the label.
    pub fn accessible_name(mut self, name: &'a str) -> Self {
        self.name = Some(name);
        self
    }

    /// A leading 8 px dot in `color` (the material chips' swatches).
    pub fn dot(mut self, color: Color32) -> Self {
        self.dot = Some(color);
        self
    }

    /// `.chip-dim`: drawn at 45 % opacity (still clickable; the screen explains why).
    pub fn dimmed(mut self, dim: bool) -> Self {
        self.dim = dim;
        self
    }

    /// Disabled: drawn dimmed, reported disabled, not clickable.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    fn font() -> eframe::egui::FontId {
        lab_theme::font::medium(12.0)
    }

    /// The drawn size.
    pub fn desired_size(&self, ui: &Ui) -> eframe::egui::Vec2 {
        let g = galley(ui, self.label, Self::font(), t::TEXT_CHIP);
        let dot = if self.dot.is_some() { 8.0 + 6.0 } else { 0.0 };
        vec2(1.0 + 12.0 + dot + g.size().x + 12.0 + 1.0, t::size::CHIP)
    }

    /// Draws it at the cursor; `.clicked()` means "flip it".
    pub fn show(self, ui: &mut Ui) -> Response {
        let size = self.desired_size(ui);
        let (id, rect) = ui.allocate_space(size);
        self.paint(ui, id, rect)
    }

    /// Draws it into `rect` (for custom layouts).
    pub fn show_at(self, ui: &mut Ui, rect: Rect) -> Response {
        let id = ui.next_auto_id();
        ui.skip_ahead_auto_ids(1);
        self.paint(ui, id, rect)
    }

    fn paint(self, ui: &Ui, id: eframe::egui::Id, rect: Rect) -> Response {
        let resp = interact(
            ui,
            id,
            hit::target(rect),
            self.enabled,
            Role::Button,
            self.name.unwrap_or(self.label),
            Some(self.on),
        );
        let hovered = resp.hovered() && self.enabled;
        let (fill, border, mut fg) = if self.on {
            (t::TINT, t::EDGE_ON, t::INK)
        } else if hovered {
            (t::CONTROL, t::BORDER_OVERLAY, t::TEXT_STRONG)
        } else {
            (t::CONTROL, t::BORDER_STRONG, t::TEXT_CHIP)
        };
        let alpha = if self.dim || !self.enabled { 0.45 } else { 1.0 };
        let fade = |c: Color32| c.gamma_multiply(alpha);
        fg = fade(fg);
        let painter = ui.painter();
        painter.rect(
            rect,
            CornerRadius::same(t::radius::PILL),
            fade(fill),
            Stroke::new(1.0, fade(border)),
            StrokeKind::Inside,
        );
        let mut x = rect.left() + 13.0;
        let cy = rect.center().y;
        if let Some(c) = self.dot {
            painter.circle_filled(pos2(x + 4.0, cy), 4.0, fade(c));
            x += 8.0 + 6.0;
        }
        let g = galley(ui, self.label, Self::font(), fg);
        painter.galley(pos2(x, text_top(cy, &g)), g, fg);
        focus_ring_if(ui, &resp, rect, CornerRadius::same(t::radius::PILL));
        resp
    }
}

/// `input[type=checkbox]` in a `<label>` row (Tower "Tuned mass damper at the top", Ground
/// "Earthquake shaking", Advanced "Cross-braced"): an 18 px box as Chrome draws it with
/// `accent-color: #D6C7A1` — white with a 1 px `#767676` border when off, the accent with a
/// dark check when on (radius 2) — then the 13 px label, `gap: 10px`, `min-height: 44px`
/// (Advanced: `.gap(8.0).height(40.0)`). The whole row is the target. AccessKit: a `CheckBox`
/// named by the label with its checked state.
#[must_use = "call .show(ui)"]
pub struct Checkbox<'a> {
    label: &'a str,
    checked: bool,
    gap: f32,
    height: f32,
    enabled: bool,
}

impl<'a> Checkbox<'a> {
    /// A checkbox row.
    pub fn new(label: &'a str, checked: bool) -> Self {
        Self {
            label,
            checked,
            gap: 10.0,
            height: t::size::MIN_TARGET,
            enabled: true,
        }
    }

    /// The gap between box and label.
    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    /// The row's min-height.
    pub fn height(mut self, h: f32) -> Self {
        self.height = h;
        self
    }

    /// Disabled.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Draws it at the cursor; `.clicked()` means "flip it".
    pub fn show(self, ui: &mut Ui) -> Response {
        let font = lab_theme::font::body(13.0);
        let g = galley(ui, self.label, font, t::TEXT);
        let b = t::size::CHECKBOX;
        let size = vec2(b + self.gap + g.size().x, self.height);
        let (id, rect) = ui.allocate_space(size);
        let resp = ui.interact(hit::target(rect), id, super::sense(self.enabled));
        super::reveal_on_focus(&resp);
        let (label, enabled, checked) = (self.label.to_string(), self.enabled, self.checked);
        resp.widget_info(|| {
            eframe::egui::WidgetInfo::selected(Role::CheckBox, enabled, checked, &label)
        });
        let resp = if self.enabled {
            resp.on_hover_cursor(eframe::egui::CursorIcon::PointingHand)
        } else {
            resp
        };
        let alpha = if self.enabled { 1.0 } else { 0.45 };
        let cy = rect.center().y;
        let boxr = Rect::from_min_size(pos2(rect.left(), cy - b / 2.0), vec2(b, b));
        let painter = ui.painter();
        if self.checked {
            painter.rect_filled(boxr, CornerRadius::same(2), t::ACCENT.gamma_multiply(alpha));
            let s = Stroke::new(2.0, t::PAPER.base.gamma_multiply(alpha));
            let p = |x: f32, y: f32| pos2(boxr.left() + x, boxr.top() + y);
            painter.line_segment([p(4.5, 9.5), p(7.5, 12.5)], s);
            painter.line_segment([p(7.5, 12.5), p(13.5, 5.5)], s);
        } else {
            painter.rect(
                boxr,
                CornerRadius::same(2),
                t::CHECK_BOX.gamma_multiply(alpha),
                Stroke::new(1.0, t::CHECK_BORDER.gamma_multiply(alpha)),
                StrokeKind::Inside,
            );
        }
        let fg = t::TEXT.gamma_multiply(alpha);
        painter.galley(pos2(boxr.right() + self.gap, text_top(cy, &g)), g, fg);
        focus_ring_if(ui, &resp, boxr.expand(2.0), CornerRadius::same(4));
        resp
    }
}
