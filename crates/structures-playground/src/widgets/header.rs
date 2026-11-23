//! The header (NAV-1), drawn to the mockup's `<header>`: `min-height: 56px; display: flex;
//! align-items: center; gap: 12px 16px; padding: 8px 16px; border-bottom: 1px solid #242424;
//! background: rgba(12,12,12,.85); flex-wrap: wrap; box-sizing: border-box` holding
//!
//! 1. the brand (`gap: 10px`) — the 26 px logo mark, "Structures Playground" (Space Grotesk 600
//!    16 px, `letter-spacing: -.01em`), a `#4A4A4A` "/", the document name (13 px `#D9D9D9`);
//!    an AccessKit `Link` to the Series screen (NAV-3);
//! 2. the mode switcher: Playground (truss) · Advanced (chart) links in a `rgba(6,6,6,.88)`
//!    well, a `Navigation` named "Mode";
//! 3. the screen's actions (`margin-left: auto; flex-wrap: wrap; gap: 8px`): the `.prim` drive
//!    button (play icon + label, or pause icon + "Pause"), the `.ghost` Reset (restart icon),
//!    the mono clock (12 px `#A3A3A3`, `padding: 0 8px`) and the `.ghost` Share.
//!
//! The header has no app switcher (PROGRESS decisions). At 390 px each part wraps onto its own
//! row like the browser (see `flow`).

use super::buttons::{ghost, prim};
use super::flow::{FlowItem, flow};
use super::icons::Icon;
use super::seg::{SegGroup, SegItem, SegStyle};
use super::text::{ellipsized, galley, galley_spaced, text_top};
use super::{hit, interact};
use crate::router::{Mode, Route};
use crate::screens::Drive;
use eframe::egui::{
    CornerRadius, Rect, Sense, Stroke, Ui, UiBuilder, Vec2, WidgetInfo, accesskit::Role, pos2, vec2,
};
use lab_ui::paint::{advance_block, focus_ring_if};
use sp_theme as t;

/// Header padding (left / right).
pub const PAD_X: f32 = 16.0;
/// Header padding (top / bottom).
pub const PAD_Y: f32 = 8.0;
/// Gap between the header's parts: 16 px across, 12 px between wrapped rows.
pub const GAP: Vec2 = vec2(16.0, 12.0);
/// Gap between the actions.
pub const SLOT_GAP: f32 = 8.0;
/// The brand's internal gap.
const BRAND_GAP: f32 = 10.0;
/// The logo mark's size.
const LOGO: f32 = 26.0;

const MODES: [SegItem<'static>; 2] = [
    SegItem::new("Playground").icon(Icon::Truss),
    SegItem::new("Advanced").icon(Icon::Chart),
];

/// What the header shows (gathered from the route and its screen).
#[derive(Debug, Clone)]
pub struct HeaderModel<'a> {
    /// The current route (highlights its mode).
    pub route: Route,
    /// The document name after the brand.
    pub document: &'a str,
    /// The drive button, if any.
    pub drive: Option<Drive>,
    /// Show Reset.
    pub reset: bool,
    /// The mono clock (empty: none).
    pub clock: String,
}

/// What was clicked in the header this frame.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HeaderClicks {
    /// The brand (→ Series, NAV-3).
    pub brand: bool,
    /// A mode in the switcher.
    pub mode: Option<Mode>,
    /// The drive button.
    pub drive: bool,
    /// Reset.
    pub reset: bool,
    /// Share.
    pub share: bool,
}

/// The mode switcher's group for `route`.
fn mode_group(route: Route) -> SegGroup<'static> {
    let current = route
        .mode()
        .and_then(|m| Mode::ALL.iter().position(|x| *x == m));
    SegGroup::new("Mode", &MODES, current)
        .style(SegStyle::Header)
        .links()
        .navigation()
}

/// One action of the right-hand group.
enum Action {
    Drive(Drive),
    Reset,
    Clock(String),
    Share,
}

