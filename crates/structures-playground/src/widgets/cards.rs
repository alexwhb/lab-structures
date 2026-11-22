//! Frames and readouts: `.panel` / `.well` cards, stat tiles, "What's happening", the pinned
//! measurement `.chip` and mono code blocks.

use super::text::{css_line_height, galley, paint_line, text_top, wrapped, wrapped_job};
use eframe::egui::{
    Align2, Color32, CornerRadius, Frame, InnerResponse, Margin, Pos2, Rect, Response, Sense,
    Stroke, StrokeKind, Ui, UiBuilder, WidgetInfo, accesskit::Role, epaint::text::LayoutJob, pos2,
    vec2,
};
use sp_theme as t;

/// A bordered, rounded box laying its children out top-down with a gap.
///
/// - [`Card::panel`]: the `.panel` (`rgba(17,17,17,.88)`, `1px solid #242424`, radius 12; padding
///   20, gap 14 as the Series sections)
/// - [`Card::well`]: the `.well` (`#0D0D0D`, `1px solid #242424`, radius 8; padding 12, gap 6)
#[derive(Debug, Clone, Copy)]
#[must_use = "call .show(ui, …)"]
pub struct Card {
    fill: Color32,
    border: Color32,
    border_w: f32,
    padding: f32,
    gap: f32,
    radius: u8,
    label: Option<&'static str>,
}

impl Card {
    /// `.panel`.
    pub fn panel() -> Self {
        Self {
            fill: t::PANEL,
            border: t::BORDER,
            border_w: 1.0,
            padding: 20.0,
            gap: 14.0,
            radius: t::radius::PANEL,
            label: None,
        }
    }

    /// `.well`.
    pub fn well() -> Self {
        Self {
            fill: t::WELL,
            border: t::BORDER,
            border_w: 1.0,
            padding: 12.0,
            gap: 6.0,
            radius: t::radius::WELL,
            label: None,
        }
    }

    /// Another corner radius.
    pub fn radius(mut self, r: u8) -> Self {
        self.radius = r;
        self
    }

    /// Another fill.
    pub fn fill(mut self, fill: Color32) -> Self {
        self.fill = fill;
        self
    }
    /// Another border colour (the identity cards use their edge colour).
    pub fn border(mut self, border: Color32) -> Self {
        self.border = border;
        self
    }
    /// Another border width (a highlighted card uses 2).
    pub fn border_width(mut self, w: f32) -> Self {
        self.border_w = w;
        self
    }
    /// Another padding (all sides).
    pub fn padding(mut self, p: f32) -> Self {
        self.padding = p;
        self
    }
    /// Another gap between children.
    pub fn gap(mut self, g: f32) -> Self {
        self.gap = g;
        self
    }
    /// Make the card an AccessKit `Group` with this name (`<section aria-label>`).
    pub fn label(mut self, label: &'static str) -> Self {
        self.label = Some(label);
        self
    }

    /// Draws the card filling the available width; `add` lays out its children.
    pub fn show<R>(self, ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
        // egui draws the stroke outside the inner margin (like a CSS border around padding);
        // a thicker border eats into the padding so the card keeps its size
        let pad = self.padding - (self.border_w - 1.0).max(0.0);
        let inner = |ui: &mut Ui| {
            Frame::new()
                .fill(self.fill)
                .stroke(Stroke::new(self.border_w, self.border))
                .corner_radius(CornerRadius::same(self.radius))
                .inner_margin(Margin::same(pad.round() as i8))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.spacing_mut().item_spacing = vec2(self.gap, self.gap);
                    add(ui)
                })
        };
        match self.label {
            Some(label) => {
                let rect = ui.available_rect_before_wrap();
                let mut child = ui.new_child(
                    UiBuilder::new()
                        .max_rect(rect)
                        .accessibility_role(Role::Group)
                        .accessibility_label(label),
                );
                let r = inner(&mut child);
                let used = child.min_rect();
                super::set_bounds(&child, used);
                ui.advance_cursor_after_rect(used);
                InnerResponse::new(r.inner, r.response)
            }
            None => {
                let r = inner(ui);
                InnerResponse::new(r.inner, r.response)
            }
        }
    }
}

/// A stat tile (a `.well`): a caption (11 px muted) over a mono value in a colour,
/// `padding: 8px 10px` with 18 px values; [`StatTile::compact`] is `9px 10px` with 16 px. AccessKit: a `Label` "<label>: <value>".
#[derive(Debug, Clone, Copy)]
#[must_use = "call .show(ui)"]
pub struct StatTile<'a> {
    label: &'a str,
    value: &'a str,
    color: Color32,
    compact: bool,
}

