//! Segmented groups: the header's mode switcher and Advanced's truss style (`.seg` /
//! `.seg-on` in a `rgba(6,6,6,.88)` well), a compact tool group and `.mini` groups
//! (`.mini-on`, `.mini-off` disabled), plus a lone [`MiniButton`].

use super::icons::Icon;
use super::text::{galley, text_top, wrapped};
use super::{hit, interact};
use eframe::egui::{
    Color32, CornerRadius, Rect, Response, Stroke, StrokeKind, Ui, UiBuilder, accesskit::Role,
    pos2, vec2,
};
use lab_ui::paint::focus_ring_if;
use sp_theme as t;

/// `.mini` item height (a compact segmented look; the Structures mockups use `.seg`).
pub const MINI_H: f32 = 28.0;
/// `.mini` item radius.
pub const MINI_R: u8 = 5;
/// `.mini` well radius.
pub const MINI_WELL_R: u8 = 7;

/// The look of a [`SegGroup`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SegStyle {
    /// `.seg` groups (the header's mode switcher, Advanced's truss style): 3 px padded
    /// `rgba(6,6,6,.88)` well with a `#242424` border (radius 8), 36 px
    /// `.seg` items (`padding: 0 14px`, Inter 500 13, radius 6, 14 px icons).
    Header,
    /// A compact tool group: 2 px padded well, 28 px items, 12 px text.
    Tool,
    /// `.mini` items (28 px, `padding: 0 10px`, Inter 500 12, radius 5) in a 2 px padded
    /// `#060606` well (radius 7). `fill`: items share the width equally (`flex: 1`; long
    /// labels wrap inside the fixed height like the mockup's "R-pentomino"); otherwise they
    /// keep their width and wrap onto more rows.
    Mini {
        /// `flex: 1` items.
        fill: bool,
    },
}

impl SegStyle {
    fn pad(self) -> f32 {
        match self {
            SegStyle::Header => 3.0,
            _ => 2.0,
        }
    }
    fn item_h(self) -> f32 {
        match self {
            SegStyle::Header => t::size::BUTTON,
            _ => MINI_H,
        }
    }
    fn item_pad(self) -> f32 {
        match self {
            SegStyle::Mini { .. } => 10.0,
            _ => 14.0,
        }
    }
    fn font_size(self) -> f32 {
        match self {
            SegStyle::Header => 13.0,
            _ => 12.0,
        }
    }
    fn well_radius(self) -> u8 {
        match self {
            SegStyle::Mini { .. } => MINI_WELL_R,
            _ => t::radius::WELL,
        }
    }
    fn item_radius(self) -> u8 {
        match self {
            SegStyle::Mini { .. } => MINI_R,
            _ => t::radius::BUTTON,
        }
    }
    fn well_fill(self) -> Color32 {
        match self {
            SegStyle::Mini { .. } => t::INSET_BAR,
            _ => t::INSET_BAR,
        }
    }
}

/// One item of a [`SegGroup`].
#[derive(Debug, Clone, Copy)]
pub struct SegItem<'a> {
    label: &'a str,
    icon: Option<Icon>,
    enabled: bool,
    name: Option<&'a str>,
}

impl<'a> SegItem<'a> {
    /// An item labelled `label`.
    pub const fn new(label: &'a str) -> Self {
        Self {
            label,
            icon: None,
            enabled: true,
            name: None,
        }
    }
    /// A 14 px leading icon (mode switcher).
    pub const fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }
    /// Disabled (`.mini-off`, `aria-disabled`): drawn `#5E5E5E`, not clickable.
    pub const fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
    /// Accessible name when it differs from the label.
    pub const fn accessible_name(mut self, name: &'a str) -> Self {
        self.name = Some(name);
        self
    }
}

/// What a [`SegGroup`] did.
#[derive(Debug)]
pub struct SegResponse {
    /// The item clicked this frame (also when it was already selected).
    pub clicked: Option<usize>,
    /// The whole group.
    pub response: Response,
    /// Each item's response.
    pub items: Vec<Response>,
}

/// A segmented group. The group is an AccessKit `Group` (or `Navigation` with
/// [`SegGroup::navigation`]) named `label`; items are toggle `Button`s reporting pressed, or
/// `Link`s reporting selected with [`SegGroup::links`].
///
/// ```ignore
/// let tools = [SegItem::new("Draw"), SegItem::new("Erase"), SegItem::new("Stamp")];
/// if let Some(i) = SegGroup::new("Tool", &tools, Some(cur)).style(SegStyle::Tool).show(ui).clicked {
///     cur = i;
/// }
/// ```
#[must_use = "call .show(ui)"]
pub struct SegGroup<'a> {
    label: &'a str,
    items: &'a [SegItem<'a>],
    selected: Option<usize>,
    style: SegStyle,
    links: bool,
    navigation: bool,
    fill: bool,
    wrap: bool,
}

