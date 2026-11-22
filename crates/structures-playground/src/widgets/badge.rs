//! `.badge` — `display: inline-flex; align-items: center; gap: 6px; padding: 3px 8px;
//! border-radius: 5px; font: 500 11px 'JetBrains Mono'; border: 1px solid #2E2E2E; color:
//! #D9D9D9; background: #121212`, with an optional 8 px `.dot` (PROV-1 provenance colours).
//! AccessKit: a `Label` named by its text.

use super::text::{css_line_height, galley, text_top};
use crate::screens::Badge;
use eframe::egui::{
    Color32, CornerRadius, Rect, Response, Sense, Stroke, StrokeKind, Ui, Vec2, WidgetInfo,
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
    let g = galley(ui, &badge.text, font(), t::TEXT_DIM);
    let dot = if badge.dot.is_some() { DOT + GAP } else { 0.0 };
    // the line box is CSS `line-height: normal` for the mono 11 px
    let line = css_line_height(ui, &font());
    vec2(
        2.0 * (PAD_X + 1.0) + dot + g.size().x,
        2.0 * (PAD_Y + 1.0) + line.max(DOT),
    )
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
    let cy = rect.center().y;
    let mut x = rect.left() + PAD_X + 1.0;
    if let Some(c) = badge.dot {
        p.circle_filled(pos2(x + DOT / 2.0, cy), DOT / 2.0, c);
        x += DOT + GAP;
    }
    let g = galley(ui, &badge.text, font(), look.text);
    p.galley(pos2(x, text_top(cy, &g)), g, look.text);
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