const CLOCK_PAD: f32 = 8.0;

fn clock_font() -> eframe::egui::FontId {
    lab_theme::font::mono(12.0)
}

impl Action {
    fn size(&self, ui: &Ui) -> Vec2 {
        match self {
            Action::Drive(d) => prim(d.text()).icon(drive_icon(d)).desired_size(ui),
            Action::Reset => ghost("Reset").icon(Icon::Restart).desired_size(ui),
            Action::Clock(s) => {
                let g = galley(ui, s, clock_font(), t::TEXT_MUTED);
                vec2(g.size().x + 2.0 * CLOCK_PAD, t::size::BUTTON)
            }
            Action::Share => ghost("Share").desired_size(ui),
        }
    }
}

fn drive_icon(d: &Drive) -> Icon {
    if d.running { Icon::Pause } else { Icon::Play }
}

/// Draws the header across the full width at the cursor and advances past it. Returns its
/// rect (border included) and what was clicked.
pub fn header(ui: &mut Ui, model: &HeaderModel) -> (Rect, HeaderClicks) {
    let mut clicks = HeaderClicks::default();
    let route = model.route;
    let outer = ui.available_rect_before_wrap();
    let (left, top, width) = (outer.left(), outer.top(), outer.width());
    let inner_w = (width - 2.0 * PAD_X).max(0.0);
    // `box-sizing: border-box; min-height: 56px` includes the padding and the border
    let min_content = t::size::HEADER - 2.0 * PAD_Y - 1.0;

    // ---- measure
    let word = galley_spaced(
        ui,
        crate::APP_NAME,
        lab_theme::font::heading(16.0),
        t::TEXT,
        -0.16,
    );
    let slash = galley(ui, "/", lab_theme::font::body(16.0), t::SLASH);
    // a long document name is cut with an ellipsis so the brand fits its line (the full name
    // stays the accessible label)
    let before_doc = LOGO + BRAND_GAP + word.size().x + BRAND_GAP + slash.size().x + BRAND_GAP;
    let doc_room = (inner_w - before_doc).max(40.0);
    let doc = ellipsized(
        ui,
        model.document,
        lab_theme::font::body(13.0),
        t::TEXT_DIM,
        doc_room,
    );
    let brand_w = before_doc + doc.size().x;
    let modes = mode_group(route);
    let modes_size = modes.one_row_size(ui);
    let mut actions = Vec::new();
    if let Some(d) = model.drive {
        actions.push(Action::Drive(d));
    }
    if model.reset {
        actions.push(Action::Reset);
    }
    if !model.clock.is_empty() {
        actions.push(Action::Clock(model.clock.clone()));
    }
    actions.push(Action::Share);
    let action_items: Vec<FlowItem> = actions
        .iter()
        .map(|a| FlowItem::fixed(a.size(ui)))
        .collect();
    let natural = flow(&action_items, f32::INFINITY, vec2(SLOT_GAP, SLOT_GAP), 0.0);
    let natural_w = natural.rects.last().map_or(0.0, |r| r.right().max(0.0));
    // the group shrinks to its line and wraps inside: lay the header out, then the group in
    // the width it got, then the header again with the group's wrapped height
    let mut slot_size = vec2(natural_w, t::size::BUTTON);
    let mut lay;
    let mut inner;
    let mut passes = 0;
    loop {
        passes += 1;
        let items = [
            FlowItem::fixed(vec2(brand_w, LOGO)),
            FlowItem::fixed(modes_size),
            FlowItem::fixed(slot_size).auto().shrink(),
        ];
        lay = flow(&items, inner_w, GAP, min_content);
        let w = lay.rects[2].width();
        inner = flow(&action_items, w, vec2(SLOT_GAP, SLOT_GAP), 0.0);
        if (inner.height - slot_size.y).abs() < 0.5 || passes >= 3 {
            break;
        }
        slot_size.y = inner.height;
    }
    let origin = vec2(left + PAD_X, top + PAD_Y);
    let at = |i: usize| lay.rects[i].translate(origin);
    let bar = Rect::from_min_size(pos2(left, top), vec2(width, lay.height + 2.0 * PAD_Y + 1.0));

    // ---- paint the bar
    let painter = ui.painter();
    painter.rect_filled(bar, CornerRadius::ZERO, t::BAR);
    painter.hline(
        bar.x_range(),
        bar.bottom() - 0.5,
        Stroke::new(1.0, t::BORDER),
    );
    let mut bar_ui = ui.new_child(
        UiBuilder::new()
            .id_salt("sp-header")
            .max_rect(bar)
            .accessibility_role(Role::Banner),
    );
    super::set_bounds(&bar_ui, bar);

    // ---- brand (a link to the Series screen, NAV-3)
    let brand = at(0);
    let resp = interact(
        &bar_ui,
        bar_ui.scope_id().with("brand"),
        hit::target(brand),
        true,
        Role::Link,
        crate::APP_NAME,
        None,
    );
    let cy = brand.center().y;
    let p = bar_ui.painter();
    let logo = Rect::from_min_size(pos2(brand.left(), cy - LOGO / 2.0), vec2(LOGO, LOGO));
    Icon::Logo.paint(p, logo, t::ACCENT);
    let mut x = logo.right() + BRAND_GAP;
    p.galley(pos2(x, text_top(cy, &word)), word.clone(), t::TEXT);
    x += word.size().x + BRAND_GAP;
    p.galley(pos2(x, text_top(cy, &slash)), slash.clone(), t::SLASH);
    x += slash.size().x + BRAND_GAP;
    let doc_rect = Rect::from_min_size(pos2(x, cy - doc.size().y / 2.0), doc.size());
    p.galley(pos2(x, text_top(cy, &doc)), doc, t::TEXT_DIM);
    focus_ring_if(&bar_ui, &resp, brand, CornerRadius::same(6));
    clicks.brand = resp.clicked();
    // the document name, as text for assistive tech (NAV-1)
    let document = model.document.to_string();
    bar_ui
        .interact(doc_rect, bar_ui.scope_id().with("document"), Sense::hover())
        .widget_info(|| WidgetInfo::labeled(Role::Label, true, &document));

    // ---- mode switcher
    let mut mode_ui = bar_ui.new_child(UiBuilder::new().id_salt("modes").max_rect(at(1)));
    if let Some(i) = modes.show(&mut mode_ui).clicked {
        clicks.mode = Some(Mode::ALL[i]);
    }

    // ---- the screen's actions
    let slot = at(2);
    let mut slot_ui = bar_ui.new_child(UiBuilder::new().id_salt("actions").max_rect(slot));
    for (a, r) in actions.iter().zip(&inner.rects) {
        let r = r.translate(slot.min.to_vec2());
        match a {
            Action::Drive(d) => {
                clicks.drive = prim(d.text())
                    .icon(drive_icon(d))
                    .enabled(d.enabled)
                    .show_at(&mut slot_ui, r)
                    .clicked();
            }
            Action::Reset => {
                clicks.reset = ghost("Reset")
                    .icon(Icon::Restart)
                    .show_at(&mut slot_ui, r)
                    .clicked();
            }
            Action::Clock(s) => {
                let g = galley(&slot_ui, s, clock_font(), t::TEXT_MUTED);
                let cy = r.center().y;
                slot_ui.painter().galley(
                    pos2(r.left() + CLOCK_PAD, text_top(cy, &g)),
                    g,
                    t::TEXT_MUTED,
                );
                let s = s.clone();
                slot_ui
                    .interact(r, slot_ui.scope_id().with("clock"), Sense::hover())
                    .widget_info(|| WidgetInfo::labeled(Role::Label, true, &s));
            }
            Action::Share => {
                clicks.share = ghost("Share").show_at(&mut slot_ui, r).clicked();
            }
        }
    }

    advance_block(ui, bar);
    (bar, clicks)
}
