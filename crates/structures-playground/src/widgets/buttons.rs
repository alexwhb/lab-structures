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

/// `.tog` — a pill toggle: 28 px, `padding: 0 10px`, Inter 500 12 px, `#141414` with a
/// `#2E2E2E` border and muted text; on: edge border, tint fill, ink text. AccessKit: a toggle
/// `Button` with its pressed state.
#[must_use = "call .show(ui)"]
pub struct Toggle<'a> {
    label: &'a str,
    on: bool,
    name: Option<&'a str>,
}

impl<'a> Toggle<'a> {
    /// A toggle showing `label`, currently `on`.
    pub fn new(label: &'a str, on: bool) -> Self {
        Self {
            label,
            on,
            name: None,
        }
    }

    /// Accessible name when it differs from the label.
    pub fn accessible_name(mut self, name: &'a str) -> Self {
        self.name = Some(name);
        self
    }

    /// Draws it; `.clicked()` means "flip it".
    pub fn show(self, ui: &mut Ui) -> Response {
        let font = lab_theme::font::medium(12.0);
        let fg = if self.on { t::INK } else { t::TEXT_MUTED };
        let g = galley(ui, self.label, font, fg);
        let size = vec2(g.size().x + 22.0, super::seg::MINI_H);
        let (id, rect) = ui.allocate_space(size);
        let resp = interact(
            ui,
            id,
            hit::target(rect),
            true,
            Role::Button,
            self.name.unwrap_or(self.label),
            Some(self.on),
        );
        let (fill, border) = if self.on {
            (t::TINT, t::EDGE)
        } else {
            (t::CONTROL, t::BORDER_STRONG)
        };
        let painter = ui.painter();
        painter.rect(
            rect,
            CornerRadius::same(t::radius::PILL),
            fill,
            Stroke::new(1.0, border),
            StrokeKind::Inside,
        );
        painter.galley(
            pos2(rect.left() + 11.0, text_top(rect.center().y, &g)),
            g,
            fg,
        );
        focus_ring_if(ui, &resp, rect, CornerRadius::same(t::radius::PILL));
        resp
    }
}
