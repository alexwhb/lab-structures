//! The mockup's components, drawn to its CSS (sizes, radii, fonts, hover / pressed states)
//! with colours from [`sp_theme`]. Every interactive widget reports an AccessKit role and name
//! (and its pressed / disabled state), draws the accent focus ring when keyboard-focused, and
//! extends its click target towards 44 × 44 where the surrounding padding allows.
//!
//! | widget | CSS | |
//! |---|---|---|
//! | [`Button`] ([`ghost`], [`prim`]) | `.ghost`, `.prim` (36 px), big (44 px) | actions |
//! | [`SegGroup`] / [`SegItem`] | `.seg` (mode switcher, 36 px), `.mini` groups | choices |
//! | [`Select`] | `<select>` drop-down (36 px) with a menu | |
//! | [`Toggle`] | `.chip` toggle (34 px pill, `.chip-on`, `.chip-dim`, optional dot) | |
//! | [`Checkbox`] | `input[type=checkbox]` row (18 px, accent) | |
//! | [`lbl`] | `.lbl` eyebrow | text |
//! | [`Card`] | `.panel` / `.well` frames | frames |
//! | [`StatTile`] | stat tiles (label + mono value) | readouts |
//! | [`Callout`] | "What's happening" with the pulsing dot | |
//! | [`Chip`] | the floating measurement `.chip` pinned to a stage | |
//! | [`CodeBlock`] | mono block | |
//! | [`badge()`] | `.badge` with a provenance `.dot` | |
//! | [`Slider`] | `input[type=range]` in the accent, with its label / value row | |
//! | [`Plot`] | sparkline / plot (line, fill, guides, playhead) | |
//! | [`Icon`] | every mockup icon as vector paths | icons |
//! | [`Toasts`] | transient confirmations | |
//! | [`header`] | the header (NAV-1; no world tab row in v2) | chrome |
//! | [`tensile`] | the tensile rig and stress–strain plot painters (v1 Materials; the builder's sample drawer), generic over the scene they draw | |
//! | [`world`] | band bars, comparison rows, readout tiles, goal rows, … (v1 Tower / Ground; reused by the builder's fold-ins) | |
//!
//! [`gallery`] draws all of them (the widget snapshot test renders it). Screen agents may add
//! widget files here for widgets only their screen needs (one `pub mod` line each).

pub mod badge;
pub mod buttons;
pub mod cards;
pub mod flow;
pub mod gallery;
pub mod header;
pub mod icons;
pub mod menu;
pub mod plot;
pub mod seg;
pub mod select;
pub mod slider;
pub mod tensile;
pub mod text;
pub mod toast;
pub mod world;

pub use badge::{BadgeLook, badge, badge_with};
pub use buttons::{Button, ButtonKind, Checkbox, Toggle, ghost, prim};
pub use cards::{Callout, Card, Chip, CodeBlock, StatTile};
pub use icons::Icon;
pub use plot::Plot;
pub use seg::{MiniButton, SegGroup, SegItem, SegResponse, SegStyle};
pub use select::{Select, SelectResponse};
pub use slider::Slider;
pub use text::{lbl, lbl_colored};
pub use toast::{Toast, Toasts};

use eframe::egui::{Id, Rect, Response, Sense, Ui, vec2};

/// Hit areas: controls drawn smaller than 44 px keep their drawn size and grow
/// their click target into the surrounding padding.
pub mod hit {
    use super::*;

    /// The smallest target (WCAG 2.5.5).
    pub const MIN: f32 = sp_theme::size::MIN_TARGET;

    /// `visual` grown symmetrically until both sides reach 44 px (never shrunk).
    pub fn target(visual: Rect) -> Rect {
        let dx = ((MIN - visual.width()) / 2.0).max(0.0);
        let dy = ((MIN - visual.height()) / 2.0).max(0.0);
        visual.expand2(vec2(dx, dy))
    }

    /// Targets for a group of controls drawn close together (a row of bits, a 5 × 5 grid, a
    /// segmented well): each grows towards 44 px, but never past the middle of the gap to a
    /// neighbour, so targets tile the group without overlapping and a click always lands on
    /// the control under (or nearest to) the pointer.
    pub fn tile(visuals: &[Rect]) -> Vec<Rect> {
        let mut out: Vec<Rect> = visuals.iter().map(|v| target(*v)).collect();
        for i in 0..visuals.len() {
            for j in 0..visuals.len() {
                if i == j || !out[i].intersects(out[j]) {
                    continue;
                }
                let (a, b) = (visuals[i], visuals[j]);
                // separated horizontally?
                if a.right() <= b.left() && overlap(a.y_range(), b.y_range()) {
                    let mid = (a.right() + b.left()) / 2.0;
                    out[i].max.x = out[i].max.x.min(mid);
                } else if b.right() <= a.left() && overlap(a.y_range(), b.y_range()) {
                    let mid = (b.right() + a.left()) / 2.0;
                    out[i].min.x = out[i].min.x.max(mid);
                } else if a.bottom() <= b.top() {
                    let mid = (a.bottom() + b.top()) / 2.0;
                    out[i].max.y = out[i].max.y.min(mid);
                } else if b.bottom() <= a.top() {
                    let mid = (b.bottom() + a.top()) / 2.0;
                    out[i].min.y = out[i].min.y.max(mid);
                }
            }
        }
        out
    }

    fn overlap(a: eframe::egui::Rangef, b: eframe::egui::Rangef) -> bool {
        a.min < b.max && b.min < a.max
    }
}