impl<'a> StatTile<'a> {
    /// A tile captioned `label` showing `value` in [`TEXT`](sp_theme::TEXT).
    pub fn new(label: &'a str, value: &'a str) -> Self {
        Self {
            label,
            value,
            color: t::TEXT,
            compact: false,
        }
    }
    /// The value's colour (status colours, force colours).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = c;
        self
    }
    /// The compact variant (`padding: 9px 10px`, 16 px value).
    pub fn compact(mut self) -> Self {
        self.compact = true;
        self
    }

    /// Draws it full width.
    pub fn show(self, ui: &mut Ui) -> Response {
        let width = ui.available_width();
        let (pad_y, gap, size) = if self.compact {
            (9.0, 3.0, 16.0)
        } else {
            (8.0, 2.0, 18.0)
        };
        let label_font = lab_theme::font::body(11.0);
        let value_font = lab_theme::font::mono_medium(size);
        let lg = galley(ui, self.label, label_font.clone(), t::TEXT_MUTED);
        let vg = galley(ui, self.value, value_font.clone(), self.color);
        // each span is a CSS `line-height: normal` box
        let (lh, vh) = (
            css_line_height(ui, &label_font),
            css_line_height(ui, &value_font),
        );
        let h = 2.0 + 2.0 * pad_y + lh + gap + vh;
        let (rect, resp) = ui.allocate_exact_size(vec2(width, h), Sense::hover());
        let painter = ui.painter();
        painter.rect(
            rect,
            CornerRadius::same(t::radius::WELL),
            t::WELL,
            Stroke::new(1.0, t::BORDER),
            StrokeKind::Inside,
        );
        let x = rect.left() + 11.0;
        let y = rect.top() + 1.0 + pad_y;
        paint_line(painter, &lg, x, y, lh, t::TEXT_MUTED);
        paint_line(painter, &vg, x, y + lh + gap, vh, self.color);
        let s = format!("{}: {}", self.label, self.value);
        resp.widget_info(|| WidgetInfo::labeled(Role::Label, true, &s));
        resp
    }
}

/// "What's happening" (every Playground aside): `<section style="display: flex;
/// flex-direction: column; gap: 6px">` with the `.lbl` title over the sentence (13 px, line
/// height 1.55, `#D4D4D4`). AccessKit: a `Group` named by the title with the text as a label.
#[derive(Debug, Clone, Copy)]
#[must_use = "call .show(ui)"]
pub struct Callout<'a> {
    title: &'a str,
    text: &'a str,
}

impl<'a> Callout<'a> {
    /// A section titled `title` (the mockup's "What’s happening").
    pub fn new(title: &'a str, text: &'a str) -> Self {
        Self { title, text }
    }

    /// "What’s happening" with `text`.
    pub fn happening(text: &'a str) -> Self {
        Self::new("What’s happening", text)
    }

    /// Draws it full width.
    pub fn show(self, ui: &mut Ui) -> Response {
        let width = ui.available_width();
        let lg = super::text::lbl_galley(ui, self.title, t::TEXT_MUTED);
        let font = lab_theme::font::body(13.0);
        let xg = wrapped(ui, self.text, font, t::TEXT_BODY, width, Some(13.0 * 1.55));
        let h = lg.size().y + 6.0 + xg.size().y;
        let origin = ui.cursor().min;
        let rect = Rect::from_min_size(origin, vec2(width, h));
        let child = ui.new_child(
            UiBuilder::new()
                .max_rect(rect)
                .accessibility_role(Role::Group)
                .accessibility_label(self.title),
        );
        super::set_bounds(&child, rect);
        let painter = child.painter();
        painter.galley(rect.min, lg.clone(), t::TEXT_MUTED);
        let text_y = rect.top() + lg.size().y + 6.0;
        painter.galley(pos2(rect.left(), text_y), xg, t::TEXT_BODY);
        let text = self.text.to_string();
        let xr = child.interact(
            Rect::from_min_max(pos2(rect.left(), text_y), rect.max),
            child.scope_id().with("x"),
            Sense::hover(),
        );
        xr.widget_info(|| WidgetInfo::labeled(Role::Label, true, &text));
        let resp = child.response();
        ui.advance_cursor_after_rect(rect);
        resp
    }
}

/// `.chip` — a measurement chip pinned to the stage: `padding: 4px 9px`, radius 6,
/// `rgba(11,12,14,.9)`, `1px solid #2A2D33`, JetBrains Mono 12 px `#ECEAE6`; parts may have
/// their own colour ("top 928 mm · " + accent "H/259").
#[derive(Debug, Clone, Default)]
#[must_use = "call .show_at(…)"]
pub struct Chip {
    parts: Vec<(String, Color32)>,
}

