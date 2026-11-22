//! `.badge` — `display: inline-flex; align-items: center; gap: 6px; padding: 3px 8px;
//! border-radius: 5px; font: 500 11px 'JetBrains Mono'; border: 1px solid #2E2E2E; color:
//! #D9D9D9; background: #121212`, with an optional 8 px `.dot` (PROV-1 provenance colours).
//! AccessKit: a `Label` named by its text.

use super::text::{css_line_height, galley, wrapped};
use crate::screens::Badge;
use eframe::egui::{
    Color32, CornerRadius, Galley, Rect, Response, Sense, Stroke, StrokeKind, Ui, Vec2, WidgetInfo,
    accesskit::Role, pos2, vec2,
};
use sp_theme as t;

const PAD_X: f32 = 8.0;
const PAD_Y: f32 = 3.0;
const DOT: f32 = 8.0;
const GAP: f32 = 6.0;

fn font() -> eframe::egui::FontId {
    lab_theme::font::mono_medium(11.0)
}

/// The badge's colours: text, border (`.badge` defaults; Series' danger badge overrides).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BadgeLook {
    /// Text colour.
    pub text: Color32,
    /// Border colour.
    pub border: Color32,
}

impl Default for BadgeLook {
    fn default() -> Self {
        Self {
            text: t::TEXT_DIM,
            border: t::BORDER_STRONG,
        }
    }
}

impl BadgeLook {
    /// `color: #E8A39E; border-color: #5C2A25` ("Educational. Not for real structural design.").
    pub const DANGER: BadgeLook = BadgeLook {
        text: t::DANGER_INK,
        border: t::DANGER_EDGE,
    };
}

/// The badge's drawn size.
pub fn badge_size(ui: &Ui, badge: &Badge) -> Vec2 {
    badge_size_in(ui, badge, f32::INFINITY)
}

/// The badge's size when it may be at most `max_w` wide (its text wraps).
pub fn badge_size_in(ui: &Ui, badge: &Badge, max_w: f32) -> Vec2 {
    let dot = if badge.dot.is_some() { DOT + GAP } else { 0.0 };
    let chrome = 2.0 * (PAD_X + 1.0) + dot;
    let g = text_galley(ui, badge, t::TEXT_DIM, max_w - chrome);
    vec2(
        chrome + g.size().x,
        2.0 * (PAD_Y + 1.0) + g.size().y.max(DOT),
    )
}

/// The badge's text, wrapped at `width` (one line when it fits), in CSS `line-height: normal`
/// line boxes.
fn text_galley(ui: &Ui, badge: &Badge, color: Color32, width: f32) -> std::sync::Arc<Galley> {
    let line = css_line_height(ui, &font());
    let one = galley(ui, &badge.text, font(), color);
    if one.size().x <= width {
        return wrapped(ui, &badge.text, font(), color, f32::INFINITY, Some(line));
    }
    wrapped(ui, &badge.text, font(), color, width.max(24.0), Some(line))
}

/// Paints `badge` into `rect` (sized with [`badge_size`]) and registers its label.
pub fn paint_badge(ui: &Ui, rect: Rect, badge: &Badge, look: BadgeLook) -> Response {
    let p = ui.painter();
    p.rect(
        rect,
        CornerRadius::same(t::radius::BADGE),
        t::ROW,
        Stroke::new(1.0, look.border),
        StrokeKind::Inside,
    );
    let mut x = rect.left() + PAD_X + 1.0;
    let dot = if badge.dot.is_some() { DOT + GAP } else { 0.0 };
    let g = text_galley(
        ui,
        badge,
        look.text,
        rect.width() - 2.0 * (PAD_X + 1.0) - dot,
    );
    let top = rect.top() + PAD_Y + 1.0;
    if let Some(c) = badge.dot {
        // centred on the first line (`align-items: center` for one line)
        let first = g.rows.first().map_or(g.size().y, |r| r.rect().height());
        let cy = if g.rows.len() > 1 {
            top + first / 2.0
        } else {
            rect.center().y
        };
        p.circle_filled(pos2(x + DOT / 2.0, cy), DOT / 2.0, c);
        x += dot;
    }
    let y = if g.rows.len() > 1 {
        top
    } else {
        rect.center().y - g.size().y / 2.0
    };
    p.galley(pos2(x, y), g, look.text);
    let text = badge.text.clone();
    let resp = ui.interact(
        rect,
        ui.scope_id().with(("badge", &badge.text)),
        Sense::hover(),
    );
    resp.widget_info(|| WidgetInfo::labeled(Role::Label, true, &text));
    resp
}

/// Draws `badge` at the cursor.
pub fn badge(ui: &mut Ui, badge: &Badge) -> Response {
    badge_with(ui, badge, BadgeLook::default())
}

/// Draws `badge` at the cursor with other colours.
pub fn badge_with(ui: &mut Ui, badge: &Badge, look: BadgeLook) -> Response {
    let size = badge_size(ui, badge);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    paint_badge(ui, rect, badge, look)
}
