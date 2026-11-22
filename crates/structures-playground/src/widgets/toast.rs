//! Toasts: short confirmations ("Copied river-level to the clipboard", "Breadbox isn't
//! installed on this Mac") shown bottom-centre for 2.6 s (BR-4). Time is egui's input time, so
//! tests stepping the harness see them expire deterministically.
//!
//! Drawn like the Playground stage's toast: `padding: 6px 12px; border-radius: 8px;
//! background: rgba(10,10,10,.92); border: 1px solid #3A3A3A; font-size: 13px; color:
//! #EAEAEA`. AccessKit: a `Status` node (a polite live region) named by the text.

use eframe::egui::{
    self, Align2, CornerRadius, Id, Rect, Sense, Stroke, StrokeKind, WidgetInfo, accesskit::Role,
    pos2, vec2,
};
use sp_theme as t;

/// How long a toast stays (seconds of egui time).
pub const TOAST_SECS: f64 = 2.6;
/// `padding: 6px 12px`.
const PAD_X: f32 = 12.0;
const PAD_Y: f32 = 6.0;

/// One toast.
#[derive(Debug, Clone, PartialEq)]
pub struct Toast {
    /// The message.
    pub text: String,
    /// When it was pushed (egui time), `None` until first shown.
    pub since: Option<f64>,
}

/// The toast queue (newest last).
#[derive(Debug, Clone, Default)]
pub struct Toasts {
    items: Vec<Toast>,
}

impl Toasts {
    /// Queues a toast. Pushing the text already showing restarts its timer instead of adding
    /// a duplicate.
    pub fn push(&mut self, text: impl Into<String>) {
        let text = text.into();
        self.items.retain(|t| t.text != text);
        self.items.push(Toast { text, since: None });
        if self.items.len() > 3 {
            self.items.remove(0);
        }
    }

    /// The toasts currently queued or showing.
    pub fn items(&self) -> &[Toast] {
        &self.items
    }

    /// Removes every toast.
    pub fn clear(&mut self) {
        self.items.clear();
    }

    /// Draws the toasts over everything, bottom-centre, and expires old ones.
    pub fn show(&mut self, ctx: &egui::Context) {
        let now = ctx.input(|i| i.time);
        for t in &mut self.items {
            t.since.get_or_insert(now);
        }
        self.items
            .retain(|t| now - t.since.unwrap_or(now) < TOAST_SECS);
        if self.items.is_empty() {
            return;
        }
        let screen = ctx.content_rect();
        let mut bottom = screen.bottom() - 16.0;
        let mut dismissed = None;
        for (i, toast) in self.items.iter().enumerate().rev() {
            let id = Id::unique("sp-toast").with(i);
            let area = egui::Area::new(id)
                .order(egui::Order::Foreground)
                .fixed_pos(pos2(screen.center().x, bottom))
                .pivot(Align2::CENTER_BOTTOM)
                .interactable(true);
            let shown = area.show(ctx, |ui| {
                let font = lab_theme::font::body(13.0);
                let max_w = (screen.width() - 32.0).min(420.0) - 2.0 * PAD_X - 2.0;
                let g = super::text::wrapped(ui, &toast.text, font, t::TEXT, max_w, None);
                let size = vec2(
                    g.size().x + 2.0 * PAD_X + 2.0,
                    g.size().y + 2.0 * PAD_Y + 2.0,
                );
                let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
                let painter = ui.painter();
                painter.rect(
                    rect,
                    CornerRadius::same(t::radius::WELL),
                    t::OVERLAY,
                    Stroke::new(1.0, t::BORDER_OVERLAY),
                    StrokeKind::Inside,
                );
                painter.galley(rect.min + vec2(PAD_X + 1.0, PAD_Y + 1.0), g, t::TEXT);
                let text = toast.text.clone();
                resp.widget_info(|| WidgetInfo::labeled(Role::Status, true, &text));
                if resp.clicked() {
                    dismissed = Some(i);
                }
                rect
            });
            let r: Rect = shown.inner;
            bottom -= r.height() + 8.0;
        }
        if let Some(i) = dismissed {
            self.items.remove(i);
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(250));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_dedupes_and_caps() {
        let mut t = Toasts::default();
        t.push("a");
        t.push("b");
        t.push("a");
        assert_eq!(
            t.items()
                .iter()
                .map(|t| t.text.as_str())
                .collect::<Vec<_>>(),
            ["b", "a"]
        );
        for s in ["c", "d", "e"] {
            t.push(s);
        }
        assert_eq!(t.items().len(), 3);
        assert_eq!(t.items()[2].text, "e");
    }
}