impl Chip {
    /// A chip with one part in the default chip text colour.
    pub fn new(text: impl Into<String>) -> Self {
        Self::default().part(text, t::CHIP_TEXT)
    }

    /// Appends a coloured part.
    pub fn part(mut self, text: impl Into<String>, color: Color32) -> Self {
        self.parts.push((text.into(), color));
        self
    }

    /// The plain text of all parts.
    pub fn text(&self) -> String {
        self.parts.iter().map(|(s, _)| s.as_str()).collect()
    }

    /// The chip's size.
    pub fn size(&self, ui: &Ui) -> eframe::egui::Vec2 {
        self.galley(ui).size() + vec2(18.0 + 2.0, 8.0 + 2.0)
    }

    fn galley(&self, ui: &Ui) -> std::sync::Arc<eframe::egui::Galley> {
        let mut job = LayoutJob::default();
        for (s, c) in &self.parts {
            job.append(
                s,
                0.0,
                eframe::egui::TextFormat::simple(lab_theme::font::mono(12.0), *c),
            );
        }
        ui.painter().layout_job(job)
    }

    /// Paints the chip with its `anchor` corner at `pos` (e.g. `Align2::RIGHT_TOP` at the
    /// stage's top-right minus 16 / 14). AccessKit: a `Label` with the text.
    pub fn show_at(self, ui: &Ui, pos: Pos2, anchor: Align2) -> Response {
        let g = self.galley(ui);
        let size = g.size() + vec2(20.0, 10.0);
        let rect = anchor.anchor_size(pos, size);
        let painter = ui.painter();
        painter.rect(
            rect,
            CornerRadius::same(6),
            t::CHIP,
            Stroke::new(1.0, t::CHIP_BORDER),
            StrokeKind::Inside,
        );
        painter.galley(
            pos2(rect.left() + 10.0, text_top(rect.center().y, &g)),
            g,
            t::CHIP_TEXT,
        );
        let text = self.text();
        let resp = ui.interact(rect, ui.scope_id().with(("chip", &text)), Sense::hover());
        resp.widget_info(|| WidgetInfo::labeled(Role::Label, true, &text));
        resp
    }
}

/// `.code` — JetBrains Mono 12 px / 1.65 `#D9D9D9`, `white-space: pre` (or `pre-wrap;
/// word-break: break-all` with [`CodeBlock::wrap`]). Put it in `Card::panel().fill(INSET)`.
#[derive(Debug, Clone, Copy)]
#[must_use = "call .show(ui)"]
pub struct CodeBlock<'a> {
    text: &'a str,
    wrap: bool,
    color: Color32,
    label: Option<&'a str>,
}

impl<'a> CodeBlock<'a> {
    /// A code block.
    pub fn new(text: &'a str) -> Self {
        Self {
            text,
            wrap: false,
            color: t::TEXT_DIM,
            label: None,
        }
    }
    /// Wrap anywhere (the RLE export).
    pub fn wrap(mut self) -> Self {
        self.wrap = true;
        self
    }
    /// Another text colour (the RLE is muted).
    pub fn color(mut self, c: Color32) -> Self {
        self.color = c;
        self
    }
    /// The accessible name (default: the text itself).
    pub fn label(mut self, l: &'a str) -> Self {
        self.label = Some(l);
        self
    }

    /// Draws it (unwrapped blocks are clipped to the available width; scroll them with an
    /// `egui::ScrollArea::horizontal` if needed).
    pub fn show(self, ui: &mut Ui) -> Response {
        let width = ui.available_width();
        let font = lab_theme::font::mono(12.0);
        let g = if self.wrap {
            wrapped_job(
                ui,
                self.text,
                font,
                self.color,
                width,
                Some(19.8),
                0.0,
                true,
            )
        } else {
            wrapped_job(
                ui,
                self.text,
                font,
                self.color,
                f32::INFINITY,
                Some(19.8),
                0.0,
                false,
            )
        };
        let (rect, resp) = ui.allocate_exact_size(vec2(width, g.size().y), Sense::hover());
        ui.painter()
            .with_clip_rect(rect.intersect(ui.clip_rect()))
            .galley(rect.min, g, self.color);
        let s = self.label.unwrap_or(self.text).to_string();
        resp.widget_info(|| WidgetInfo::labeled(Role::Label, true, &s));
        resp
    }
}
