//! A small popup menu under a button (e.g. an Export: Copy netlist, Save .str…,
//! Save .rle…, Save .rule…). The mockup leaves Export as a bare button (SPEC DONE-3), so the
//! look follows its other surfaces: a `#111111` card with a `#2E2E2E` border (radius 8,
//! padding 4) and a soft shadow, 36 px items in Inter 500 13 px `#D9D9D9` that light up
//! `#2A2A2A` on hover or focus, a hairline between groups.
//!
//! AccessKit: a `Menu` named after the button, with `MenuItem`s; the button reports
//! `has_popup = menu` and `expanded` ([`menu_button_state`]). Keyboard: opening it with
//! Enter / Space focuses the first item, ↑ / ↓ move, Enter activates, Escape closes; a click
//! outside closes it.

use super::text::{galley, text_top};
use super::{hit, interact};
use eframe::egui::{
    self, Area, CornerRadius, Id, Key, Order, Rect, Response, Sense, Stroke, StrokeKind, UiBuilder,
    accesskit, accesskit::Role, epaint::Shadow, pos2, vec2,
};
use lab_ui::paint::focus_ring_if;
use sp_theme as t;

/// Item height.
pub const ITEM_H: f32 = 36.0;
/// Inner padding of the menu card.
const PAD: f32 = 4.0;

/// One entry of a [`Menu`].
#[derive(Debug, Clone, Copy)]
pub struct MenuItem<'a> {
    /// The label (also the accessible name).
    pub label: &'a str,
    /// Draw a hairline above this item (starts a new group).
    pub separator: bool,
}

impl<'a> MenuItem<'a> {
    /// An item.
    pub const fn new(label: &'a str) -> Self {
        Self {
            label,
            separator: false,
        }
    }

    /// Starts a new group (a hairline above it).
    pub const fn group(mut self) -> Self {
        self.separator = true;
        self
    }
}

/// What a [`Menu`] did this frame.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MenuResponse {
    /// The item activated.
    pub clicked: Option<usize>,
    /// The menu wants to close (an item was activated, Escape, or a click outside).
    pub close: bool,
}

/// A popup menu anchored under `anchor` (right-aligned to it), shown while the caller keeps
/// it open.
#[must_use = "call .show(ctx)"]
pub struct Menu<'a> {
    id: Id,
    label: &'a str,
    anchor: Rect,
    items: &'a [MenuItem<'a>],
    focus_first: bool,
}

