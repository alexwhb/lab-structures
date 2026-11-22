//! The Advanced screen's own small controls, drawn to the mockup's CSS: `.panel` sections with
//! their header row, the status `.pill`, Chrome's `<select>`, the toolbar's inline
//! range (`label · range · output`) and a `.seg` well that wraps (the influence-line picks).
//! Colours from `sp_theme` / `sp_theme::advanced`.

use crate::widgets::menu::{Menu, MenuItem, menu_button_state};
use crate::widgets::text::{galley, text_top};
use crate::widgets::{Slider, hit, interact};
use eframe::egui::{
    self, Align, Color32, CornerRadius, Id, Key, Layout, Rect, Response, Sense, Shape, Stroke,
    StrokeKind, Ui, UiBuilder, Vec2, WidgetInfo, accesskit::Role, pos2, vec2,
};
use lab_theme::font;
use lab_ui::paint::focus_ring_if;
use sp_theme::{self as t, advanced as a};

/// `.panel` header padding `10px 14px`.
pub const HEAD_PAD: Vec2 = vec2(14.0, 10.0);
/// The `h2.h`: Space Grotesk 600 15, `letter-spacing: -.01em`.
pub fn h2_font() -> egui::FontId {
    font::heading(15.0)
}

/// A `.panel` section across the available width at the cursor: `rgba(17,17,17,.88)`,
/// `1px solid #242424`, radius 12, clipped (`overflow: hidden`). An AccessKit `Region` named
/// `label`. `add` lays out the content top-down with no spacing. Returns the panel's rect.
pub fn panel<R>(ui: &mut Ui, label: &str, add: impl FnOnce(&mut Ui) -> R) -> (Rect, R) {
    let width = ui.available_width();
    let origin = ui.cursor().min;
    // the content sits inside the 1 px border (CSS border-box)
    let max = Rect::from_min_size(origin + vec2(1.0, 1.0), vec2(width - 2.0, f32::INFINITY));
    let mut child = ui.new_child(
        UiBuilder::new()
            .id_salt(("panel", label))
            .max_rect(max)
            .layout(Layout::top_down(Align::Min))
            .accessibility_role(Role::Region)
            .accessibility_label(label),
    );
    child.spacing_mut().item_spacing = Vec2::ZERO;
    let bg = child.painter().add(Shape::Noop);
    let r = add(&mut child);
    let used = Rect::from_min_max(
        origin,
        pos2(
            origin.x + width,
            child.min_rect().bottom().max(origin.y + 1.0) + 1.0,
        ),
    );
    child.painter().set(
        bg,
        Shape::rect_filled(used, CornerRadius::same(t::radius::PANEL), t::PANEL),
    );
    ui.painter().rect_stroke(
        used,
        CornerRadius::same(t::radius::PANEL),
        Stroke::new(1.0, t::BORDER),
        StrokeKind::Inside,
    );
    crate::widgets::set_bounds(&child, used);
    ui.advance_cursor_after_rect(used);
    (used, r)
}