impl<'a> SegGroup<'a> {
    /// A group named `label` with `items`, `selected` highlighted.
    pub fn new(label: &'a str, items: &'a [SegItem<'a>], selected: Option<usize>) -> Self {
        Self {
            label,
            items,
            selected,
            style: SegStyle::Mini { fill: false },
            links: false,
            navigation: false,
            fill: false,
            wrap: false,
        }
    }

    /// The look (default `.mini`, wrapping).
    pub fn style(mut self, style: SegStyle) -> Self {
        self.style = style;
        self
    }

    /// `.seg` / tool items share the available width equally (`flex: 1; justify-content:
    /// center`, e.g. Materials' Strain axis); the well spans the width. (`.mini` groups use `SegStyle::Mini { fill }`.)
    pub fn fill(mut self) -> Self {
        self.fill = true;
        self
    }

    /// `.seg` / tool items wrap onto more rows when they don't fit the width (`flex-wrap:
    /// wrap`, e.g. Advanced's influence-line picks); the well hugs the rows.
    pub fn wrap(mut self) -> Self {
        self.wrap = true;
        self
    }

    /// Items are links (the mode switcher's `<a class="seg">`).
    pub fn links(mut self) -> Self {
        self.links = true;
        self
    }

    /// The group is a `Navigation` landmark (`<nav aria-label="Mode">`).
    pub fn navigation(mut self) -> Self {
        self.navigation = true;
        self
    }

    fn widths(&self, ui: &Ui) -> Vec<f32> {
        let font = lab_theme::font::medium(self.style.font_size());
        self.items
            .iter()
            .map(|it| {
                let tw = galley(ui, it.label, font.clone(), Color32::WHITE).size().x;
                let iw = it.icon.map_or(0.0, |_| 14.0 + 8.0);
                tw + iw + 2.0 * self.style.item_pad()
            })
            .collect()
    }

    /// The size the group takes when laid out on one row (for header layouts).
    pub fn one_row_size(&self, ui: &Ui) -> eframe::egui::Vec2 {
        let ws = self.widths(ui);
        let pad = self.style.pad();
        let n = ws.len() as f32;
        vec2(
            ws.iter().sum::<f32>() + 2.0 * (n - 1.0).max(0.0) + 2.0 * pad + 2.0,
            self.style.item_h() + 2.0 * pad + 2.0,
        )
    }

    /// Item rects relative to the group's top-left for an available `width`.
    fn layout(&self, ui: &Ui, width: f32) -> (Vec<Rect>, eframe::egui::Vec2) {
        let ws = self.widths(ui);
        let pad = self.style.pad() + 1.0;
        let h = self.style.item_h();
        let gap = 2.0;
        let n = ws.len();
        match self.style {
            SegStyle::Mini { fill: true } => {
                let inner = (width - 2.0 * pad).max(0.0);
                let w = ((inner - gap * (n as f32 - 1.0)) / n as f32).max(0.0);
                let rects = (0..n)
                    .map(|i| Rect::from_min_size(pos2(pad + i as f32 * (w + gap), pad), vec2(w, h)))
                    .collect();
                (rects, vec2(width, h + 2.0 * pad))
            }
            SegStyle::Mini { fill: false } => {
                let inner = (width - 2.0 * pad).max(0.0);
                let mut rects = Vec::with_capacity(n);
                let (mut x, mut y, mut maxx) = (0.0f32, 0.0f32, 0.0f32);
                for w in ws {
                    if x > 0.0 && x + w > inner + 0.01 {
                        x = 0.0;
                        y += h + gap;
                    }
                    rects.push(Rect::from_min_size(pos2(pad + x, pad + y), vec2(w, h)));
                    x += w + gap;
                    maxx = maxx.max(x - gap);
                }
                // `display: flex` (block-level): the well spans the available width
                (rects, vec2(width.max(maxx + 2.0 * pad), y + h + 2.0 * pad))
            }
            SegStyle::Header | SegStyle::Tool if self.fill => {
                let inner = (width - 2.0 * pad).max(0.0);
                let w = ((inner - gap * (n as f32 - 1.0)) / n as f32).max(0.0);
                let rects = (0..n)
                    .map(|i| Rect::from_min_size(pos2(pad + i as f32 * (w + gap), pad), vec2(w, h)))
                    .collect();
                (rects, vec2(width, h + 2.0 * pad))
            }
            SegStyle::Header | SegStyle::Tool if self.wrap => {
                let inner = (width - 2.0 * pad).max(0.0);
                let mut rects = Vec::with_capacity(n);
                let (mut x, mut y, mut maxx) = (0.0f32, 0.0f32, 0.0f32);
                for w in ws {
                    if x > 0.0 && x + w > inner + 0.01 {
                        x = 0.0;
                        y += h + gap;
                    }
                    rects.push(Rect::from_min_size(pos2(pad + x, pad + y), vec2(w, h)));
                    x += w + gap;
                    maxx = maxx.max(x - gap);
                }
                (rects, vec2(maxx + 2.0 * pad, y + h + 2.0 * pad))
            }
            SegStyle::Header | SegStyle::Tool => {
                let mut x = pad;
                let rects = ws
                    .iter()
                    .map(|w| {
                        let r = Rect::from_min_size(pos2(x, pad), vec2(*w, h));
                        x += w + gap;
                        r
                    })
                    .collect();
                (rects, vec2(x - gap + pad, h + 2.0 * pad))
            }
        }
    }

    /// Draws the group at the cursor.
    pub fn show(self, ui: &mut Ui) -> SegResponse {
        let width = ui.available_width();
        let (rel, size) = self.layout(ui, width);
        let origin = ui.cursor().min;
        let well = Rect::from_min_size(origin, size);
        let role = if self.navigation {
            Role::Navigation
        } else {
            Role::Group
        };
        let group = ui.new_child(
            UiBuilder::new()
                .id_salt(("seg", self.label))
                .max_rect(well)
                .accessibility_role(role)
                .accessibility_label(self.label),
        );
        super::set_bounds(&group, well);
        group.painter().rect(
            well,
            CornerRadius::same(self.style.well_radius()),
            self.style.well_fill(),
            Stroke::new(1.0, t::BORDER),
            StrokeKind::Inside,
        );
        let visuals: Vec<Rect> = rel.iter().map(|r| r.translate(origin.to_vec2())).collect();
        let targets = hit::tile(&visuals);
        let font = lab_theme::font::medium(self.style.font_size());
        let mut clicked = None;
        let mut items = Vec::with_capacity(self.items.len());
        for (i, it) in self.items.iter().enumerate() {
            let r = visuals[i];
            let on = self.selected == Some(i);
            let id = group.scope_id().with(i);
            let item_role = if self.links { Role::Link } else { Role::Button };
            let pressed = (!self.links).then_some(on);
            let resp = interact(
                &group,
                id,
                targets[i],
                it.enabled,
                item_role,
                it.name.unwrap_or(it.label),
                pressed,
            );
            if self.links {
                group
                    .ctx()
                    .accesskit_node_builder(resp.id, |b| b.set_selected(on));
            }
            let fg = if !it.enabled {
                t::TEXT_DISABLED
            } else if on {
                t::TEXT_STRONG
            } else if resp.hovered() {
                t::TEXT
            } else {
                t::TEXT_MUTED
            };
            let painter = group.painter();
            if on {
                painter.rect_filled(r, CornerRadius::same(self.style.item_radius()), t::SEG_ON);
            }
            let icon_w = it.icon.map_or(0.0, |_| 14.0 + 8.0);
            let fill_mode = matches!(self.style, SegStyle::Mini { fill: true });
            let g = if fill_mode {
                wrapped(
                    &group,
                    it.label,
                    font.clone(),
                    fg,
                    (r.width() - 2.0 * self.style.item_pad()).max(1.0),
                    None,
                )
            } else {
                galley(&group, it.label, font.clone(), fg)
            };
            let content = icon_w + g.size().x;
            // `flex: 1; justify-content: center` items centre their content
            let mut x = if fill_mode || self.fill {
                r.center().x - content / 2.0
            } else {
                r.left() + self.style.item_pad()
            };
            let cy = r.center().y;
            if let Some(icon) = it.icon {
                icon.paint(
                    painter,
                    Rect::from_center_size(pos2(x + 7.0, cy), vec2(14.0, 14.0)),
                    fg,
                );
                x += 14.0 + 8.0;
            }
            let ty = if g.rows.len() > 1 {
                cy - g.size().y / 2.0
            } else {
                text_top(cy, &g)
            };
            // `text-align: center`: each wrapped row is centred on its own
            if fill_mode && g.rows.len() > 1 {
                paint_centered_rows(painter, &g, r.center().x, ty, fg);
            } else {
                painter.galley(pos2(x, ty), g, fg);
            }
            focus_ring_if(
                &group,
                &resp,
                r,
                CornerRadius::same(self.style.item_radius()),
            );
            if resp.clicked() {
                clicked = Some(i);
            }
            items.push(resp);
        }
        let response = group.response();
        ui.advance_cursor_after_rect(well);
        SegResponse {
            clicked,
            response,
            items,
        }
    }
}

/// Paints each row of a multi-row galley centred on `cx`, starting at `top`.
fn paint_centered_rows(
    painter: &eframe::egui::Painter,
    g: &std::sync::Arc<eframe::egui::Galley>,
    cx: f32,
    top: f32,
    color: Color32,
) {
    let font = g
        .job
        .sections
        .first()
        .map(|s| s.format.font_id.clone())
        .unwrap_or_default();
    for row in &g.rows {
        let text: String = row.row.glyphs.iter().map(|gl| gl.chr).collect();
        let rg = painter.layout_no_wrap(text.trim_end().to_string(), font.clone(), color);
        painter.galley(pos2(cx - rg.size().x / 2.0, top + row.pos.y), rg, color);
    }
}

/// A lone `.mini` button (no well): a compact preset button ("Moore r1", "von
/// Neumann", …), text left-aligned, `.mini-on` when selected; `width` fixes its width
/// (labels then wrap like the mockup's "von Neumann"). Always 28 px tall, as CSS's
/// `height: 28px`: a wrapped label overflows it, centred, instead of growing the button.
#[must_use = "call .show(ui)"]
pub struct MiniButton<'a> {
    label: &'a str,
    on: bool,
    enabled: bool,
    width: Option<f32>,
}