impl<'a> Menu<'a> {
    /// A menu named `label` (the button's label) under `anchor` (the button's rect).
    pub fn new(id: Id, label: &'a str, anchor: Rect, items: &'a [MenuItem<'a>]) -> Self {
        Self {
            id,
            label,
            anchor,
            items,
            focus_first: false,
        }
    }

    /// Move keyboard focus to the first item (the menu was opened from the keyboard).
    pub fn focus_first(mut self, on: bool) -> Self {
        self.focus_first = on;
        self
    }

    /// The menu's size.
    fn size(&self, ctx: &egui::Context) -> egui::Vec2 {
        let font = lab_theme::font::medium(13.0);
        let widest = self
            .items
            .iter()
            .map(|it| {
                ctx.fonts_mut(|f| {
                    f.layout_no_wrap(it.label.to_string(), font.clone(), t::TEXT)
                        .size()
                        .x
                })
            })
            .fold(0.0, f32::max);
        let seps = self.items.iter().filter(|i| i.separator).count() as f32;
        vec2(
            (widest + 2.0 * 12.0 + 2.0 * PAD + 2.0).max(180.0),
            self.items.len() as f32 * ITEM_H + seps * 9.0 + 2.0 * PAD + 2.0,
        )
    }

    /// Draws the menu on top of everything.
    pub fn show(self, ctx: &egui::Context) -> MenuResponse {
        let size = self.size(ctx);
        let screen = ctx.content_rect();
        let x = (self.anchor.right() - size.x)
            .min(screen.right() - size.x - 8.0)
            .max(screen.left() + 8.0);
        let rect = Rect::from_min_size(pos2(x, self.anchor.bottom() + 6.0), size);
        let mut out = MenuResponse::default();
        Area::new(self.id)
            .order(Order::Foreground)
            .fixed_pos(rect.min)
            .constrain(false)
            .show(ctx, |ui| {
                let (frame, _) = ui.allocate_exact_size(size, Sense::hover());
                let menu = ui.new_child(
                    UiBuilder::new()
                        .id_salt("menu")
                        .max_rect(frame)
                        .accessibility_role(Role::Menu)
                        .accessibility_label(self.label),
                );
                super::set_bounds(&menu, frame);
                let painter = menu.painter();
                let shadow = Shadow {
                    offset: [0, 8],
                    blur: 24,
                    spread: 0,
                    color: t::rgba(0, 0, 0, 0.5),
                };
                painter.add(shadow.as_shape(frame, CornerRadius::same(8)));
                painter.rect(
                    frame,
                    CornerRadius::same(8),
                    t::PANEL,
                    Stroke::new(1.0, t::BORDER_STRONG),
                    StrokeKind::Inside,
                );
                let font = lab_theme::font::medium(13.0);
                // the items' rects (and the separators above them), then targets that tile
                // the column: each grows towards 44 px but never into its neighbour's (a click
                // at the bottom of one item never picks the next)
                let mut y = frame.top() + 1.0 + PAD;
                let mut rects = Vec::with_capacity(self.items.len());
                let mut separators = Vec::new();
                for it in self.items {
                    if it.separator {
                        separators.push((y + 4.0).round() + 0.5);
                        y += 9.0;
                    }
                    rects.push(Rect::from_min_size(
                        pos2(frame.left() + 1.0 + PAD, y),
                        vec2(frame.width() - 2.0 - 2.0 * PAD, ITEM_H),
                    ));
                    y += ITEM_H;
                }
                for sy in separators {
                    painter.hline(
                        (frame.left() + 1.0 + PAD)..=(frame.right() - 1.0 - PAD),
                        sy,
                        Stroke::new(1.0, t::BORDER),
                    );
                }
                let targets = hit::tile(&rects);
                let mut responses: Vec<Response> = Vec::with_capacity(self.items.len());
                for (i, it) in self.items.iter().enumerate() {
                    let r = rects[i];
                    let resp = interact(
                        &menu,
                        menu.scope_id().with(i),
                        targets[i],
                        true,
                        Role::MenuItem,
                        it.label,
                        None,
                    );
                    let lit = resp.hovered() || resp.has_focus();
                    if lit {
                        painter.rect_filled(r, CornerRadius::same(super::seg::MINI_R), t::SEG_ON);
                    }
                    let fg = if lit { t::TEXT_STRONG } else { t::TEXT_DIM };
                    let g = galley(&menu, it.label, font.clone(), fg);
                    painter.galley(pos2(r.left() + 12.0, text_top(r.center().y, &g)), g, fg);
                    focus_ring_if(&menu, &resp, r, CornerRadius::same(super::seg::MINI_R));
                    if resp.clicked() {
                        out.clicked = Some(i);
                    }
                    responses.push(resp);
                }
                if self.focus_first
                    && let Some(first) = responses.first()
                {
                    first.request_focus();
                }
                // ↑ / ↓ move between items while one has focus
                if let Some(cur) = responses.iter().position(Response::has_focus) {
                    // keep egui's own arrow-key focus moves out of the way
                    let id = responses[cur].id;
                    ui.memory_mut(|m| {
                        m.set_focus_lock_filter(
                            id,
                            egui::EventFilter {
                                vertical_arrows: true,
                                horizontal_arrows: true,
                                ..Default::default()
                            },
                        )
                    });
                    let (up, down) = ui.input_mut(|i| {
                        (
                            i.consume_key(egui::Modifiers::NONE, Key::ArrowUp),
                            i.consume_key(egui::Modifiers::NONE, Key::ArrowDown),
                        )
                    });
                    let n = responses.len();
                    if down {
                        responses[(cur + 1) % n].request_focus();
                    } else if up {
                        responses[(cur + n - 1) % n].request_focus();
                    }
                }
            });
        let (escape, outside) = ctx.input(|i| {
            let outside = i.pointer.any_pressed()
                && i.pointer
                    .interact_pos()
                    .is_some_and(|p| !rect.contains(p) && !self.anchor.contains(p));
            (i.key_pressed(Key::Escape), outside)
        });
        out.close = out.clicked.is_some() || escape || outside;
        out
    }
}

/// Marks a menu button's AccessKit node: `has_popup = menu`, `expanded = open`.
pub fn menu_button_state(ui: &egui::Ui, button: &Response, open: bool) {
    ui.ctx().accesskit_node_builder(button.id, |b| {
        b.set_has_popup(accesskit::HasPopup::Menu);
        b.set_expanded(open);
    });
}