/// A panel's header row: the `h2` on the left, `right` (of `right_size`) on the right
/// (`justify-content: space-between`), wrapping under the title when they don't fit
/// (`flex-wrap: wrap; gap: 8px`), then the 1 px `border-bottom` (part of its height). `right`
/// draws into the rect given.
pub fn panel_header(
    ui: &mut Ui,
    title: &str,
    right_size: Vec2,
    right: impl FnOnce(&mut Ui, Rect),
) -> Rect {
    let width = ui.available_width();
    let origin = ui.cursor().min;
    let inner_w = width - 2.0 * HEAD_PAD.x;
    let g = galley(ui, title, h2_font(), t::TEXT);
    let title_h = 19.0f32.max(g.size().y);
    let one_row = g.size().x + 8.0 + right_size.x <= inner_w;
    let (h, title_c, right_rect) = if one_row {
        let h = title_h.max(right_size.y) + 2.0 * HEAD_PAD.y + 1.0;
        let cy = origin.y + (h - 1.0) / 2.0;
        let rr = Rect::from_min_size(
            pos2(
                origin.x + width - HEAD_PAD.x - right_size.x,
                cy - right_size.y / 2.0,
            ),
            right_size,
        );
        (h, cy, rr)
    } else {
        let h = title_h + 8.0 + right_size.y + 2.0 * HEAD_PAD.y + 1.0;
        let cy = origin.y + HEAD_PAD.y + title_h / 2.0;
        let rr = Rect::from_min_size(
            pos2(origin.x + HEAD_PAD.x, origin.y + HEAD_PAD.y + title_h + 8.0),
            right_size,
        );
        (h, cy, rr)
    };
    let rect = Rect::from_min_size(origin, vec2(width, h));
    let tpos = pos2(origin.x + HEAD_PAD.x, text_top(title_c, &g));
    ui.painter().galley(tpos, g.clone(), t::TEXT);
    let title_s = title.to_string();
    ui.interact(
        Rect::from_min_size(tpos, g.size()),
        ui.scope_id().with(("h2", title)),
        Sense::hover(),
    )
    .widget_info(|| WidgetInfo::labeled(Role::Heading, true, &title_s));
    right(ui, right_rect);
    ui.painter().hline(
        rect.x_range(),
        rect.bottom() - 0.5,
        Stroke::new(1.0, t::BORDER),
    );
    ui.advance_cursor_after_rect(rect);
    rect
}

/// `.pill` size for `label`: `padding: 4px 10px; gap: 6px`, an 8 px dot, Inter 600 12.
pub fn pill_size(ui: &Ui, label: &str) -> Vec2 {
    let g = galley(ui, label, font::semibold(12.0), t::TEXT);
    vec2(2.0 * 11.0 + 8.0 + 6.0 + g.size().x, 2.0 * 5.0 + 15.0)
}

/// Paints a `.pill` (`border-radius: 999px; border: 1px solid #2E2E2E; background: #141414`)
/// with its dot and label in `color`. AccessKit: a `Label`.
pub fn pill(ui: &Ui, rect: Rect, label: &str, color: Color32) -> Response {
    let p = ui.painter();
    p.rect(
        rect,
        CornerRadius::same(t::radius::PILL),
        t::CONTROL,
        Stroke::new(1.0, t::BORDER_STRONG),
        StrokeKind::Inside,
    );
    let cy = rect.center().y;
    p.circle_filled(pos2(rect.left() + 11.0 + 4.0, cy), 4.0, color);
    let g = galley(ui, label, font::semibold(12.0), color);
    p.galley(pos2(rect.left() + 11.0 + 14.0, text_top(cy, &g)), g, color);
    let s = label.to_string();
    let r = ui.interact(rect, ui.scope_id().with(("pill", label)), Sense::hover());
    r.widget_info(|| WidgetInfo::labeled(Role::Label, true, &s));
    r
}

/// What [`select`] did.
pub struct SelectResponse {
    /// The option picked this frame.
    pub picked: Option<usize>,
    /// The select's rect.
    pub rect: Rect,
}