impl<'a> MiniButton<'a> {
    /// A `.mini` labelled `label`.
    pub fn new(label: &'a str, on: bool) -> Self {
        Self {
            label,
            on,
            enabled: true,
            width: None,
        }
    }
    /// Disabled (`.mini-off`).
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
    /// A fixed width.
    pub fn width(mut self, w: f32) -> Self {
        self.width = Some(w);
        self
    }

    /// Draws it.
    pub fn show(self, ui: &mut Ui) -> Response {
        let font = lab_theme::font::medium(12.0);
        let fg = if !self.enabled {
            t::TEXT_DISABLED
        } else if self.on {
            t::TEXT_STRONG
        } else {
            t::TEXT_MUTED
        };
        let natural = galley(ui, self.label, font.clone(), fg).size().x + 20.0;
        let w = self.width.unwrap_or(natural);
        let g = wrapped(ui, self.label, font, fg, (w - 20.0).max(1.0), None);
        let size = vec2(w, MINI_H);
        let (id, rect) = ui.allocate_space(size);
        let resp = interact(
            ui,
            id,
            hit::target(rect),
            self.enabled,
            Role::Button,
            self.label,
            Some(self.on),
        );
        let fg = if self.enabled && !self.on && resp.hovered() {
            t::TEXT
        } else {
            fg
        };
        let painter = ui.painter();
        if self.on {
            painter.rect_filled(rect, CornerRadius::same(MINI_R), t::SEG_ON);
        }
        let ty = if g.rows.len() > 1 {
            rect.center().y - g.size().y / 2.0
        } else {
            text_top(rect.center().y, &g)
        };
        painter.galley_with_override_text_color(pos2(rect.left() + 10.0, ty), g, fg);
        focus_ring_if(ui, &resp, rect, CornerRadius::same(MINI_R));
        resp
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui;

    /// CSS `.mini { height: 28px }`: a label that wraps ("von Neumann" in 84 px) overflows the
    /// button instead of growing it, so a row of presets stays aligned.
    #[test]
    fn mini_buttons_stay_28_px_tall_when_their_label_wraps() {
        let ctx = egui::Context::default();
        sp_theme::install(&ctx);
        let mut heights = Vec::new();
        let mut wraps = false;
        for _ in 0..2 {
            heights.clear();
            let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
                for (label, width) in [("Moore r1", None), ("von Neumann", Some(84.0))] {
                    let r = ui.scope(|ui| {
                        let b = MiniButton::new(label, label == "Moore r1");
                        match width {
                            Some(w) => b.width(w).show(ui),
                            None => b.show(ui),
                        }
                    });
                    heights.push(r.response.rect.height());
                }
                let font = lab_theme::font::medium(12.0);
                wraps = wrapped(ui, "von Neumann", font, t::TEXT, 64.0, None)
                    .rows
                    .len()
                    > 1;
            });
            out.textures_delta.clear();
        }
        assert!(wraps, "the label wraps at 84 px");
        assert_eq!(heights, vec![MINI_H; 2]);
    }
}