/// Gives a container node (a `Ui` built with an accessibility role: groups, regions, the tab
/// list) its bounds; egui only sets bounds on widgets, and assistive tech (and tests) need
/// them on landmarks too.
pub fn set_bounds(ui: &Ui, rect: Rect) {
    let r = eframe::egui::accesskit::Rect {
        x0: rect.min.x as f64,
        y0: rect.min.y as f64,
        x1: rect.max.x as f64,
        y1: rect.max.y as f64,
    };
    ui.ctx()
        .accesskit_node_builder(ui.unique_id(), |b| b.set_bounds(r));
}

/// Scrolls a control that just got keyboard focus (Tab / Shift-Tab) into view in its nearest
/// scroller (egui doesn't): an aside at 1440, the page when columns stack (NAV-5).
pub(crate) fn reveal_on_focus(resp: &Response) {
    if resp.gained_focus() {
        resp.scroll_to_me(None);
    }
}

fn space_key() -> Id {
    Id::unique("sp-space-passes-through")
}

/// Call while a widget that has no use for Space (a slider) has keyboard focus: Space then
/// still presses the header's drive button (NAV-5: only a text field keeps Space). Buttons,
/// tabs and checkboxes don't call it — Space activates them.
pub fn space_passes_through(ui: &Ui, id: Id) {
    ui.data_mut(|d| d.insert_temp(space_key(), id));
}

/// Whether the focused widget `focused` lets Space through (see [`space_passes_through`]).
pub fn space_passes(ctx: &eframe::egui::Context, focused: Id) -> bool {
    ctx.data(|d| d.get_temp::<Id>(space_key())) == Some(focused)
}

fn arrows_key() -> Id {
    Id::unique("sp-arrows-pass-through")
}

/// Call every frame while widget `id` has keyboard focus and the arrow keys should reach the
/// screen as [`crate::Command::Pan`] / [`crate::Command::Nudge`] (the builder's stage, a joint
/// button: NAV-5 "arrow keys pan the view, ⌥ nudges a selected joint") instead of moving focus
/// to the neighbouring widget. Locks egui's arrow-key focus moves for `id`; Tab still leaves.
/// With nothing focused the arrows reach the Playground anyway.
pub fn arrows_pass_through(ui: &Ui, id: Id) {
    ui.data_mut(|d| d.insert_temp(arrows_key(), id));
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
}

/// Whether the focused widget `focused` lets the arrow keys through (see
/// [`arrows_pass_through`]).
pub fn arrows_pass(ctx: &eframe::egui::Context, focused: Id) -> bool {
    ctx.data(|d| d.get_temp::<Id>(arrows_key())) == Some(focused)
}

/// Click sense when enabled, hover only when disabled.
pub(crate) fn sense(enabled: bool) -> Sense {
    if enabled {
        Sense::click()
    } else {
        Sense::hover()
    }
}

/// Interacts with `target` (the hit area) for a control drawn at `visual`, registering the
/// AccessKit node with `role`, `name`, enabled state and optional pressed state.
pub(crate) fn interact(
    ui: &Ui,
    id: Id,
    target: Rect,
    enabled: bool,
    role: eframe::egui::accesskit::Role,
    name: &str,
    pressed: Option<bool>,
) -> Response {
    let resp = ui.interact(target, id, sense(enabled));
    reveal_on_focus(&resp);
    let name = name.to_string();
    resp.widget_info(|| {
        let mut info = eframe::egui::WidgetInfo::labeled(role, enabled, &name);
        info.selected = pressed;
        info
    });
    if enabled {
        resp.on_hover_cursor(eframe::egui::CursorIcon::PointingHand)
    } else {
        resp
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::pos2;

    #[test]
    fn targets_grow_to_44() {
        let r = Rect::from_min_size(pos2(10.0, 10.0), vec2(28.0, 30.0));
        let t = hit::target(r);
        assert_eq!(t.size(), vec2(44.0, 44.0));
        assert_eq!(t.center(), r.center());
        let big = Rect::from_min_size(pos2(0.0, 0.0), vec2(80.0, 50.0));
        assert_eq!(hit::target(big), big);
    }

    #[test]
    fn tiled_targets_never_overlap() {
        // a row of nine 28 × 30 controls, 4 px apart
        let bits: Vec<Rect> = (0..9)
            .map(|i| Rect::from_min_size(pos2(i as f32 * 32.0, 0.0), vec2(28.0, 30.0)))
            .collect();
        let t = hit::tile(&bits);
        for i in 0..9 {
            assert!(t[i].contains_rect(bits[i]));
            assert_eq!(t[i].height(), 44.0);
            for j in 0..9 {
                if i != j {
                    let x = t[i].intersect(t[j]);
                    assert!(!x.is_positive(), "{i} and {j} overlap: {x:?}");
                }
            }
        }
        // inner bits span the full pitch, the ends reach 44 outwards
        assert_eq!(t[4].width(), 32.0);
        assert_eq!(t[0].left(), -8.0);
        // a 5 × 5 grid of 30 px cells, 3 px apart
        let cells: Vec<Rect> = (0..25)
            .map(|i| {
                Rect::from_min_size(
                    pos2((i % 5) as f32 * 33.0, (i / 5) as f32 * 33.0),
                    vec2(30.0, 30.0),
                )
            })
            .collect();
        let t = hit::tile(&cells);
        for i in 0..25 {
            for j in 0..25 {
                if i != j {
                    assert!(!t[i].intersect(t[j]).is_positive(), "{i} {j}");
                }
            }
            assert!(t[i].contains(cells[i].center()));
        }
    }
}