/// The `<select>` (36 px, `padding: 0 8px`, `border: 1px solid #2E2E2E`, radius 6, `#121212`,
/// 13 px) with Chrome's chevron, `width` wide at the cursor. Click (or Enter / Space) opens a
/// menu of `options`; ↑ / ↓ step through them while it is focused. AccessKit: a `ComboBox`
/// named `name` whose value is the selected option.
pub fn select(
    ui: &mut Ui,
    name: &str,
    options: &[&str],
    selected: usize,
    width: f32,
    open: &mut bool,
) -> SelectResponse {
    let (rect, _) = ui.allocate_exact_size(vec2(width, 36.0), Sense::hover());
    let id = ui.scope_id().with(("select", name));
    let resp = interact(ui, id, hit::target(rect), true, Role::ComboBox, name, None);
    let current = options.get(selected).copied().unwrap_or("");
    let value = current.to_string();
    ui.ctx()
        .accesskit_node_builder(id, |b| b.set_value(value.clone()));
    menu_button_state(ui, &resp, *open);
    let p = ui.painter();
    p.rect(
        rect,
        CornerRadius::same(t::radius::BUTTON),
        a::SELECT,
        Stroke::new(1.0, a::SELECT_BORDER),
        StrokeKind::Inside,
    );
    let g = galley(ui, current, font::body(13.0), t::TEXT);
    p.galley(
        pos2(rect.left() + 9.0, text_top(rect.center().y, &g)),
        g,
        t::TEXT,
    );
    let cx = rect.right() - 13.0;
    let cy = rect.center().y;
    p.add(Shape::line(
        vec![
            pos2(cx - 4.0, cy - 2.0),
            pos2(cx, cy + 2.0),
            pos2(cx + 4.0, cy - 2.0),
        ],
        Stroke::new(1.5, a::SELECT_CHEVRON),
    ));
    focus_ring_if(ui, &resp, rect, CornerRadius::same(t::radius::BUTTON));
    let mut picked = None;
    let keyboard_open =
        resp.has_focus() && ui.input(|i| i.key_pressed(Key::Enter) || i.key_pressed(Key::Space));
    if resp.clicked() || keyboard_open {
        *open = !*open;
    }
    if resp.has_focus() && !*open && !options.is_empty() {
        let (up, down) = ui.input(|i| (i.key_pressed(Key::ArrowUp), i.key_pressed(Key::ArrowDown)));
        if up && selected > 0 {
            picked = Some(selected - 1);
        }
        if down && selected + 1 < options.len() {
            picked = Some(selected + 1);
        }
    }
    if *open {
        let items: Vec<MenuItem> = options.iter().map(|o| MenuItem::new(o)).collect();
        let m = Menu::new(id.with("menu"), name, rect, &items)
            .focus_first(keyboard_open)
            .show(ui.ctx());
        if let Some(i) = m.clicked {
            picked = Some(i);
        }
        if m.close || m.clicked.is_some() {
            *open = false;
        }
    }
    SelectResponse { picked, rect }
}

/// The toolbar's inline range: `<label>` (13 px) · `input[type=range]` · `<output>` (mono 12,
/// accent, `min-width`), gap 8, across `rect`. Returns the slider's response.
#[allow(clippy::too_many_arguments)]
pub fn inline_slider(
    ui: &mut Ui,
    rect: Rect,
    label: &str,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    step: f32,
    output: &str,
    output_min: f32,
    enabled: bool,
) -> Response {
    let cy = rect.center().y;
    let lg = galley(ui, label, font::body(13.0), t::TEXT);
    ui.painter()
        .galley(pos2(rect.left(), text_top(cy, &lg)), lg.clone(), t::TEXT);
    let og = galley(ui, output, font::mono(12.0), a::OUTPUT);
    let out_w = og.size().x.max(output_min);
    let x0 = rect.left() + lg.size().x + 8.0;
    let x1 = rect.right() - out_w - 8.0;
    let srect = Rect::from_min_max(pos2(x0, cy - 14.0), pos2(x1.max(x0 + 20.0), cy + 14.0));
    let resp = ui
        .scope_builder(
            UiBuilder::new()
                .id_salt(("inline-slider", label))
                .max_rect(srect)
                .layout(Layout::top_down(Align::Min)),
            |ui| {
                if !enabled {
                    ui.disable();
                }
                Slider::new(label, value, range)
                    .step(step)
                    .width(srect.width())
                    .show(ui)
            },
        )
        .inner;
    ui.painter()
        .galley(pos2(srect.right() + 8.0, text_top(cy, &og)), og, a::OUTPUT);
    resp
}

