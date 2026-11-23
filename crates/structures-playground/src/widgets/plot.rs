//! The population plot: a line (accent, 1.6 px) over a translucent fill on `#0B0C0E` with
//! three guide lines, an optional "stable" band (cyan, from the first repeat to the end) and
//! an optional playhead (cyan, 1.5 px). Two mappings from the mockup:
//!
//! - [`Plot::sparkline`] (84 px): the last `capacity` values right-aligned in
//!   `capacity` slots; `y = h − 4 − v / max · (h − 12)`; fill closes at the bottom.
//! - [`Plot::series`] (120 px): values spread over the width;
//!   `y = h − 6 − v / max · (h − 16)`.
//!
//! `max` is `max(10, values…)` like the mockup. The height is the `<svg height>` (its content
//! box): the widget is 2 px taller for the svg's `border: 1px`, and the view box maps into
//! the content box inside the border.

use eframe::egui::{
    CornerRadius, Mesh, Painter, Pos2, Rect, Response, Sense, Stroke, StrokeKind, Ui, WidgetInfo,
    accesskit::Role, epaint::PathShape, pos2, vec2,
};
use sp_theme as t;

/// A population plot. `Plot::sparkline(&history, 120).show(ui)`.
#[derive(Debug, Clone)]
#[must_use = "call .show(ui) or .paint(painter, rect)"]
pub struct Plot<'a> {
    values: &'a [f32],
    capacity: Option<usize>,
    height: f32,
    bottom: f32,
    top: f32,
    band_from: Option<usize>,
    playhead: Option<usize>,
    label: &'a str,
}

impl<'a> Plot<'a> {
    /// A sparkline: the last `capacity` values, right-aligned, 84 px tall.
    pub fn sparkline(values: &'a [f32], capacity: usize) -> Self {
        Self {
            values,
            capacity: Some(capacity.max(2)),
            height: 84.0,
            bottom: 4.0,
            top: 8.0,
            band_from: None,
            playhead: None,
            label: "History",
        }
    }

    /// A plot: every value across the width, 120 px tall.
    pub fn series(values: &'a [f32]) -> Self {
        Self {
            values,
            capacity: None,
            height: 120.0,
            bottom: 6.0,
            top: 10.0,
            band_from: None,
            playhead: None,
            label: "Population across the run",
        }
    }

    /// Another content height (px, the `<svg height>`; the border adds 2).
    pub fn height(mut self, h: f32) -> Self {
        self.height = h;
        self
    }
    /// A shaded band from index `from` to the end.
    pub fn stable_from(mut self, from: Option<usize>) -> Self {
        self.band_from = from;
        self
    }
    /// The playhead at index `at`.
    pub fn playhead(mut self, at: Option<usize>) -> Self {
        self.playhead = at;
        self
    }
    /// The accessible name (`aria-label` of the mockup's `<svg role="img">`).
    pub fn label(mut self, label: &'a str) -> Self {
        self.label = label;
        self
    }

    /// The x of value `i` in the content box `rect`.
    pub fn x_of(&self, rect: Rect, i: usize) -> f32 {
        let w = rect.width();
        match self.capacity {
            Some(cap) => {
                let slots = (cap - 1) as f32;
                let len = self.values.len().max(1);
                rect.left() + (i as f32 / slots) * w + (w - ((len - 1) as f32 / slots) * w)
            }
            None => {
                let n = self.values.len().saturating_sub(1).max(1) as f32;
                rect.left() + i as f32 / n * w
            }
        }
    }

    /// The y of value `v` in the content box `rect`, over [`Plot::value_range`] (a non-finite
    /// value sits on the zero line).
    pub fn y_of(&self, rect: Rect, v: f32) -> f32 {
        let (lo, hi) = self.value_range();
        let v = if v.is_finite() { v } else { 0.0 };
        rect.bottom()
            - self.bottom
            - (v - lo) / (hi - lo) * (rect.height() - self.bottom - self.top)
    }

    /// The values' span: `[min(0, min), max(10, max)]` over the finite values, so a series that
    /// dips below zero (a compression force) stays inside the box.
    pub fn value_range(&self) -> (f32, f32) {
        let finite = self.values.iter().copied().filter(|v| v.is_finite());
        let (lo, hi) = finite.fold((0.0f32, 10.0f32), |(lo, hi), v| (lo.min(v), hi.max(v)));
        (lo, hi)
    }

    /// Draws the plot full width at the cursor. AccessKit: an `Image` named by the label.
    pub fn show(self, ui: &mut Ui) -> Response {
        let w = ui.available_width();
        let (rect, resp) = ui.allocate_exact_size(vec2(w, self.height + 2.0), Sense::hover());
        self.paint(ui.painter(), rect);
        let label = self.label.to_string();
        resp.widget_info(|| WidgetInfo::labeled(Role::Image, true, &label));
        resp
    }

