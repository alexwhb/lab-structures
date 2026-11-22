//! Drawing helpers for the Materials screen: the SVG viewBox mapping (`preserveAspectRatio`
//! "xMidYMid meet"), polygon fills, the status `.pill` and small text helpers.

use crate::widgets::text::{css_line_height, galley, text_top, wrapped, wrapped_job};
use eframe::egui::{
    Color32, CornerRadius, FontId, Galley, Painter, Pos2, Rect, Shape, Stroke, StrokeKind, Ui,
    pos2, vec2,
};
use sp_engine::materials::Tone;
use sp_theme as t;
use std::sync::Arc;

/// An SVG `viewBox` drawn into a box with the default `preserveAspectRatio` (uniform scale to
/// fit, centred).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewBox {
    /// Screen position of the viewBox origin.
    pub origin: Pos2,
    /// Screen px per viewBox unit.
    pub scale: f32,
}

impl ViewBox {
    /// The viewBox `size` fitted into `rect`.
    pub fn meet(rect: Rect, size: [f64; 2]) -> Self {
        let (w, h) = (size[0] as f32, size[1] as f32);
        let scale = (rect.width() / w).min(rect.height() / h).max(0.0);
        let origin = rect.center() - vec2(w * scale, h * scale) / 2.0;
        Self { origin, scale }
    }

    /// A viewBox point on screen.
    pub fn pt(&self, p: [f64; 2]) -> Pos2 {
        self.origin + vec2(p[0] as f32, p[1] as f32) * self.scale
    }

    /// Many points.
    pub fn pts(&self, ps: &[[f64; 2]]) -> Vec<Pos2> {
        ps.iter().map(|p| self.pt(*p)).collect()
    }

    /// A viewBox rectangle `[x0, y0, x1, y1]` on screen.
    pub fn rect(&self, r: [f64; 4]) -> Rect {
        Rect::from_two_pos(self.pt([r[0], r[1]]), self.pt([r[2], r[3]]))
    }

    /// A stroke width in viewBox units on screen.
    pub fn w(&self, width: f32) -> f32 {
        width * self.scale
    }
}

/// Fills a simple (possibly concave) polygon: the shared ear-clipping fill.
pub use crate::widgets::icons::fill_polygon;

/// Fills a convex polygon (anti-aliased).
pub fn fill_convex(painter: &Painter, ps: Vec<Pos2>, fill: Color32) {
    painter.add(Shape::convex_polygon(ps, fill, Stroke::NONE));
}

/// Strokes a closed outline.
pub fn outline(painter: &Painter, ps: &[Pos2], stroke: Stroke) {
    if ps.len() >= 2 {
        painter.add(Shape::closed_line(ps.to_vec(), stroke));
    }
}

/// The colour of a tone.
pub fn tone_color(tone: Tone) -> Color32 {
    match tone {
        Tone::Muted => t::TEXT_MUTED,
        Tone::Ok => t::OK,
        Tone::Accent => t::ACCENT,
        Tone::Caution => t::CAUTION,
        Tone::Danger => t::DANGER,
    }
}

/// `.pill` metrics: `padding: 4px 10px; gap: 6px; border: 1px; font: 600 12px`, 8 px dot.
pub fn pill_size(ui: &Ui, text: &str) -> eframe::egui::Vec2 {
    let font = lab_theme::font::semibold(12.0);
    let g = galley(ui, text, font.clone(), t::TEXT);
    let line = css_line_height(ui, &font);
    vec2(2.0 * 11.0 + 8.0 + 6.0 + g.size().x, 2.0 * 5.0 + line)
}

/// Paints a `.pill` (`#141414`, `#2E2E2E` border, radius 999) with a dot and text in `color`
/// (`dot` may differ, e.g. an unlit challenge chip).
pub fn paint_pill(ui: &Ui, rect: Rect, text: &str, color: Color32, dot: Color32) {
    let p = ui.painter();
    p.rect(
        rect,
        CornerRadius::same(t::radius::PILL),
        t::CONTROL,
        Stroke::new(1.0, t::BORDER_STRONG),
        StrokeKind::Inside,
    );
    let cy = rect.center().y;
    let x = rect.left() + 11.0;
    p.circle_filled(pos2(x + 4.0, cy), 4.0, dot);
    let g = galley(ui, text, lab_theme::font::semibold(12.0), color);
    p.galley(pos2(x + 8.0 + 6.0, text_top(cy, &g)), g, color);
}

/// A wrapped paragraph galley.
pub fn para(
    ui: &Ui,
    text: &str,
    font: FontId,
    color: Color32,
    width: f32,
    line_h: Option<f32>,
) -> Arc<Galley> {
    wrapped(ui, text, font, color, width, line_h)
}

/// A wrapped `.lbl` eyebrow galley (uppercase, letter-spaced, 11 px 600).
pub fn lbl_wrapped(ui: &Ui, text: &str, color: Color32, width: f32) -> Arc<Galley> {
    wrapped_job(
        ui,
        &text.to_uppercase(),
        lab_theme::font::semibold(11.0),
        color,
        width,
        None,
        crate::widgets::text::LBL_SPACING,
        false,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meet_centres_and_scales() {
        let r = Rect::from_min_size(pos2(0.0, 0.0), vec2(542.0, 400.0));
        let vb = ViewBox::meet(r, [560.0, 400.0]);
        assert!((vb.scale - 542.0 / 560.0).abs() < 1e-6);
        assert!((vb.pt([0.0, 0.0]).y - (400.0 - 400.0 * vb.scale) / 2.0).abs() < 1e-4);
        assert!((vb.pt([560.0, 0.0]).x - 542.0).abs() < 1e-3);
    }
}
