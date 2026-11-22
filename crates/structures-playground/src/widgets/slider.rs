//! `input[type=range]` in the accent, as Chrome draws it in the mockup: 28 px tall, an 8 px
//! track (radius 4) filled with the accent up to a 16 px accent thumb, the rest `#3B3B3B`
//! with a 1 px `#858585` border. Optionally with the mockup's label / value row above it
//! (`.lbl` "SPEED" left, "10 gen/s" in mono 12 right).

use super::text::lbl_row;
use eframe::egui::{
    CornerRadius, Key, Rect, Response, Sense, Stroke, StrokeKind, Ui, WidgetInfo, accesskit, pos2,
    vec2,
};
use lab_ui::paint::focus_ring_if;
use sp_theme as t;
use std::ops::RangeInclusive;

/// A range slider. AccessKit: a `Slider` named `label` with value, min, max, step and the
/// increment / decrement / set-value actions; arrow keys step it when focused.
///
/// ```ignore
/// Slider::new("Speed", &mut speed, 1.0..=30.0).step(1.0).value_text(format!("{speed} gen/s")).show(ui);
/// ```
#[must_use = "call .show(ui)"]
pub struct Slider<'a> {
    label: &'a str,
    value: &'a mut f32,
    range: RangeInclusive<f32>,
    step: f32,
    value_text: Option<String>,
    width: Option<f32>,
}

impl<'a> Slider<'a> {
    /// A slider for `value` within `range`, named `label`.
    pub fn new(label: &'a str, value: &'a mut f32, range: RangeInclusive<f32>) -> Self {
        Self {
            label,
            value,
            range,
            step: 0.0,
            value_text: None,
            width: None,
        }
    }

    /// Snap to multiples of `step` (0 = continuous).
    pub fn step(mut self, step: f32) -> Self {
        self.step = step;
        self
    }

    /// Show the label / value row above the slider with this value text ("10 gen/s").
    pub fn value_text(mut self, text: impl Into<String>) -> Self {
        self.value_text = Some(text.into());
        self
    }

    /// A fixed width (default: the available width).
    pub fn width(mut self, w: f32) -> Self {
        self.width = Some(w);
        self
    }

    /// Draws it; `.changed()` when the value moved.
    pub fn show(self, ui: &mut Ui) -> Response {
        if let Some(v) = &self.value_text {
            lbl_row(ui, self.label, v, lab_theme::font::mono(12.0), t::TEXT_DIM);
            ui.add_space(10.0 - ui.spacing().item_spacing.y);
        }
        let w = self.width.unwrap_or_else(|| ui.available_width());
        let (id, rect) = ui.allocate_space(vec2(w, 28.0));
        // 28 px drawn, 44 px target
        let mut resp = ui.interact(rect.expand2(vec2(0.0, 8.0)), id, Sense::click_and_drag());
        super::reveal_on_focus(&resp);
        let (lo, hi) = (*self.range.start(), *self.range.end());
        let thumb_r = 8.0;
        let x0 = rect.left() + thumb_r;
        let x1 = rect.right() - thumb_r;
        let step = self.step;
        let snap = |v: f32| {
            let v = v.clamp(lo, hi);
            if step > 0.0 {
                (lo + ((v - lo) / step).round() * step).clamp(lo, hi)
            } else {
                v
            }
        };
        let before = *self.value;
        if let Some(p) = resp.interact_pointer_pos()
            && (resp.dragged() || resp.clicked() || resp.is_pointer_button_down_on())
            && x1 > x0
        {
            let f = ((p.x - x0) / (x1 - x0)).clamp(0.0, 1.0);
            *self.value = snap(lo + f * (hi - lo));
        }
        let key_step = if step > 0.0 { step } else { (hi - lo) / 100.0 };
        if resp.has_focus() {
            ui.memory_mut(|m| {
                m.set_focus_lock_filter(
                    id,
                    eframe::egui::EventFilter {
                        horizontal_arrows: true,
                        vertical_arrows: true,
                        ..Default::default()
                    },
                )
            });
            let (dec, inc) = ui.input(|i| {
                (
                    i.key_pressed(Key::ArrowLeft) || i.key_pressed(Key::ArrowDown),
                    i.key_pressed(Key::ArrowRight) || i.key_pressed(Key::ArrowUp),
                )
            });
            if dec {
                *self.value = snap(*self.value - key_step);
            }
            if inc {
                *self.value = snap(*self.value + key_step);
            }
        }
        if let Some(v) = ui.input(|i| {
            i.events.iter().find_map(|e| match e {
                eframe::egui::Event::AccessKitActionRequest(req)
                    if req.target_node == id.accesskit_id() =>
                {
                    match (&req.action, &req.data) {
                        (
                            accesskit::Action::SetValue,
                            Some(accesskit::ActionData::NumericValue(v)),
                        ) => Some(*v as f32),
                        (accesskit::Action::Increment, _) => Some(f32::INFINITY),
                        (accesskit::Action::Decrement, _) => Some(f32::NEG_INFINITY),
                        _ => None,
                    }
                }
                _ => None,
            })
        }) {
            *self.value = if v == f32::INFINITY {
                snap(*self.value + key_step)
            } else if v == f32::NEG_INFINITY {
                snap(*self.value - key_step)
            } else {
                snap(v)
            };
        }
        if *self.value != before {
            resp.mark_changed();
        }
        let value = *self.value;
        let label = self.label.to_string();
        resp.widget_info(|| WidgetInfo::slider(true, value as f64, &label));
        ui.ctx().accesskit_node_builder(id, |b| {
            b.set_min_numeric_value(lo as f64);
            b.set_max_numeric_value(hi as f64);
            if step > 0.0 {
                b.set_numeric_value_step(step as f64);
            }
            b.add_action(accesskit::Action::Increment);
            b.add_action(accesskit::Action::Decrement);
            b.add_action(accesskit::Action::SetValue);
        });
        paint_range(
            ui,
            rect,
            if hi > lo {
                (value - lo) / (hi - lo)
            } else {
                0.0
            },
        );
        focus_ring_if(ui, &resp, rect, CornerRadius::same(t::radius::BUTTON));
        resp.on_hover_cursor(eframe::egui::CursorIcon::PointingHand)
    }
}

/// Paints the Chrome-style range at `frac` (0–1) inside the 28 px `rect`.
pub fn paint_range(ui: &Ui, rect: Rect, frac: f32) {
    let painter = ui.painter();
    let cy = rect.center().y;
    let thumb_r = 8.0;
    let tx = rect.left() + thumb_r + frac.clamp(0.0, 1.0) * (rect.width() - 2.0 * thumb_r);
    let track = Rect::from_min_max(pos2(rect.left(), cy - 4.0), pos2(rect.right(), cy + 4.0));
    painter.rect(
        track,
        CornerRadius::same(4),
        t::RANGE_TRACK,
        Stroke::new(1.0, t::RANGE_BORDER),
        StrokeKind::Inside,
    );
    let filled = Rect::from_min_max(track.min, pos2(tx.max(track.left() + 8.0), track.bottom()));
    painter.rect_filled(filled, CornerRadius::same(4), t::ACCENT);
    painter.circle_filled(pos2(tx, cy), thumb_r, t::ACCENT);
}