    /// Paints the plot into `outer` (border box: the view box maps into it minus the border).
    pub fn paint(&self, painter: &Painter, outer: Rect) {
        let radius = CornerRadius::same(t::radius::WELL);
        painter.rect_filled(outer, radius, t::GRAPH.base);
        let rect = outer.shrink(1.0);
        let clip = painter.with_clip_rect(rect.intersect(painter.clip_rect()));
        for k in 1..=3 {
            // `M0 21H288…`: a 1 px stroke centred on the view-box line, as Chrome draws it
            let y = rect.top() + rect.height() * k as f32 / 4.0;
            clip.hline(rect.x_range(), y, Stroke::new(1.0, t::PLOT_GUIDE));
        }
        if let Some(from) = self.band_from
            && from < self.values.len().max(1)
        {
            let x = self.x_of(rect, from);
            clip.rect_filled(
                Rect::from_min_max(pos2(x, rect.top()), rect.max),
                0.0,
                t::PLOT_GUIDE,
            );
        }
        if self.values.len() > 1 {
            let pts: Vec<Pos2> = self
                .values
                .iter()
                .enumerate()
                .map(|(i, v)| pos2(self.x_of(rect, i), self.y_of(rect, *v)))
                .collect();
            // fill between the line and the zero line: one quad strip
            let zero = self.y_of(rect, 0.0);
            let mut mesh = Mesh::default();
            for (k, p) in pts.iter().enumerate() {
                mesh.colored_vertex(*p, t::PLOT_FILL);
                mesh.colored_vertex(pos2(p.x, zero), t::PLOT_FILL);
                if k > 0 {
                    let i = (2 * k) as u32;
                    mesh.add_triangle(i - 2, i - 1, i);
                    mesh.add_triangle(i - 1, i + 1, i);
                }
            }
            clip.add(mesh);
            clip.add(PathShape::line(pts, Stroke::new(1.6, t::ACCENT)));
        }
        if let Some(at) = self.playhead {
            let x = self.x_of(rect, at);
            clip.vline(x, rect.y_range(), Stroke::new(1.5, t::INFO));
        }
        painter.rect_stroke(
            outer,
            radius,
            Stroke::new(1.0, t::BORDER),
            StrokeKind::Inside,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The mockup's mapping: `X(i) = i/119·288 + (288 − (len−1)/119·288)`,
    /// `Y(v) = 80 − v/mx·72` in a 288 × 84 box.
    #[test]
    fn sparkline_matches_the_mockup() {
        let hist: Vec<f32> = (0..31).map(|i| 50.0 + i as f32).collect();
        let p = Plot::sparkline(&hist, 120);
        let r = Rect::from_min_size(pos2(0.0, 0.0), vec2(288.0, 84.0));
        let mx = 80.0;
        for (i, v) in hist.iter().enumerate() {
            let x = i as f32 / 119.0 * 288.0 + (288.0 - 30.0 / 119.0 * 288.0);
            assert!((p.x_of(r, i) - x).abs() < 1e-3);
            assert!((p.y_of(r, *v) - (80.0 - v / mx * 72.0)).abs() < 1e-3);
        }
        // the newest value is at the right edge
        assert!((p.x_of(r, 30) - 288.0).abs() < 1e-3);
    }

    /// `PX(t) = t/400·800`, `PY(v) = 114 − v/mx·104` in 800 × 120; mx ≥ 10.
    #[test]
    fn series_matches_the_mockup() {
        let pops: Vec<f32> = (0..=400).map(|t| (t % 7) as f32).collect();
        let p = Plot::series(&pops);
        let r = Rect::from_min_size(pos2(0.0, 0.0), vec2(800.0, 120.0));
        assert!((p.x_of(r, 120) - 240.0).abs() < 1e-3);
        assert!((p.y_of(r, 5.0) - (114.0 - 5.0 / 10.0 * 104.0)).abs() < 1e-3);
    }

    /// A series that dips below zero (the gallery's compression force) stays inside the box,
    /// non-finite values sit on the zero line, and an all-positive series maps as before.
    #[test]
    fn negative_values_stay_inside() {
        let v = [0.0, -400.0, -1004.0, -400.0, 0.0, f32::NAN];
        let p = Plot::series(&v);
        let r = Rect::from_min_size(pos2(0.0, 0.0), vec2(800.0, 120.0));
        assert_eq!(p.value_range(), (-1004.0, 10.0));
        for x in v {
            let y = p.y_of(r, x);
            assert!(y >= r.top() - 1e-3 && y <= r.bottom() + 1e-3, "{x} → {y}");
        }
        assert!(p.y_of(r, -1004.0) > p.y_of(r, 0.0), "down is negative");
        assert_eq!(p.y_of(r, f32::NAN), p.y_of(r, 0.0));
    }
}