/// A `.seg` group in its well (`padding: 3px; gap: 2px`, 36 px items, Inter 500 13) whose
/// items wrap onto more rows when they don't fit `max_w` (`flex-wrap: wrap`).
pub struct SegWrap<'a> {
    name: &'a str,
    items: &'a [String],
    selected: Option<usize>,
}

impl<'a> SegWrap<'a> {
    /// A group named `name`.
    pub fn new(name: &'a str, items: &'a [String], selected: Option<usize>) -> Self {
        Self {
            name,
            items,
            selected,
        }
    }

    fn widths(&self, ui: &Ui) -> Vec<f32> {
        self.items
            .iter()
            .map(|s| galley(ui, s.as_str(), font::medium(13.0), t::TEXT).size().x + 28.0)
            .collect()
    }

    fn layout(&self, ui: &Ui, max_w: f32) -> (Vec<Rect>, Vec2) {
        let ws = self.widths(ui);
        let pad = 4.0; // 3 px padding + 1 px border
        let inner = (max_w - 2.0 * pad).max(0.0);
        let (mut x, mut y, mut maxx) = (0.0f32, 0.0f32, 0.0f32);
        let mut rects = Vec::with_capacity(ws.len());
        for w in ws {
            if x > 0.0 && x + w > inner + 0.01 {
                x = 0.0;
                y += 36.0 + 2.0;
            }
            rects.push(Rect::from_min_size(pos2(pad + x, pad + y), vec2(w, 36.0)));
            x += w + 2.0;
            maxx = maxx.max(x - 2.0);
        }
        (rects, vec2(maxx + 2.0 * pad, y + 36.0 + 2.0 * pad))
    }

    /// The group's size within `max_w`.
    pub fn size(&self, ui: &Ui, max_w: f32) -> Vec2 {
        self.layout(ui, max_w).1
    }

    /// Draws it with its top-left at `origin`; returns the item clicked.
    pub fn show_at(self, ui: &mut Ui, origin: egui::Pos2, max_w: f32) -> Option<usize> {
        let (rel, size) = self.layout(ui, max_w);
        let well = Rect::from_min_size(origin, size);
        let group = ui.new_child(
            UiBuilder::new()
                .id_salt(("segwrap", self.name))
                .max_rect(well)
                .accessibility_role(Role::Group)
                .accessibility_label(self.name),
        );
        crate::widgets::set_bounds(&group, well);
        group.painter().rect(
            well,
            CornerRadius::same(t::radius::WELL),
            t::INSET_BAR,
            Stroke::new(1.0, t::BORDER),
            StrokeKind::Inside,
        );
        let visuals: Vec<Rect> = rel.iter().map(|r| r.translate(origin.to_vec2())).collect();
        let targets = hit::tile(&visuals);
        let mut clicked = None;
        for (i, label) in self.items.iter().enumerate() {
            let r = visuals[i];
            let on = self.selected == Some(i);
            let resp = interact(
                &group,
                group.scope_id().with(i),
                targets[i],
                true,
                Role::Button,
                label,
                Some(on),
            );
            let fg = if on {
                t::TEXT_STRONG
            } else if resp.hovered() {
                t::TEXT
            } else {
                t::TEXT_MUTED
            };
            let p = group.painter();
            if on {
                p.rect_filled(r, CornerRadius::same(t::radius::BUTTON), t::SEG_ON);
            }
            let g = galley(&group, label.as_str(), font::medium(13.0), fg);
            p.galley(pos2(r.left() + 14.0, text_top(r.center().y, &g)), g, fg);
            focus_ring_if(&group, &resp, r, CornerRadius::same(t::radius::BUTTON));
            if resp.clicked() {
                clicked = Some(i);
            }
        }
        clicked
    }
}

/// An id for state kept in egui memory by the screen.
pub fn mem_id(name: &str) -> Id {
    Id::unique(("advanced", name))
}
